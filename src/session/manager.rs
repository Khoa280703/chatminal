use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::thread;

use indexmap::IndexMap;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tokio::sync::mpsc;

use crate::session::pty_worker::{SessionEvent, pty_reader_thread, pty_writer_thread};
use crate::session::{Session, SessionId};

const MAX_INPUT_BYTES: usize = 65_536;

#[derive(Debug)]
pub enum SessionError {
    SessionNotFound,
    InputTooLarge,
    ChannelFull,
    ChannelClosed,
    Pty(String),
    Shell(String),
}

pub struct SessionManager {
    sessions: IndexMap<SessionId, Session>,
    event_tx: mpsc::Sender<SessionEvent>,
    configured_shell: Option<String>,
    scrollback_max_lines: usize,
}

impl SessionManager {
    pub fn new(
        event_tx: mpsc::Sender<SessionEvent>,
        configured_shell: Option<String>,
        scrollback_max_lines: usize,
    ) -> Self {
        Self {
            sessions: IndexMap::new(),
            event_tx,
            configured_shell,
            scrollback_max_lines,
        }
    }

    pub fn contains(&self, id: SessionId) -> bool {
        self.sessions.contains_key(&id)
    }

    pub fn list_sessions(&self) -> Vec<(SessionId, String)> {
        self.sessions
            .iter()
            .map(|(id, session)| (*id, session.name.clone()))
            .collect()
    }

    pub fn session_ids(&self) -> Vec<SessionId> {
        self.sessions.keys().copied().collect()
    }

    pub fn create_session(
        &mut self,
        name: String,
        cols: usize,
        rows: usize,
    ) -> Result<SessionId, SessionError> {
        let shell = self.resolve_shell_path(self.configured_shell.as_deref())?;

        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(Self::pty_size(cols, rows))
            .map_err(|err| SessionError::Pty(err.to_string()))?;

        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");

        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| SessionError::Pty(err.to_string()))?;

        let reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|err| SessionError::Pty(err.to_string()))?;
        let writer = pty_pair
            .master
            .take_writer()
            .map_err(|err| SessionError::Pty(err.to_string()))?;

        let id = SessionId::new_v4();
        let (input_tx, input_rx) = mpsc::channel::<Vec<u8>>(16);

        let reader_tx = self.event_tx.clone();
        let scrollback_max_lines = self.scrollback_max_lines;
        let reader_handle = thread::spawn(move || {
            pty_reader_thread(reader, reader_tx, id, cols, rows, scrollback_max_lines)
        });

        let writer_handle = thread::spawn(move || pty_writer_thread(writer, input_rx));

        self.sessions.insert(
            id,
            Session {
                id,
                name,
                child,
                master: pty_pair.master,
                input_tx,
                reader_handle: Some(reader_handle),
                writer_handle: Some(writer_handle),
            },
        );

        Ok(id)
    }

    pub fn close_session(&mut self, id: SessionId) {
        let Some(mut session) = self.sessions.shift_remove(&id) else {
            return;
        };

        let _ = session.child.kill();
        drop(session.input_tx);
        drop(session.master);

        if let Some(handle) = session.reader_handle.take() {
            let _ = handle.join();
        }

        let _ = session.child.wait();

        if let Some(handle) = session.writer_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn send_input(&self, id: SessionId, bytes: Vec<u8>) -> Result<(), SessionError> {
        if bytes.len() > MAX_INPUT_BYTES {
            log::warn!("Rejected PTY input larger than {} bytes", MAX_INPUT_BYTES);
            return Err(SessionError::InputTooLarge);
        }

        let session = self
            .sessions
            .get(&id)
            .ok_or(SessionError::SessionNotFound)?;

        session.input_tx.try_send(bytes).map_err(|err| match err {
            mpsc::error::TrySendError::Full(_) => SessionError::ChannelFull,
            mpsc::error::TrySendError::Closed(_) => SessionError::ChannelClosed,
        })
    }

    pub fn resize_all_sessions(&mut self, cols: usize, rows: usize) {
        for session in self.sessions.values_mut() {
            if let Err(err) = session.master.resize(Self::pty_size(cols, rows)) {
                log::warn!("Failed to resize session {}: {err}", session.id);
            }
        }
    }

    fn pty_size(cols: usize, rows: usize) -> PtySize {
        PtySize {
            rows: rows.max(1).min(u16::MAX as usize) as u16,
            cols: cols.max(1).min(u16::MAX as usize) as u16,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    fn resolve_shell_path(&self, config_shell: Option<&str>) -> Result<String, SessionError> {
        let mut candidates = Vec::new();

        if let Some(shell) = config_shell {
            candidates.push(shell.to_string());
        }

        if let Ok(shell) = std::env::var("SHELL") {
            candidates.push(shell);
        }

        candidates.push("/bin/bash".to_string());
        candidates.push("/bin/sh".to_string());

        for candidate in candidates {
            if self.validate_shell_path(&candidate).is_ok() {
                return Ok(candidate);
            }
        }

        Err(SessionError::Shell(
            "No valid shell found in config, env, or defaults".to_string(),
        ))
    }

    fn validate_shell_path(&self, raw_path: &str) -> Result<(), SessionError> {
        let raw = PathBuf::from(raw_path);

        fs::symlink_metadata(&raw)
            .map_err(|err| SessionError::Shell(format!("Invalid shell path: {err}")))?;

        let canonical = fs::canonicalize(&raw)
            .map_err(|err| SessionError::Shell(format!("Cannot canonicalize shell: {err}")))?;

        if !self.is_allowed_shell(&raw, &canonical)? {
            return Err(SessionError::Shell(format!(
                "Shell is not in /etc/shells: {}",
                raw.display()
            )));
        }

        let meta = fs::metadata(&canonical)
            .map_err(|err| SessionError::Shell(format!("Cannot stat shell: {err}")))?;

        if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
            return Err(SessionError::Shell("Shell is not executable".to_string()));
        }

        Ok(())
    }

    fn is_allowed_shell(&self, raw: &Path, canonical: &Path) -> Result<bool, SessionError> {
        let shells = fs::read_to_string("/etc/shells")
            .map_err(|err| SessionError::Shell(format!("Cannot read /etc/shells: {err}")))?;

        for line in shells.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let candidate = PathBuf::from(line);
            if candidate == raw {
                return Ok(true);
            }

            if let Ok(candidate_canonical) = fs::canonicalize(&candidate)
                && candidate_canonical == canonical
            {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::SessionNotFound => write!(f, "session not found"),
            SessionError::InputTooLarge => write!(f, "input exceeds maximum size"),
            SessionError::ChannelFull => write!(f, "session input queue is full"),
            SessionError::ChannelClosed => write!(f, "session input channel closed"),
            SessionError::Pty(err) => write!(f, "pty error: {err}"),
            SessionError::Shell(err) => write!(f, "shell resolution error: {err}"),
        }
    }
}

impl std::error::Error for SessionError {}
