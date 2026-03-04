use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use chatminal_protocol::{
    ClientFrame, CreateSessionResponse, Event, LifecyclePreferences, PingResponse, Request,
    Response, ServerFrame, SessionExplorerEntry, SessionExplorerFileContent, SessionExplorerState,
    SessionInfo, SessionSnapshot, SessionStatus, WorkspaceState,
};
use chatminal_store::{Store, StoredSession, StoredSessionExplorerState};

use crate::config::{DaemonConfig, resolve_session_cwd};
use crate::session::{SessionEvent, SessionRuntime};
const MAX_INPUT_BYTES: usize = 65_536;
const KEEP_ALIVE_ON_CLOSE_KEY: &str = "keep_alive_on_close";
const START_IN_TRAY_KEY: &str = "start_in_tray";
const DEFAULT_KEEP_ALIVE_ON_CLOSE: bool = true;
const DEFAULT_START_IN_TRAY: bool = false;
const MAX_EXPLORER_FILE_PREVIEW_BYTES: usize = 512 * 1024;
const MAX_EXPLORER_ENTRIES_PER_DIR: usize = 2_000;

struct SessionEntry {
    session: StoredSession,
    runtime: Option<SessionRuntime>,
    live_output: String,
    generation: u64,
}

struct StateInner {
    config: DaemonConfig,
    store: Store,
    sessions: HashMap<String, SessionEntry>,
    clients: HashMap<u64, std_mpsc::SyncSender<ServerFrame>>,
    shutdown_requested: bool,
}

#[derive(Clone)]
pub struct DaemonState {
    inner: Arc<Mutex<StateInner>>,
    events: std_mpsc::SyncSender<SessionEvent>,
}

impl DaemonState {
    pub fn new(config: DaemonConfig, store: Store) -> Result<Self, String> {
        let (events_tx, events_rx) = std_mpsc::sync_channel::<SessionEvent>(4096);
        let mut sessions = HashMap::new();
        let (profiles, _, _, _) = store.load_workspace()?;

        if profiles.is_empty() {
            return Err("store has no profiles".to_string());
        }

        for profile in profiles {
            for session in store.list_sessions_by_profile(&profile.profile_id)? {
                let stored = store
                    .get_session(&session.session_id)?
                    .ok_or_else(|| format!("session '{}' missing in store", session.session_id))?;
                sessions.insert(
                    stored.session_id.clone(),
                    SessionEntry {
                        session: stored,
                        runtime: None,
                        live_output: String::new(),
                        generation: 0,
                    },
                );
            }
        }

        // Keep disconnected state at startup; clients will reactivate when needed.
        for session_id in sessions.keys() {
            let _ = store.set_session_status(session_id, SessionStatus::Disconnected);
        }

        let state = Self {
            inner: Arc::new(Mutex::new(StateInner {
                config,
                store,
                sessions,
                clients: HashMap::new(),
                shutdown_requested: false,
            })),
            events: events_tx.clone(),
        };

        let cloned = state.clone();
        std::thread::spawn(move || {
            while let Ok(event) = events_rx.recv() {
                cloned.apply_session_event(event);
            }
        });

        Ok(state)
    }

    pub fn register_client(&self, client_id: u64, tx: std_mpsc::SyncSender<ServerFrame>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.clients.insert(client_id, tx);
            inner.broadcast_daemon_health();
        }
    }

    pub fn unregister_client(&self, client_id: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.clients.remove(&client_id);
            inner.broadcast_daemon_health();
        }
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.inner
            .lock()
            .map(|inner| inner.shutdown_requested)
            .unwrap_or(true)
    }

    pub fn health_interval_ms(&self) -> u64 {
        self.inner
            .lock()
            .map(|inner| inner.config.health_interval_ms)
            .unwrap_or(5_000)
    }

    pub fn broadcast_daemon_health(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.broadcast_daemon_health();
        }
    }

    pub fn handle_request(&self, frame: ClientFrame) -> ServerFrame {
        let id = frame.id;
        let mut inner = match self.inner.lock() {
            Ok(value) => value,
            Err(_) => return ServerFrame::err(id, "state lock poisoned".to_string()),
        };

        let result = inner.handle_request(frame.request, self.events.clone());
        match result {
            Ok(response) => ServerFrame::ok(id, response),
            Err(err) => ServerFrame::err(id, err),
        }
    }

    fn apply_session_event(&self, event: SessionEvent) {
        let mut inner = match self.inner.lock() {
            Ok(value) => value,
            Err(_) => return,
        };

        match event {
            SessionEvent::Output {
                session_id,
                generation,
                chunk,
                ts,
            } => {
                let mut frame = None;
                let mut seq_after = None;
                let mut persist_history = false;
                if let Some(entry) = inner.sessions.get_mut(&session_id) {
                    if entry.generation != generation {
                        return;
                    }
                    entry.session.seq += 1;
                    entry.session.status = SessionStatus::Running;
                    seq_after = Some(entry.session.seq);
                    persist_history = entry.session.persist_history;
                    if !persist_history {
                        entry.live_output.push_str(&chunk);
                        trim_live_output(&mut entry.live_output, 1024 * 1024);
                    }

                    frame = Some(ServerFrame::event(Event::PtyOutput(
                        chatminal_protocol::PtyOutputEvent {
                            session_id: session_id.clone(),
                            chunk: chunk.clone(),
                            seq: entry.session.seq,
                            ts,
                        },
                    )));
                }

                if let Some(seq) = seq_after {
                    if let Err(err) = inner.store.update_session_seq(&session_id, seq) {
                        inner.broadcast(ServerFrame::event(Event::PtyError(
                            chatminal_protocol::PtyErrorEvent {
                                session_id: session_id.clone(),
                                message: format!("persist seq failed: {err}"),
                            },
                        )));
                    }
                    if let Err(err) = inner
                        .store
                        .set_session_status(&session_id, SessionStatus::Running)
                    {
                        inner.broadcast(ServerFrame::event(Event::PtyError(
                            chatminal_protocol::PtyErrorEvent {
                                session_id: session_id.clone(),
                                message: format!("persist status failed: {err}"),
                            },
                        )));
                    }
                    if persist_history {
                        if let Err(err) = inner
                            .store
                            .append_scrollback_chunk(&session_id, seq, &chunk, ts)
                        {
                            inner.broadcast(ServerFrame::event(Event::PtyError(
                                chatminal_protocol::PtyErrorEvent {
                                    session_id: session_id.clone(),
                                    message: format!("persist chunk failed: {err}"),
                                },
                            )));
                        } else if let Err(err) =
                            inner.store.enforce_session_scrollback_line_limit(
                                &session_id,
                                inner.config.max_scrollback_lines_per_session,
                            )
                        {
                            inner.broadcast(ServerFrame::event(Event::PtyError(
                                chatminal_protocol::PtyErrorEvent {
                                    session_id: session_id.clone(),
                                    message: format!("apply retention failed: {err}"),
                                },
                            )));
                        }
                    }
                }

                if let Some(frame) = frame {
                    inner.broadcast(frame);
                }
            }
            SessionEvent::Exited {
                session_id,
                generation,
                exit_code,
                reason,
            } => {
                let mut updated = false;
                if let Some(entry) = inner.sessions.get_mut(&session_id) {
                    if entry.generation != generation {
                        return;
                    }
                    entry.runtime = None;
                    entry.session.status = SessionStatus::Disconnected;
                    let _ = inner
                        .store
                        .set_session_status(&session_id, SessionStatus::Disconnected);
                    updated = true;
                }

                inner.broadcast(ServerFrame::event(Event::PtyExited(
                    chatminal_protocol::PtyExitedEvent {
                        session_id: session_id.clone(),
                        exit_code,
                        reason,
                    },
                )));
                if updated {
                    inner.publish_session_updated_for(&session_id);
                    inner.publish_workspace_updated();
                }
            }
            SessionEvent::Error {
                session_id,
                generation,
                message,
            } => {
                if let Some(entry) = inner.sessions.get(&session_id)
                    && entry.generation != generation
                {
                    return;
                }
                inner.broadcast(ServerFrame::event(Event::PtyError(
                    chatminal_protocol::PtyErrorEvent {
                        session_id,
                        message,
                    },
                )));
            }
        }
    }
}

impl StateInner {
    fn handle_request(
        &mut self,
        request: Request,
        events: std_mpsc::SyncSender<SessionEvent>,
    ) -> Result<Response, String> {
        match request {
            Request::Ping => Ok(Response::Ping(PingResponse {
                message: "pong chatminald/1".to_string(),
            })),
            Request::LifecyclePreferencesGet => {
                Ok(Response::LifecyclePreferences(self.get_lifecycle_preferences()?))
            }
            Request::LifecyclePreferencesSet {
                keep_alive_on_close,
                start_in_tray,
            } => Ok(Response::LifecyclePreferences(self.set_lifecycle_preferences(
                keep_alive_on_close,
                start_in_tray,
            )?)),
            Request::WorkspaceLoad => {
                self.ensure_active_session_runtime(events.clone())?;
                Ok(Response::Workspace(self.load_workspace()?))
            }
            Request::ProfileList => Ok(Response::Profiles(self.store.list_profiles()?)),
            Request::ProfileCreate { name } => {
                let created = self.store.create_profile(name)?;
                self.publish_workspace_updated();
                Ok(Response::Profile(created))
            }
            Request::ProfileRename { profile_id, name } => {
                let renamed = self.store.rename_profile(&profile_id, &name)?;
                self.publish_workspace_updated();
                Ok(Response::Profile(renamed))
            }
            Request::ProfileDelete { profile_id } => {
                self.store.delete_profile(&profile_id)?;
                self.close_profile_runtimes(&profile_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::ProfileSwitch { profile_id } => {
                let exists = self
                    .store
                    .list_profiles()?
                    .iter()
                    .any(|value| value.profile_id == profile_id);
                if !exists {
                    return Err("profile not found".to_string());
                }
                self.store.set_active_profile(&profile_id)?;
                self.publish_workspace_updated();
                Ok(Response::Workspace(self.load_workspace()?))
            }
            Request::SessionList => {
                let (_, active_profile_id, sessions, _) = self.store.load_workspace()?;
                let filtered: Vec<SessionInfo> = sessions
                    .into_iter()
                    .filter(|value| value.profile_id == active_profile_id)
                    .collect();
                Ok(Response::Sessions(filtered))
            }
            Request::SessionCreate {
                name,
                cols,
                rows,
                cwd,
                persist_history,
            } => {
                let (_, active_profile_id, _, _) = self.store.load_workspace()?;
                let created = self.store.create_session(
                    &active_profile_id,
                    name,
                    resolve_session_cwd(cwd),
                    self.config.default_shell.clone(),
                    persist_history.unwrap_or(false),
                )?;

                self.store
                    .set_active_session(&active_profile_id, Some(&created.session_id))?;

                let runtime = SessionRuntime::spawn(
                    created.session_id.clone(),
                    0,
                    created.shell.clone(),
                    created.cwd.clone(),
                    cols,
                    rows,
                    events,
                )
                .map_err(|err| {
                    let _ = self.store.delete_session(&created.session_id);
                    err
                })?;
                self.store
                    .set_session_status(&created.session_id, SessionStatus::Running)?;

                let mut entry = SessionEntry {
                    session: created.clone(),
                    runtime: Some(runtime),
                    live_output: String::new(),
                    generation: 0,
                };
                entry.session.status = SessionStatus::Running;
                self.sessions.insert(created.session_id.clone(), entry);
                self.publish_session_updated_for(&created.session_id);
                self.publish_workspace_updated();

                Ok(Response::SessionCreate(CreateSessionResponse {
                    session_id: created.session_id,
                    name: created.name,
                }))
            }
            Request::SessionActivate {
                session_id,
                cols,
                rows,
            } => {
                let profile_id = if let Some(entry) = self.sessions.get(&session_id) {
                    entry.session.profile_id.clone()
                } else {
                    return Err("session not found".to_string());
                };

                let Some(entry) = self.sessions.get_mut(&session_id) else {
                    return Err("session not found".to_string());
                };

                if entry.runtime.is_none() {
                    entry.generation = entry.generation.saturating_add(1);
                    let runtime = SessionRuntime::spawn(
                        entry.session.session_id.clone(),
                        entry.generation,
                        entry.session.shell.clone(),
                        entry.session.cwd.clone(),
                        cols,
                        rows,
                        events,
                    )?;
                    entry.runtime = Some(runtime);
                    entry.session.status = SessionStatus::Running;
                    self.store
                        .set_session_status(&entry.session.session_id, SessionStatus::Running)?;
                }
                self.store
                    .set_active_session(&profile_id, Some(&session_id))?;
                self.publish_session_updated_for(&session_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::SessionRename { session_id, name } => {
                self.store.rename_session(&session_id, &name)?;
                if let Some(entry) = self.sessions.get_mut(&session_id) {
                    entry.session.name = name.trim().to_string();
                }
                self.publish_session_updated_for(&session_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::SessionClose { session_id } => {
                self.store.delete_session(&session_id)?;
                if let Some(mut entry) = self.sessions.remove(&session_id) {
                    if let Some(mut runtime) = entry.runtime.take() {
                        runtime.kill();
                    }
                }
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::SessionSetPersist {
                session_id,
                persist_history,
            } => {
                let mut flush_seq: Option<u64> = None;
                let mut flush_chunk: Option<String> = None;
                if let Some(entry) = self.sessions.get(&session_id) {
                    if entry.session.persist_history != persist_history
                        && persist_history
                        && !entry.live_output.is_empty()
                    {
                        flush_seq = Some(entry.session.seq.saturating_add(1));
                        flush_chunk = Some(entry.live_output.clone());
                    }
                }

                self.store
                    .set_session_persist(&session_id, persist_history)?;
                if let (Some(seq), Some(chunk)) = (flush_seq, flush_chunk.as_ref()) {
                    let ts = now_millis();
                    self.store.update_session_seq(&session_id, seq)?;
                    self.store
                        .append_scrollback_chunk(&session_id, seq, chunk, ts)?;
                    self.store.enforce_session_scrollback_line_limit(
                        &session_id,
                        self.config.max_scrollback_lines_per_session,
                    )?;
                }
                if let Some(entry) = self.sessions.get_mut(&session_id) {
                    if entry.session.persist_history != persist_history {
                        if persist_history {
                            if let Some(seq) = flush_seq {
                                entry.session.seq = seq;
                                entry.live_output.clear();
                            }
                        } else {
                            entry.live_output.clear();
                        }
                    }
                    entry.session.persist_history = persist_history;
                }
                self.publish_session_updated_for(&session_id);
                Ok(Response::Empty)
            }
            Request::SessionInputWrite { session_id, data } => {
                if data.len() > MAX_INPUT_BYTES {
                    return Err(format!(
                        "input payload too large ({} bytes > {} bytes)",
                        data.len(),
                        MAX_INPUT_BYTES
                    ));
                }
                let Some(entry) = self.sessions.get_mut(&session_id) else {
                    return Err("session not found".to_string());
                };
                let Some(runtime) = entry.runtime.as_ref() else {
                    return Err("session is not running".to_string());
                };
                runtime.write_input(&data)?;
                Ok(Response::Empty)
            }
            Request::SessionResize {
                session_id,
                cols,
                rows,
            } => {
                let Some(entry) = self.sessions.get_mut(&session_id) else {
                    return Err("session not found".to_string());
                };
                let Some(runtime) = entry.runtime.as_ref() else {
                    return Err("session is not running".to_string());
                };
                runtime.resize(cols, rows)?;
                Ok(Response::Empty)
            }
            Request::SessionSnapshotGet {
                session_id,
                preview_lines,
            } => {
                if !self.sessions.contains_key(&session_id) {
                    return Err("session not found".to_string());
                }

                let from_store = self.store.session_snapshot(
                    &session_id,
                    preview_lines.unwrap_or(self.config.default_preview_lines),
                )?;
                let merged = if let Some(entry) = self.sessions.get(&session_id) {
                    if entry.live_output.is_empty() || entry.session.persist_history {
                        from_store
                    } else {
                        SessionSnapshot {
                            content: format!("{}{}", from_store.content, entry.live_output),
                            seq: entry.session.seq.max(from_store.seq),
                        }
                    }
                } else {
                    from_store
                };
                Ok(Response::SessionSnapshot(merged))
            }
            Request::SessionExplorerStateGet { session_id } => Ok(Response::SessionExplorerState(
                self.get_session_explorer_state(&session_id)?,
            )),
            Request::SessionExplorerRootSet {
                session_id,
                root_path,
            } => Ok(Response::SessionExplorerState(
                self.set_session_explorer_root(&session_id, &root_path)?,
            )),
            Request::SessionExplorerStateUpdate {
                session_id,
                current_dir,
                selected_path,
                open_file_path,
            } => Ok(Response::SessionExplorerState(
                self.update_session_explorer_state(
                    &session_id,
                    &current_dir,
                    selected_path.as_deref(),
                    open_file_path.as_deref(),
                )?,
            )),
            Request::SessionExplorerList {
                session_id,
                relative_path,
            } => Ok(Response::SessionExplorerEntries(
                self.list_session_explorer_entries(&session_id, relative_path.as_deref())?,
            )),
            Request::SessionExplorerReadFile {
                session_id,
                relative_path,
                max_bytes,
            } => Ok(Response::SessionExplorerFileContent(
                self.read_session_explorer_file(&session_id, &relative_path, max_bytes)?,
            )),
            Request::SessionHistoryClear { session_id } => {
                self.store.clear_session_history(&session_id)?;
                if let Some(entry) = self.sessions.get_mut(&session_id) {
                    entry.generation = entry.generation.saturating_add(1);
                    entry.session.seq = 0;
                    entry.live_output.clear();
                    if let Some(mut runtime) = entry.runtime.take() {
                        runtime.kill();
                    }
                    entry.session.status = SessionStatus::Disconnected;
                    self.store
                        .set_session_status(&session_id, SessionStatus::Disconnected)?;
                }
                self.publish_session_updated_for(&session_id);
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::WorkspaceHistoryClearAll => {
                self.store.clear_all_history()?;
                let mut updated_ids = Vec::new();
                for entry in self.sessions.values_mut() {
                    entry.generation = entry.generation.saturating_add(1);
                    entry.session.seq = 0;
                    entry.live_output.clear();
                    if let Some(mut runtime) = entry.runtime.take() {
                        runtime.kill();
                    }
                    entry.session.status = SessionStatus::Disconnected;
                    let _ = self
                        .store
                        .set_session_status(&entry.session.session_id, SessionStatus::Disconnected);
                    updated_ids.push(entry.session.session_id.clone());
                }
                for session_id in updated_ids {
                    self.publish_session_updated_for(&session_id);
                }
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
            Request::AppShutdown => {
                self.shutdown_requested = true;
                let mut updated_ids = Vec::new();
                for entry in self.sessions.values_mut() {
                    if let Some(mut runtime) = entry.runtime.take() {
                        runtime.kill();
                    }
                    entry.session.status = SessionStatus::Disconnected;
                    let _ = self
                        .store
                        .set_session_status(&entry.session.session_id, SessionStatus::Disconnected);
                    updated_ids.push(entry.session.session_id.clone());
                }
                for session_id in updated_ids {
                    self.publish_session_updated_for(&session_id);
                }
                self.publish_workspace_updated();
                Ok(Response::Empty)
            }
        }
    }

    fn load_workspace(&self) -> Result<WorkspaceState, String> {
        let (profiles, active_profile_id, mut sessions, active_session_id) =
            self.store.load_workspace()?;
        for session in &mut sessions {
            if let Some(entry) = self.sessions.get(&session.session_id) {
                session.status = entry.session.status.clone();
                session.seq = entry.session.seq;
                session.persist_history = entry.session.persist_history;
                session.cwd = entry.session.cwd.clone();
                session.name = entry.session.name.clone();
            }
        }

        Ok(WorkspaceState {
            profiles,
            active_profile_id: Some(active_profile_id),
            sessions,
            active_session_id,
        })
    }

    fn get_lifecycle_preferences(&self) -> Result<LifecyclePreferences, String> {
        Ok(LifecyclePreferences {
            keep_alive_on_close: self
                .store
                .get_bool_state(KEEP_ALIVE_ON_CLOSE_KEY, DEFAULT_KEEP_ALIVE_ON_CLOSE)?,
            start_in_tray: self
                .store
                .get_bool_state(START_IN_TRAY_KEY, DEFAULT_START_IN_TRAY)?,
        })
    }

    fn set_lifecycle_preferences(
        &self,
        keep_alive_on_close: Option<bool>,
        start_in_tray: Option<bool>,
    ) -> Result<LifecyclePreferences, String> {
        if let Some(next) = keep_alive_on_close {
            self.store.set_bool_state(KEEP_ALIVE_ON_CLOSE_KEY, next)?;
        }
        if let Some(next) = start_in_tray {
            self.store.set_bool_state(START_IN_TRAY_KEY, next)?;
        }
        self.get_lifecycle_preferences()
    }

    fn get_session_explorer_state(&self, session_id: &str) -> Result<SessionExplorerState, String> {
        self.ensure_session_exists(session_id)?;
        let state = self.store.get_session_explorer_state(session_id)?;
        Ok(explorer_state_to_protocol(session_id, state))
    }

    fn set_session_explorer_root(
        &self,
        session_id: &str,
        root_path: &str,
    ) -> Result<SessionExplorerState, String> {
        self.ensure_session_exists(session_id)?;
        let root = resolve_explorer_root_path(root_path)?;
        let saved = self
            .store
            .set_session_explorer_root(session_id, &root.to_string_lossy())?;
        Ok(explorer_state_to_protocol(session_id, Some(saved)))
    }

    fn update_session_explorer_state(
        &self,
        session_id: &str,
        current_dir: &str,
        selected_path: Option<&str>,
        open_file_path: Option<&str>,
    ) -> Result<SessionExplorerState, String> {
        self.ensure_session_exists(session_id)?;
        let Some(current_state) = self.store.get_session_explorer_state(session_id)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&current_state.root_path)?;
        let normalized_current_dir = normalize_relative_path(current_dir)?;
        let current_dir_path = resolve_explorer_target(&root, &normalized_current_dir)?;
        if !current_dir_path.is_dir() {
            return Err("current explorer directory is not valid".to_string());
        }

        let selected = match selected_path {
            Some(value) => {
                let normalized = normalize_relative_path(value)?;
                if !normalized.is_empty() {
                    let target = resolve_explorer_target(&root, &normalized)?;
                    if !target.exists() {
                        return Err("selected explorer path does not exist".to_string());
                    }
                }
                Some(normalized)
            }
            None => None,
        };

        let open_file = match open_file_path {
            Some(value) => {
                let normalized = normalize_relative_path(value)?;
                let target = resolve_explorer_target(&root, &normalized)?;
                if !target.is_file() {
                    return Err("open file path is not a file".to_string());
                }
                Some(normalized)
            }
            None => None,
        };

        let saved = self.store.update_session_explorer_state(
            session_id,
            &normalized_current_dir,
            selected.as_deref(),
            open_file.as_deref(),
        )?;
        Ok(explorer_state_to_protocol(session_id, Some(saved)))
    }

    fn list_session_explorer_entries(
        &self,
        session_id: &str,
        relative_path: Option<&str>,
    ) -> Result<Vec<SessionExplorerEntry>, String> {
        self.ensure_session_exists(session_id)?;
        let Some(state) = self.store.get_session_explorer_state(session_id)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&state.root_path)?;
        let target_relative = match relative_path {
            Some(value) => normalize_relative_path(value)?,
            None => state.current_dir,
        };
        let target_dir = resolve_explorer_target(&root, &target_relative)?;
        if !target_dir.is_dir() {
            return Err("explorer target is not a directory".to_string());
        }
        let lexical_target_dir = if target_relative.is_empty() {
            root.clone()
        } else {
            root.join(&target_relative)
        };

        let mut entries = Vec::new();
        let read_dir = std::fs::read_dir(&lexical_target_dir)
            .map_err(|err| format!("read explorer directory failed: {err}"))?;
        for item in read_dir {
            if entries.len() >= MAX_EXPLORER_ENTRIES_PER_DIR {
                break;
            }

            let entry = match item {
                Ok(value) => value,
                Err(_) => continue,
            };
            let entry_path = entry.path();
            let lexical_relative = match entry_path.strip_prefix(&root) {
                Ok(value) => value.to_path_buf(),
                Err(_) => continue,
            };

            let canonical = match std::fs::canonicalize(&entry_path) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if !canonical.starts_with(&root) {
                continue;
            }

            let metadata = match std::fs::metadata(&canonical) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let is_dir = metadata.is_dir();
            let relative = normalize_relative_path(&lexical_relative.to_string_lossy())?;
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(SessionExplorerEntry {
                name,
                relative_path: relative,
                is_dir,
                size: if is_dir { None } else { Some(metadata.len()) },
            });
        }

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase()),
        });
        Ok(entries)
    }

    fn read_session_explorer_file(
        &self,
        session_id: &str,
        relative_path: &str,
        max_bytes: Option<usize>,
    ) -> Result<SessionExplorerFileContent, String> {
        self.ensure_session_exists(session_id)?;
        let Some(state) = self.store.get_session_explorer_state(session_id)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&state.root_path)?;
        let normalized = normalize_relative_path(relative_path)?;
        let target = resolve_explorer_target(&root, &normalized)?;
        if !target.is_file() {
            return Err("explorer target is not a file".to_string());
        }

        let max_bytes = max_bytes
            .unwrap_or(256 * 1024)
            .clamp(1_024, MAX_EXPLORER_FILE_PREVIEW_BYTES);
        let file = File::open(&target).map_err(|err| format!("open explorer file failed: {err}"))?;
        let mut buffer = Vec::new();
        file.take((max_bytes + 1) as u64)
            .read_to_end(&mut buffer)
            .map_err(|err| format!("read explorer file failed: {err}"))?;

        let truncated = buffer.len() > max_bytes;
        if truncated {
            buffer.truncate(max_bytes);
        }
        if buffer.contains(&0) {
            return Err("binary file preview is not supported yet".to_string());
        }

        Ok(SessionExplorerFileContent {
            relative_path: normalized,
            content: String::from_utf8_lossy(&buffer).to_string(),
            truncated,
            byte_len: buffer.len(),
        })
    }

    fn ensure_session_exists(&self, session_id: &str) -> Result<(), String> {
        if self.sessions.contains_key(session_id) {
            return Ok(());
        }
        Err("session not found".to_string())
    }

    fn ensure_active_session_runtime(
        &mut self,
        events: std_mpsc::SyncSender<SessionEvent>,
    ) -> Result<(), String> {
        let (_, active_profile_id, _, active_session_id) = self.store.load_workspace()?;
        let Some(session_id) = active_session_id else {
            return Ok(());
        };

        let mut started = false;
        if let Some(entry) = self.sessions.get_mut(&session_id) {
            if entry.session.profile_id == active_profile_id && entry.runtime.is_none() {
                entry.generation = entry.generation.saturating_add(1);
                let runtime = SessionRuntime::spawn(
                    entry.session.session_id.clone(),
                    entry.generation,
                    entry.session.shell.clone(),
                    entry.session.cwd.clone(),
                    self.config.default_cols,
                    self.config.default_rows,
                    events,
                )?;
                entry.runtime = Some(runtime);
                entry.session.status = SessionStatus::Running;
                self.store
                    .set_session_status(&entry.session.session_id, SessionStatus::Running)?;
                started = true;
            }
        }

        if started {
            self.publish_session_updated_for(&session_id);
            self.publish_workspace_updated();
        }
        Ok(())
    }

    fn publish_session_updated_for(&mut self, session_id: &str) {
        if let Some(entry) = self.sessions.get(session_id) {
            self.broadcast(ServerFrame::event(Event::SessionUpdated(
                chatminal_protocol::SessionUpdatedEvent {
                    session_id: session_id.to_string(),
                    status: entry.session.status.clone(),
                    seq: entry.session.seq,
                    persist_history: entry.session.persist_history,
                    ts: now_millis(),
                },
            )));
        }
    }

    fn publish_workspace_updated(&mut self) {
        if let Ok((profiles, active_profile_id, sessions, active_session_id)) =
            self.store.load_workspace()
        {
            self.broadcast(ServerFrame::event(Event::WorkspaceUpdated(
                chatminal_protocol::WorkspaceUpdatedEvent {
                    active_profile_id: Some(active_profile_id),
                    active_session_id,
                    profile_count: profiles.len() as u64,
                    session_count: sessions.len() as u64,
                    ts: now_millis(),
                },
            )));
        }
    }

    fn broadcast_daemon_health(&mut self) {
        let running_sessions = self
            .sessions
            .values()
            .filter(|entry| entry.runtime.is_some())
            .count() as u64;
        self.broadcast(ServerFrame::event(Event::DaemonHealth(
            chatminal_protocol::DaemonHealthEvent {
                connected_clients: self.clients.len() as u64,
                session_count: self.sessions.len() as u64,
                running_sessions,
                ts: now_millis(),
            },
        )));
    }

    fn close_profile_runtimes(&mut self, profile_id: &str) {
        let target_ids: Vec<String> = self
            .sessions
            .iter()
            .filter_map(|(session_id, entry)| {
                if entry.session.profile_id == profile_id {
                    Some(session_id.clone())
                } else {
                    None
                }
            })
            .collect();

        for session_id in target_ids {
            if let Some(mut entry) = self.sessions.remove(&session_id)
                && let Some(mut runtime) = entry.runtime.take()
            {
                runtime.kill();
            }
        }
    }

    fn broadcast(&mut self, frame: ServerFrame) {
        self.clients
            .retain(|_, tx| match tx.try_send(frame.clone()) {
                Ok(_) => true,
                Err(std_mpsc::TrySendError::Full(_)) => false,
                Err(std_mpsc::TrySendError::Disconnected(_)) => false,
            });
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn explorer_state_to_protocol(
    session_id: &str,
    state: Option<StoredSessionExplorerState>,
) -> SessionExplorerState {
    if let Some(value) = state {
        return SessionExplorerState {
            session_id: value.session_id,
            root_path: Some(value.root_path),
            current_dir: value.current_dir,
            selected_path: value.selected_path,
            open_file_path: value.open_file_path,
        };
    }

    SessionExplorerState {
        session_id: session_id.to_string(),
        root_path: None,
        current_dir: String::new(),
        selected_path: None,
        open_file_path: None,
    }
}

fn normalize_relative_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err("absolute path is not allowed in session explorer".to_string());
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(value) => normalized.push(value),
            std::path::Component::ParentDir => {
                return Err("parent path '..' is not allowed in session explorer".to_string());
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err("invalid path component in session explorer".to_string());
            }
        }
    }

    Ok(normalized.to_string_lossy().replace('\\', "/"))
}

fn resolve_explorer_root_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("explorer root path cannot be empty".to_string());
    }

    let canonical = std::fs::canonicalize(trimmed)
        .map_err(|err| format!("invalid explorer root '{trimmed}': {err}"))?;
    if !canonical.is_dir() {
        return Err("explorer root is not a directory".to_string());
    }
    Ok(canonical)
}

fn resolve_explorer_target(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let normalized = normalize_relative_path(relative)?;
    let joined = if normalized.is_empty() {
        root.to_path_buf()
    } else {
        root.join(normalized)
    };

    let canonical = std::fs::canonicalize(&joined)
        .map_err(|err| format!("invalid explorer path '{}': {err}", joined.display()))?;
    if !canonical.starts_with(root) {
        return Err("explorer path escapes selected root".to_string());
    }
    Ok(canonical)
}

fn trim_live_output(buffer: &mut String, max_bytes: usize) {
    if buffer.len() <= max_bytes {
        return;
    }

    let overflow = buffer.len().saturating_sub(max_bytes);
    let mut cut = overflow;
    while cut < buffer.len() && !buffer.is_char_boundary(cut) {
        cut += 1;
    }
    buffer.drain(..cut.min(buffer.len()));
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chatminal_protocol::{ClientFrame, Request, Response, ServerBody, SessionStatus};
    use chatminal_store::Store;

    use crate::config::DaemonConfig;
    use crate::session::SessionEvent;

    use super::{DaemonState, normalize_relative_path, resolve_explorer_target, trim_live_output};

    struct TempDb {
        path: PathBuf,
    }

    impl TempDb {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "chatminald-state-test-{}-{}.db",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_nanos())
                    .unwrap_or(0)
            ));
            Self { path }
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn create_state_with_session() -> (DaemonState, String, TempDb) {
        let db = TempDb::new();
        let store = Store::initialize(&db.path).expect("initialize test store");
        let (_, active_profile_id, _, _) = store.load_workspace().expect("load workspace");
        let session = store
            .create_session(
                &active_profile_id,
                Some("State Test".to_string()),
                "/tmp".to_string(),
                "/bin/sh".to_string(),
                false,
            )
            .expect("create session");
        store
            .set_active_session(&active_profile_id, Some(&session.session_id))
            .expect("set active session");

        let config = DaemonConfig {
            endpoint: format!(
                "/tmp/chatminald-state-{}-{}.sock",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_millis())
                    .unwrap_or(0)
            ),
            default_shell: "/bin/sh".to_string(),
            default_preview_lines: 1000,
            max_scrollback_lines_per_session: 5_000,
            default_cols: 120,
            default_rows: 32,
            health_interval_ms: 1_000,
        };
        let state = DaemonState::new(config, store).expect("create daemon state");
        (state, session.session_id, db)
    }

    fn request_ok(state: &DaemonState, request: Request) -> Response {
        let frame = state.handle_request(ClientFrame {
            id: "test-request".to_string(),
            request,
        });
        match frame.body {
            ServerBody::Response {
                ok: true,
                response: Some(response),
                ..
            } => response,
            ServerBody::Response {
                ok: false,
                error: Some(error),
                ..
            } => panic!("request failed unexpectedly: {error}"),
            other => panic!("unexpected frame body: {other:?}"),
        }
    }

    #[test]
    fn trim_live_output_keeps_tail_for_ascii() {
        let mut value = "abcdef".to_string();
        trim_live_output(&mut value, 4);
        assert_eq!(value, "cdef");
    }

    #[test]
    fn trim_live_output_preserves_utf8_boundaries() {
        let mut value = "ééé".to_string();
        trim_live_output(&mut value, 5);
        assert_eq!(value, "éé");
    }

    #[test]
    fn stale_output_event_is_ignored_by_generation_guard() {
        let (state, session_id, _db) = create_state_with_session();
        {
            let mut inner = state.inner.lock().expect("lock state");
            let entry = inner
                .sessions
                .get_mut(&session_id)
                .expect("session entry exists");
            entry.generation = 3;
            entry.session.seq = 7;
            entry.session.status = SessionStatus::Running;
        }

        state.apply_session_event(SessionEvent::Output {
            session_id: session_id.clone(),
            generation: 2,
            chunk: "ignored-output".to_string(),
            ts: 1,
        });

        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_id)
            .expect("session entry exists");
        assert_eq!(entry.session.seq, 7);
        assert_eq!(entry.session.status, SessionStatus::Running);
        assert!(entry.live_output.is_empty());
    }

    #[test]
    fn stale_exit_event_does_not_flip_session_status() {
        let (state, session_id, _db) = create_state_with_session();
        {
            let mut inner = state.inner.lock().expect("lock state");
            let entry = inner
                .sessions
                .get_mut(&session_id)
                .expect("session entry exists");
            entry.generation = 5;
            entry.session.status = SessionStatus::Running;
        }

        state.apply_session_event(SessionEvent::Exited {
            session_id: session_id.clone(),
            generation: 4,
            exit_code: Some(0),
            reason: "stale".to_string(),
        });

        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_id)
            .expect("session entry exists");
        assert_eq!(entry.session.status, SessionStatus::Running);
    }

    #[test]
    fn lifecycle_preferences_default_values() {
        let (state, _session_id, _db) = create_state_with_session();
        let inner = state.inner.lock().expect("lock state");
        let lifecycle = inner
            .get_lifecycle_preferences()
            .expect("get lifecycle preferences");
        assert!(lifecycle.keep_alive_on_close);
        assert!(!lifecycle.start_in_tray);
    }

    #[test]
    fn lifecycle_preferences_set_roundtrip() {
        let (state, _session_id, _db) = create_state_with_session();
        {
            let inner = state.inner.lock().expect("lock state");
            let updated = inner
                .set_lifecycle_preferences(Some(false), Some(true))
                .expect("set lifecycle preferences");
            assert!(!updated.keep_alive_on_close);
            assert!(updated.start_in_tray);
        }

        let inner = state.inner.lock().expect("lock state again");
        let loaded = inner
            .get_lifecycle_preferences()
            .expect("reload lifecycle preferences");
        assert!(!loaded.keep_alive_on_close);
        assert!(loaded.start_in_tray);
    }

    #[test]
    fn lifecycle_preferences_partial_update_keeps_other_key() {
        let (state, _session_id, _db) = create_state_with_session();
        {
            let inner = state.inner.lock().expect("lock state");
            let _ = inner
                .set_lifecycle_preferences(Some(false), None)
                .expect("set keep_alive only");
        }
        {
            let inner = state.inner.lock().expect("lock state");
            let loaded = inner
                .get_lifecycle_preferences()
                .expect("load lifecycle after first update");
            assert!(!loaded.keep_alive_on_close);
            assert!(!loaded.start_in_tray);
        }
        {
            let inner = state.inner.lock().expect("lock state");
            let _ = inner
                .set_lifecycle_preferences(None, Some(true))
                .expect("set start_in_tray only");
        }
        let inner = state.inner.lock().expect("lock state");
        let loaded = inner
            .get_lifecycle_preferences()
            .expect("load lifecycle after second update");
        assert!(!loaded.keep_alive_on_close);
        assert!(loaded.start_in_tray);
    }

    #[test]
    fn workspace_load_auto_connects_active_session_runtime() {
        let (state, session_id, _db) = create_state_with_session();

        let response = request_ok(&state, Request::WorkspaceLoad);
        let workspace = match response {
            Response::Workspace(value) => value,
            other => panic!("unexpected response: {other:?}"),
        };
        assert_eq!(workspace.active_session_id.as_deref(), Some(session_id.as_str()));

        {
            let inner = state.inner.lock().expect("lock state");
            let entry = inner
                .sessions
                .get(&session_id)
                .expect("session entry exists");
            assert!(entry.runtime.is_some());
            assert_eq!(entry.session.status, SessionStatus::Running);
        }

        let _ = request_ok(&state, Request::AppShutdown);
    }

    #[test]
    fn session_activate_increments_generation_on_each_spawn() {
        let (state, session_id, _db) = create_state_with_session();
        let generation_before = {
            let inner = state.inner.lock().expect("lock state");
            inner
                .sessions
                .get(&session_id)
                .expect("session entry exists")
                .generation
        };

        let _ = request_ok(
            &state,
            Request::SessionActivate {
                session_id: session_id.clone(),
                cols: 120,
                rows: 32,
            },
        );
        let generation_after_first = {
            let inner = state.inner.lock().expect("lock state");
            inner
                .sessions
                .get(&session_id)
                .expect("session entry exists")
                .generation
        };
        assert_eq!(generation_after_first, generation_before.saturating_add(1));

        let _ = request_ok(
            &state,
            Request::SessionHistoryClear {
                session_id: session_id.clone(),
            },
        );
        let _ = request_ok(
            &state,
            Request::SessionActivate {
                session_id: session_id.clone(),
                cols: 120,
                rows: 32,
            },
        );
        let generation_after_second = {
            let inner = state.inner.lock().expect("lock state");
            inner
                .sessions
                .get(&session_id)
                .expect("session entry exists")
                .generation
        };
        assert!(generation_after_second > generation_after_first);

        let _ = request_ok(&state, Request::AppShutdown);
    }

    #[test]
    fn session_history_clear_disconnects_runtime_and_resets_snapshot() {
        let (state, session_id, _db) = create_state_with_session();

        let _ = request_ok(
            &state,
            Request::SessionActivate {
                session_id: session_id.clone(),
                cols: 120,
                rows: 32,
            },
        );
        let _ = request_ok(
            &state,
            Request::SessionSetPersist {
                session_id: session_id.clone(),
                persist_history: true,
            },
        );

        state.apply_session_event(SessionEvent::Output {
            session_id: session_id.clone(),
            generation: 0,
            chunk: "echo hello\n".to_string(),
            ts: 11,
        });

        let _ = request_ok(
            &state,
            Request::SessionHistoryClear {
                session_id: session_id.clone(),
            },
        );

        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_id)
            .expect("session entry exists");
        assert!(entry.runtime.is_none());
        assert_eq!(entry.session.status, SessionStatus::Disconnected);
        assert_eq!(entry.session.seq, 0);

        let snapshot = inner
            .store
            .session_snapshot(&session_id, 100)
            .expect("load session snapshot after clear");
        assert_eq!(snapshot.seq, 0);
        assert!(snapshot.content.is_empty());
    }

    #[test]
    fn session_set_persist_flushes_live_output_into_store_snapshot() {
        let (state, session_id, _db) = create_state_with_session();
        {
            let mut inner = state.inner.lock().expect("lock state");
            let entry = inner
                .sessions
                .get_mut(&session_id)
                .expect("session entry exists");
            entry.live_output = "cached-line\n".to_string();
            entry.session.seq = 0;
            entry.session.persist_history = false;
        }

        let _ = request_ok(
            &state,
            Request::SessionSetPersist {
                session_id: session_id.clone(),
                persist_history: true,
            },
        );

        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_id)
            .expect("session entry exists");
        assert!(entry.live_output.is_empty());
        assert!(entry.session.persist_history);
        assert_eq!(entry.session.seq, 1);
        let snapshot = inner
            .store
            .session_snapshot(&session_id, 100)
            .expect("load snapshot");
        assert_eq!(snapshot.content, "cached-line\n");
    }

    #[test]
    fn persisted_history_applies_line_retention_limit() {
        let (state, session_id, _db) = create_state_with_session();
        {
            let mut inner = state.inner.lock().expect("lock state");
            inner.config.max_scrollback_lines_per_session = 2;
            let entry = inner
                .sessions
                .get_mut(&session_id)
                .expect("session entry exists");
            entry.session.persist_history = true;
            entry.session.status = SessionStatus::Running;
        }

        state.apply_session_event(SessionEvent::Output {
            session_id: session_id.clone(),
            generation: 0,
            chunk: "l1\n".to_string(),
            ts: 1,
        });
        state.apply_session_event(SessionEvent::Output {
            session_id: session_id.clone(),
            generation: 0,
            chunk: "l2\n".to_string(),
            ts: 2,
        });
        state.apply_session_event(SessionEvent::Output {
            session_id: session_id.clone(),
            generation: 0,
            chunk: "l3\n".to_string(),
            ts: 3,
        });

        let inner = state.inner.lock().expect("lock state");
        let snapshot = inner
            .store
            .session_snapshot(&session_id, 100)
            .expect("load session snapshot");
        assert_eq!(snapshot.seq, 3);
        assert_eq!(snapshot.content, "l2\nl3\n");
    }

    #[test]
    fn normalize_relative_path_rejects_parent_component() {
        let err = normalize_relative_path("../etc/passwd").expect_err("parent path must fail");
        assert!(err.contains("parent path"));
    }

    #[test]
    fn normalize_relative_path_strips_curdir_and_windows_separators() {
        let normalized = normalize_relative_path("./src\\main.rs").expect("normalize path");
        assert_eq!(normalized, "src/main.rs");
    }

    #[cfg(unix)]
    #[test]
    fn resolve_explorer_target_handles_symlink_alias_and_blocks_escape() {
        use std::os::unix::fs::symlink;

        let base = std::env::temp_dir().join(format!(
            "chatminald-explorer-symlink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        let root = base.join("root");
        let nested = root.join("nested");
        let outside = base.join("outside");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&nested).expect("create nested root dir");
        std::fs::create_dir_all(&outside).expect("create outside dir");
        std::fs::write(nested.join("inside.txt"), "ok").expect("write inside file");

        symlink(&nested, root.join("alias")).expect("create alias symlink inside root");
        symlink(&outside, root.join("escape")).expect("create escape symlink");

        let resolved_alias = resolve_explorer_target(&root, "alias/inside.txt")
            .expect("alias path inside root should resolve");
        assert!(resolved_alias.starts_with(&root));

        let escape_err = resolve_explorer_target(&root, "escape")
            .expect_err("symlink escape must be rejected");
        assert!(escape_err.contains("escapes selected root"));

        let _ = std::fs::remove_dir_all(&base);
    }
}
