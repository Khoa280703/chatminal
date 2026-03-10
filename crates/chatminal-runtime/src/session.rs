use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc as std_mpsc;
use std::sync::mpsc::TrySendError;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

const INPUT_QUEUE_CAPACITY: usize = 256;
const CONTROL_WRITE_RETRY_BUDGET: usize = 3;
const CONTROL_WRITE_RETRY_TIMEOUT_MS: u64 = 2;

#[derive(Debug, Clone, Copy, Default)]
pub struct InputWriteStats {
    pub queue_full_hits: u64,
    pub retries: u64,
    pub drops: u64,
}

#[derive(Debug, Clone)]
pub enum WriteInputError {
    Closing,
    Disconnected,
    QueueFullDropped(InputWriteStats),
}

impl std::fmt::Display for WriteInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closing => write!(f, "session runtime is closing"),
            Self::Disconnected => write!(f, "session runtime input channel disconnected"),
            Self::QueueFullDropped(stats) => write!(
                f,
                "queue input dropped after backpressure (queue_full_hits={} retries={})",
                stats.queue_full_hits, stats.retries
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Output {
        session_id: String,
        generation: u64,
        chunk: String,
        ts: u64,
    },
    Exited {
        session_id: String,
        generation: u64,
        exit_code: Option<i32>,
        reason: String,
    },
    Error {
        session_id: String,
        generation: u64,
        message: String,
    },
}

pub struct SessionRuntime {
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    input_tx: Option<std_mpsc::SyncSender<Vec<u8>>>,
    reader_handle: Option<thread::JoinHandle<()>>,
    writer_handle: Option<thread::JoinHandle<()>>,
    waiter_handle: Option<thread::JoinHandle<()>>,
}

impl SessionRuntime {
    pub fn spawn(
        session_id: String,
        generation: u64,
        shell: String,
        cwd: String,
        cols: usize,
        rows: usize,
        events: std_mpsc::SyncSender<SessionEvent>,
    ) -> Result<Self, String> {
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: rows.clamp(2, u16::MAX as usize) as u16,
                cols: cols.clamp(2, u16::MAX as usize) as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("open pty failed: {err}"))?;

        let mut command = CommandBuilder::new(shell.clone());
        command.cwd(cwd);
        command.env("TERM", "xterm-256color");
        // On macOS/zsh, inheriting stale COLUMNS/LINES from the parent process
        // can make the first prompt render against the wrong width until SIGWINCH.
        command.env_remove("COLUMNS");
        command.env_remove("LINES");
        apply_macos_zsh_startup_shim(&mut command, &shell)?;

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| format!("spawn command failed: {err}"))?;
        let child = Arc::new(Mutex::new(child));

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| format!("clone reader failed: {err}"))?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|err| format!("take writer failed: {err}"))?;

        let (input_tx, input_rx) = std_mpsc::sync_channel::<Vec<u8>>(INPUT_QUEUE_CAPACITY);

        let reader_session = session_id.clone();
        let reader_events = events.clone();
        let reader_handle = thread::spawn(move || {
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        let chunk = String::from_utf8_lossy(&buffer[..read]).to_string();
                        let _ = reader_events.send(SessionEvent::Output {
                            session_id: reader_session.clone(),
                            generation,
                            chunk,
                            ts: now_millis(),
                        });
                    }
                    Err(err) => {
                        let _ = reader_events.send(SessionEvent::Error {
                            session_id: reader_session.clone(),
                            generation,
                            message: format!("pty read failed: {err}"),
                        });
                        break;
                    }
                }
            }
        });

        let writer_session = session_id.clone();
        let writer_events = events.clone();
        let writer_handle = thread::spawn(move || {
            while let Ok(chunk) = input_rx.recv() {
                if let Err(err) = writer.write_all(&chunk) {
                    let _ = writer_events.send(SessionEvent::Error {
                        session_id: writer_session.clone(),
                        generation,
                        message: format!("pty write failed: {err}"),
                    });
                    break;
                }
                let _ = writer.flush();
            }
        });

        let waiter_session = session_id;
        let waiter_events = events;
        let waiter_child = child.clone();
        let waiter_handle = thread::spawn(move || {
            loop {
                let polled = waiter_child
                    .lock()
                    .ok()
                    .and_then(|mut guard| guard.try_wait().ok())
                    .flatten();

                if let Some(status) = polled {
                    let _ = waiter_events.send(SessionEvent::Exited {
                        session_id: waiter_session,
                        generation,
                        exit_code: Some(status.exit_code() as i32),
                        reason: "eof".to_string(),
                    });
                    break;
                }

                thread::sleep(Duration::from_millis(120));
            }
        });

        Ok(Self {
            master: pair.master,
            child,
            input_tx: Some(input_tx),
            reader_handle: Some(reader_handle),
            writer_handle: Some(writer_handle),
            waiter_handle: Some(waiter_handle),
        })
    }

    pub fn write_input(&self, data: &str) -> Result<InputWriteStats, WriteInputError> {
        let Some(tx) = self.input_tx.as_ref() else {
            return Err(WriteInputError::Closing);
        };
        write_payload_with_backpressure(tx, data.as_bytes().to_vec())
    }

    pub fn resize(&self, cols: usize, rows: usize) -> Result<(), String> {
        self.master
            .resize(PtySize {
                rows: rows.clamp(2, u16::MAX as usize) as u16,
                cols: cols.clamp(2, u16::MAX as usize) as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("resize pty failed: {err}"))
    }

    #[cfg(test)]
    pub fn size(&self) -> Result<(usize, usize), String> {
        let size = self
            .master
            .get_size()
            .map_err(|err| format!("get pty size failed: {err}"))?;
        Ok((size.cols as usize, size.rows as usize))
    }

    pub fn kill(&mut self) {
        self.input_tx.take();
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
        self.reader_handle.take();
        self.writer_handle.take();
        self.waiter_handle.take();
    }
}

impl Drop for SessionRuntime {
    fn drop(&mut self) {
        self.kill();
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn is_control_priority_payload(payload: &[u8]) -> bool {
    if payload.len() != 1 {
        return false;
    }
    let byte = payload[0];
    byte < 0x20 || byte == 0x7f
}

fn write_payload_with_backpressure(
    tx: &std_mpsc::SyncSender<Vec<u8>>,
    mut payload: Vec<u8>,
) -> Result<InputWriteStats, WriteInputError> {
    let mut stats = InputWriteStats::default();
    match tx.try_send(payload) {
        Ok(()) => return Ok(stats),
        Err(TrySendError::Disconnected(_)) => return Err(WriteInputError::Disconnected),
        Err(TrySendError::Full(returned)) => {
            stats.queue_full_hits += 1;
            payload = returned;
        }
    }

    if !is_control_priority_payload(&payload) {
        stats.drops += 1;
        return Err(WriteInputError::QueueFullDropped(stats));
    }

    for _ in 0..CONTROL_WRITE_RETRY_BUDGET {
        stats.retries += 1;
        thread::sleep(Duration::from_millis(CONTROL_WRITE_RETRY_TIMEOUT_MS));
        match tx.try_send(payload) {
            Ok(()) => return Ok(stats),
            Err(TrySendError::Disconnected(_)) => return Err(WriteInputError::Disconnected),
            Err(TrySendError::Full(returned)) => {
                stats.queue_full_hits += 1;
                payload = returned;
            }
        }
    }

    stats.drops += 1;
    Err(WriteInputError::QueueFullDropped(stats))
}

fn apply_macos_zsh_startup_shim(command: &mut CommandBuilder, shell: &str) -> Result<(), String> {
    if !cfg!(target_os = "macos") || !is_zsh_shell(shell) {
        return Ok(());
    }

    let shim_dir = ensure_macos_zsh_startup_shim_dir()?;
    let original_zdotdir = std::env::var_os("ZDOTDIR")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/"))
                .into_os_string()
        });

    command.env("CHATMINAL_ORIGINAL_ZDOTDIR", original_zdotdir);
    command.env("ZDOTDIR", shim_dir.as_os_str());
    Ok(())
}

fn is_zsh_shell(shell: &str) -> bool {
    Path::new(shell)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("zsh"))
}

fn ensure_macos_zsh_startup_shim_dir() -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("chatminal-zsh-startup-{}", std::process::id()));
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("create zsh startup shim dir failed: {err}"))?;

    let zshenv = r#"typeset -gx CHATMINAL_SHIM_ZDOTDIR="$ZDOTDIR"
if [ -n "${CHATMINAL_ORIGINAL_ZDOTDIR:-}" ] && [ -r "${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshenv" ]; then
  source "${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshenv"
elif [ -r "$HOME/.zshenv" ]; then
  source "$HOME/.zshenv"
fi
typeset -gx ZDOTDIR="$CHATMINAL_SHIM_ZDOTDIR"
"#;
    let zshrc = r#"if [ -n "${CHATMINAL_ORIGINAL_ZDOTDIR:-}" ] && [ -r "${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshrc" ]; then
  source "${CHATMINAL_ORIGINAL_ZDOTDIR}/.zshrc"
elif [ -r "$HOME/.zshrc" ]; then
  source "$HOME/.zshrc"
fi
unsetopt PROMPT_SP
unsetopt PROMPT_CR
"#;

    std::fs::write(dir.join(".zshenv"), zshenv)
        .map_err(|err| format!("write zsh startup shim .zshenv failed: {err}"))?;
    std::fs::write(dir.join(".zshrc"), zshrc)
        .map_err(|err| format!("write zsh startup shim .zshrc failed: {err}"))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::{
        CONTROL_WRITE_RETRY_BUDGET, WriteInputError, ensure_macos_zsh_startup_shim_dir,
        is_control_priority_payload, is_zsh_shell, write_payload_with_backpressure,
    };
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn non_control_payload_drops_immediately_when_queue_is_full() {
        let (tx, _rx) = mpsc::sync_channel::<Vec<u8>>(1);
        tx.try_send(vec![b'x']).expect("seed queue");

        let err = write_payload_with_backpressure(&tx, b"echo".to_vec())
            .expect_err("non-control payload should drop on full queue");
        match err {
            WriteInputError::QueueFullDropped(stats) => {
                assert_eq!(stats.queue_full_hits, 1);
                assert_eq!(stats.retries, 0);
                assert_eq!(stats.drops, 1);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn control_payload_retries_and_succeeds_when_queue_frees() {
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(1);
        tx.try_send(vec![b'x']).expect("seed queue");

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1));
            let _ = rx.recv();
            let _ = rx.recv_timeout(Duration::from_millis(50));
        });

        let stats = write_payload_with_backpressure(&tx, vec![0x03])
            .expect("control payload should be retried then delivered");
        assert!(stats.queue_full_hits >= 1);
        assert!(stats.retries >= 1);
        assert_eq!(stats.drops, 0);
    }

    #[test]
    fn control_payload_drops_after_retry_budget_if_queue_stays_full() {
        let (tx, _rx) = mpsc::sync_channel::<Vec<u8>>(1);
        tx.try_send(vec![b'x']).expect("seed queue");

        let err = write_payload_with_backpressure(&tx, vec![0x03])
            .expect_err("control payload should drop when queue remains full");
        match err {
            WriteInputError::QueueFullDropped(stats) => {
                assert_eq!(stats.retries, CONTROL_WRITE_RETRY_BUDGET as u64);
                assert_eq!(stats.drops, 1);
                assert!(stats.queue_full_hits >= 1);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn control_priority_payload_classifier_matches_terminal_controls() {
        assert!(is_control_priority_payload(&[0x03]));
        assert!(is_control_priority_payload(&[0x1a]));
        assert!(is_control_priority_payload(&[0x7f]));
        assert!(!is_control_priority_payload(b"ab"));
        assert!(!is_control_priority_payload(&[b'a']));
    }

    #[test]
    fn zsh_shell_detector_matches_expected_names() {
        assert!(is_zsh_shell("/bin/zsh"));
        assert!(is_zsh_shell("zsh"));
        assert!(!is_zsh_shell("/bin/bash"));
    }

    #[test]
    fn startup_shim_files_can_be_materialized() {
        let dir = ensure_macos_zsh_startup_shim_dir().expect("create shim dir");
        assert!(dir.join(".zshenv").is_file());
        assert!(dir.join(".zshrc").is_file());
    }
}
