use std::fs::File;
use std::io::{Read, Write};
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc as std_mpsc,
};
use std::thread;
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use std::{ffi::CStr, mem};

use indexmap::IndexMap;
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::{AppConfig, UserSettings};
use crate::models::{
    ActivateSessionPayload, CreateProfilePayload, CreateSessionPayload, CreateSessionResponse,
    DeleteProfilePayload, LifecyclePreferences, ProfileInfo, PtyErrorEvent, PtyExitedEvent,
    PtyOutputEvent, RenameProfilePayload, RenameSessionPayload, ResizeSessionPayload,
    RuntimeUiSettings, SessionActionPayload, SessionExplorerEntry, SessionExplorerFileContent,
    SessionExplorerFsChangedEvent, SessionExplorerListPayload, SessionExplorerReadFilePayload,
    SessionExplorerRootPayload, SessionExplorerState, SessionExplorerUpdateStatePayload,
    SessionInfo, SessionSnapshot, SessionStatus, SetLifecyclePreferencesPayload,
    SetSessionPersistPayload, SwitchProfilePayload, WorkspaceState, WriteInputPayload,
};
use crate::persistence::{
    HistoryChunk, PersistedProfile, PersistedSessionExplorerState, PersistedWorkspace, Persistence,
    SessionRecord, now_ts_millis,
};

const MAX_INPUT_BYTES: usize = 65_536;
const INPUT_QUEUE_SIZE: usize = 128;
const MAX_SNAPSHOT_BYTES: usize = 512 * 1024;
const MAX_SESSION_NAME_CHARS: usize = 120;
const HISTORY_FLUSH_INTERVAL: Duration = Duration::from_millis(50);
const HISTORY_BATCH_SIZE: usize = 128;
const CWD_SYNC_INTERVAL: Duration = Duration::from_millis(500);
const MAX_EXPLORER_FILE_PREVIEW_BYTES: usize = 512 * 1024;
const MAX_EXPLORER_ENTRIES_PER_DIR: usize = 2_000;
const EXPLORER_WATCH_DEBOUNCE: Duration = Duration::from_millis(180);
const EXPLORER_WATCH_MAX_CHANGED_PATHS: usize = 128;
const KEEP_ALIVE_ON_CLOSE_KEY: &str = "keep_alive_on_close";
const START_IN_TRAY_KEY: &str = "start_in_tray";

struct SessionShared {
    output: String,
    seq: u64,
    status: SessionStatus,
    exited_emitted: bool,
    persist_history: bool,
}

struct SessionRuntime {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    input_tx: mpsc::Sender<Vec<u8>>,
    reader_handle: Option<std::thread::JoinHandle<()>>,
    writer_handle: Option<std::thread::JoinHandle<()>>,
}

struct Session {
    id: Uuid,
    profile_id: String,
    name: String,
    cwd: String,
    shell: String,
    shared: Arc<Mutex<SessionShared>>,
    runtime: Option<SessionRuntime>,
}

struct CleanupMessage {
    session_id: Uuid,
    reason: String,
    exit_code: Option<i32>,
}

struct HistoryWriteRequest {
    chunk: HistoryChunk,
}

struct ExplorerWatchRequest {
    session_id: Uuid,
    root: PathBuf,
    root_path: String,
    revision: u64,
}

enum ExplorerWatchControl {
    Set(ExplorerWatchRequest),
    Stop,
    Shutdown,
}

pub struct PtyService {
    app_handle: AppHandle,
    configured_shell: Option<String>,
    settings: UserSettings,
    persistence: Option<Persistence>,
    sessions: Arc<Mutex<IndexMap<Uuid, Session>>>,
    next_session_num: Mutex<usize>,
    active_profile_id: Mutex<Option<String>>,
    active_session_id: Mutex<Option<Uuid>>,
    cleanup_tx: std_mpsc::Sender<CleanupMessage>,
    history_tx: std_mpsc::Sender<HistoryWriteRequest>,
    explorer_watch_tx: std_mpsc::Sender<ExplorerWatchControl>,
    explorer_watch_revision: AtomicU64,
}

impl PtyService {
    pub fn new(app_handle: AppHandle, config: AppConfig) -> Self {
        let sessions = Arc::new(Mutex::new(IndexMap::new()));
        let persistence = match Persistence::initialize() {
            Ok(db) => Some(db),
            Err(err) => {
                log::warn!("persistence disabled: {err}");
                None
            }
        };

        let (cleanup_tx, cleanup_rx) = std_mpsc::channel::<CleanupMessage>();
        let (history_tx, history_rx) = std_mpsc::channel::<HistoryWriteRequest>();
        let (explorer_watch_tx, explorer_watch_rx) = std_mpsc::channel::<ExplorerWatchControl>();

        let service = Self {
            app_handle: app_handle.clone(),
            configured_shell: config.shell.clone(),
            settings: config.settings.clone(),
            persistence: persistence.clone(),
            sessions: sessions.clone(),
            next_session_num: Mutex::new(1),
            active_profile_id: Mutex::new(None),
            active_session_id: Mutex::new(None),
            cleanup_tx,
            history_tx,
            explorer_watch_tx,
            explorer_watch_revision: AtomicU64::new(0),
        };

        spawn_cleanup_worker(
            sessions,
            app_handle.clone(),
            cleanup_rx,
            persistence.clone(),
        );
        spawn_history_writer(history_rx, persistence, config.settings.clone());
        spawn_cwd_sync_worker(service.sessions.clone(), service.persistence.clone());
        spawn_explorer_watch_worker(app_handle, explorer_watch_rx);
        service.restore_from_persistence();
        service.refresh_explorer_watch_target();

        service
    }

    pub fn load_workspace(&self) -> WorkspaceState {
        WorkspaceState {
            profiles: self.list_profiles(),
            active_profile_id: self
                .active_profile_id
                .lock()
                .ok()
                .and_then(|value| value.clone()),
            sessions: self.list_sessions(),
            active_session_id: self
                .active_session_id
                .lock()
                .ok()
                .and_then(|value| value.map(|id| id.to_string())),
        }
    }

    pub fn runtime_ui_settings(&self) -> RuntimeUiSettings {
        RuntimeUiSettings {
            sync_clear_command_to_history: self.settings.sync_clear_command_to_history,
        }
    }

    pub fn list_profiles(&self) -> Vec<ProfileInfo> {
        let Some(persistence) = &self.persistence else {
            let active_profile_id = self
                .active_profile_id
                .lock()
                .ok()
                .and_then(|value| value.clone());
            return active_profile_id
                .map(|profile_id| {
                    vec![ProfileInfo {
                        profile_id,
                        name: "Default".to_string(),
                    }]
                })
                .unwrap_or_default();
        };

        match persistence.list_profiles() {
            Ok(profiles) => profiles.iter().map(profile_to_info).collect(),
            Err(err) => {
                log::warn!("list profiles failed: {err}");
                Vec::new()
            }
        }
    }

    pub fn get_lifecycle_preferences(&self) -> Result<LifecyclePreferences, String> {
        let Some(persistence) = &self.persistence else {
            return Ok(LifecyclePreferences::default());
        };

        Ok(LifecyclePreferences {
            keep_alive_on_close: persistence.get_bool_state(
                KEEP_ALIVE_ON_CLOSE_KEY,
                LifecyclePreferences::default().keep_alive_on_close,
            )?,
            start_in_tray: persistence.get_bool_state(
                START_IN_TRAY_KEY,
                LifecyclePreferences::default().start_in_tray,
            )?,
        })
    }

    pub fn set_lifecycle_preferences(
        &self,
        payload: SetLifecyclePreferencesPayload,
    ) -> Result<LifecyclePreferences, String> {
        let mut current = self.get_lifecycle_preferences()?;

        if let Some(next) = payload.keep_alive_on_close {
            current.keep_alive_on_close = next;
        }
        if let Some(next) = payload.start_in_tray {
            current.start_in_tray = next;
        }

        let Some(persistence) = &self.persistence else {
            return Ok(current);
        };

        persistence.set_bool_state(KEEP_ALIVE_ON_CLOSE_KEY, current.keep_alive_on_close)?;
        persistence.set_bool_state(START_IN_TRAY_KEY, current.start_in_tray)?;

        Ok(current)
    }

    pub fn create_profile(&self, payload: CreateProfilePayload) -> Result<ProfileInfo, String> {
        let Some(persistence) = &self.persistence else {
            return Err("profile persistence is disabled".to_string());
        };

        let resolved_name = if let Some(name) = payload.name {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                let count = persistence.list_profiles()?.len() + 1;
                format!("Profile {count}")
            } else {
                trimmed.to_string()
            }
        } else {
            let count = persistence.list_profiles()?.len() + 1;
            format!("Profile {count}")
        };

        let created = persistence.create_profile(&resolved_name)?;
        Ok(profile_to_info(&created))
    }

    pub fn switch_profile(&self, payload: SwitchProfilePayload) -> Result<WorkspaceState, String> {
        let Some(persistence) = &self.persistence else {
            return Err("profile persistence is disabled".to_string());
        };

        let profile_exists = persistence
            .list_profiles()?
            .iter()
            .any(|profile| profile.id == payload.profile_id);
        if !profile_exists {
            return Err("profile not found".to_string());
        }

        self.set_active_profile(Some(payload.profile_id.clone()));
        hydrate_sessions_from_workspace(
            self,
            persistence.load_workspace(self.settings.preview_lines)?,
        );

        Ok(self.load_workspace())
    }

    pub fn rename_profile(&self, payload: RenameProfilePayload) -> Result<ProfileInfo, String> {
        let Some(persistence) = &self.persistence else {
            return Err("profile persistence is disabled".to_string());
        };

        persistence.rename_profile(&payload.profile_id, &payload.name)?;

        let profile = persistence
            .list_profiles()?
            .into_iter()
            .find(|value| value.id == payload.profile_id)
            .ok_or_else(|| "profile not found".to_string())?;

        Ok(profile_to_info(&profile))
    }

    pub fn delete_profile(&self, payload: DeleteProfilePayload) -> Result<WorkspaceState, String> {
        let Some(persistence) = &self.persistence else {
            return Err("profile persistence is disabled".to_string());
        };

        let profile_id = payload.profile_id.trim();
        if profile_id.is_empty() {
            return Err("profile not found".to_string());
        }

        let removed = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "sessions lock poisoned".to_string())?;

            let targets: Vec<Uuid> = sessions
                .iter()
                .filter_map(|(id, session)| {
                    if session.profile_id.as_str() == profile_id {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            let mut removed = Vec::with_capacity(targets.len());
            for session_id in targets {
                if let Some(mut session) = sessions.shift_remove(&session_id) {
                    removed.push((session.id, session.shared.clone(), session.runtime.take()));
                }
            }
            removed
        };

        for (session_id, shared, runtime) in removed {
            emit_exited_once(&self.app_handle, session_id, &shared, "killed", Some(0));
            if let Some(runtime) = runtime {
                close_runtime(runtime);
            }
        }

        persistence.delete_profile(profile_id)?;

        hydrate_sessions_from_workspace(
            self,
            persistence.load_workspace(self.settings.preview_lines)?,
        );
        Ok(self.load_workspace())
    }

    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let active_profile_id = self.current_profile_id();
        let sessions = self.sessions.lock().expect("sessions lock poisoned");
        sessions
            .values()
            .filter(|session| {
                active_profile_id
                    .as_ref()
                    .is_none_or(|profile_id| &session.profile_id == profile_id)
            })
            .map(session_to_info)
            .collect()
    }

    pub fn create_session(
        &self,
        payload: CreateSessionPayload,
    ) -> Result<CreateSessionResponse, String> {
        let profile_id = self.ensure_active_profile_id()?;
        let shell = self.resolve_shell_path(self.configured_shell.as_deref())?;
        let cols = payload.cols.max(10).min(u16::MAX as usize);
        let rows = payload.rows.max(5).min(u16::MAX as usize);
        let cwd = resolve_cwd(payload.cwd.as_deref())?;
        let persist_history = payload
            .persist_history
            .unwrap_or(self.settings.persist_scrollback_enabled);

        let name = match payload.name {
            Some(name) => self.validate_session_name(&name)?,
            None => {
                let mut n = self
                    .next_session_num
                    .lock()
                    .map_err(|_| "next_session_num lock poisoned".to_string())?;
                let candidate = format!("Session {}", *n);
                *n += 1;
                candidate
            }
        };

        let session_id = Uuid::new_v4();
        let shared = Arc::new(Mutex::new(SessionShared {
            output: String::new(),
            seq: 0,
            status: SessionStatus::Running,
            exited_emitted: false,
            persist_history,
        }));

        let runtime = spawn_runtime(
            session_id,
            &shell,
            &cwd,
            cols,
            rows,
            shared.clone(),
            self.app_handle.clone(),
            self.cleanup_tx.clone(),
            self.history_tx.clone(),
        )?;

        let session = Session {
            id: session_id,
            profile_id,
            name: name.clone(),
            cwd: cwd.clone(),
            shell: shell.clone(),
            shared,
            runtime: Some(runtime),
        };

        self.sessions
            .lock()
            .map_err(|_| "sessions lock poisoned".to_string())?
            .insert(session_id, session);

        self.set_active_session(Some(session_id));
        self.persist_session_state(session_id)?;

        Ok(CreateSessionResponse {
            session_id: session_id.to_string(),
            name,
        })
    }

    pub fn activate_session(&self, payload: ActivateSessionPayload) -> Result<(), String> {
        let session_id = parse_session_id(&payload.session_id)?;
        let cols = payload.cols.max(10).min(u16::MAX as usize);
        let rows = payload.rows.max(5).min(u16::MAX as usize);

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "sessions lock poisoned".to_string())?;
        let Some(session) = sessions.get_mut(&session_id) else {
            return Err("session not found".to_string());
        };

        if let Some(runtime) = session.runtime.as_mut() {
            runtime
                .master
                .resize(Self::pty_size(cols, rows))
                .map_err(|err| format!("resize failed: {err}"))?;
        } else {
            let runtime = spawn_runtime(
                session_id,
                &session.shell,
                &session.cwd,
                cols,
                rows,
                session.shared.clone(),
                self.app_handle.clone(),
                self.cleanup_tx.clone(),
                self.history_tx.clone(),
            )?;
            session.runtime = Some(runtime);

            if let Ok(mut shared) = session.shared.lock() {
                shared.status = SessionStatus::Running;
                shared.exited_emitted = false;
            }
        }

        drop(sessions);
        self.set_active_session(Some(session_id));
        self.persist_session_state(session_id)?;
        Ok(())
    }

    pub fn write_input(&self, payload: WriteInputPayload) -> Result<(), String> {
        let session_id = parse_session_id(&payload.session_id)?;

        if payload.data.len() > MAX_INPUT_BYTES {
            return Err(format!(
                "input exceeds maximum size: {} bytes",
                MAX_INPUT_BYTES
            ));
        }

        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "sessions lock poisoned".to_string())?;
        let Some(session) = sessions.get(&session_id) else {
            return Err("session not found".to_string());
        };
        let Some(runtime) = session.runtime.as_ref() else {
            return Err("session is disconnected".to_string());
        };

        runtime
            .input_tx
            .try_send(payload.data.into_bytes())
            .map_err(|err| format!("input queue send failed: {err}"))
    }

    pub fn resize_session(&self, payload: ResizeSessionPayload) -> Result<(), String> {
        let session_id = parse_session_id(&payload.session_id)?;
        let cols = payload.cols.max(10).min(u16::MAX as usize);
        let rows = payload.rows.max(5).min(u16::MAX as usize);

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "sessions lock poisoned".to_string())?;
        let Some(session) = sessions.get_mut(&session_id) else {
            return Err("session not found".to_string());
        };
        let Some(runtime) = session.runtime.as_mut() else {
            return Err("session is disconnected".to_string());
        };

        runtime
            .master
            .resize(Self::pty_size(cols, rows))
            .map_err(|err| format!("resize failed: {err}"))
    }

    pub fn rename_session(&self, payload: RenameSessionPayload) -> Result<(), String> {
        let session_id = parse_session_id(&payload.session_id)?;
        let next_name = self.validate_session_name(&payload.name)?;

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "sessions lock poisoned".to_string())?;
        let Some(session) = sessions.get_mut(&session_id) else {
            return Err("session not found".to_string());
        };
        session.name = next_name;
        drop(sessions);

        self.persist_session_state(session_id)
    }

    pub fn set_session_persist(&self, payload: SetSessionPersistPayload) -> Result<(), String> {
        let session_id = parse_session_id(&payload.session_id)?;
        let mut backfill_candidate: Option<(u64, String)> = None;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "sessions lock poisoned".to_string())?;
        let Some(session) = sessions.get_mut(&session_id) else {
            return Err("session not found".to_string());
        };

        if let Ok(mut shared) = session.shared.lock() {
            let was_persisting = shared.persist_history;
            shared.persist_history = payload.persist_history;
            if payload.persist_history
                && !was_persisting
                && shared.seq > 0
                && !shared.output.is_empty()
            {
                backfill_candidate = Some((shared.seq, shared.output.clone()));
            }
        }
        drop(sessions);

        if let Some((seq, snapshot_content)) = backfill_candidate
            && let Some(persistence) = &self.persistence
        {
            let session_id_text = session_id.to_string();
            let latest_seq = persistence.latest_persisted_seq(&session_id_text)?;
            if latest_seq.unwrap_or(0) < seq {
                persistence.append_history_batch(
                    &[HistoryChunk {
                        session_id: session_id_text,
                        seq,
                        line_count: snapshot_content
                            .chars()
                            .filter(|c| *c == '\n')
                            .count()
                            .max(1) as u64,
                        chunk_text: snapshot_content,
                        ts: now_ts_millis(),
                    }],
                    self.settings.max_lines_per_session,
                    self.settings.auto_delete_after_days,
                )?;
            }
        }

        self.persist_session_state(session_id)
    }

    pub fn close_session(&self, payload: SessionActionPayload) -> Result<(), String> {
        let session_id = parse_session_id(&payload.session_id)?;

        let current_active = self
            .active_session_id
            .lock()
            .ok()
            .and_then(|active| *active);

        let (removed_session_id, removed_profile_id, removed_shared, removed_runtime, next_active) = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "sessions lock poisoned".to_string())?;
            let Some(mut session) = sessions.shift_remove(&session_id) else {
                return Err("session not found".to_string());
            };

            let next_active = if current_active == Some(session_id) {
                sessions
                    .values()
                    .find(|candidate| candidate.profile_id == session.profile_id)
                    .map(|candidate| candidate.id)
            } else {
                current_active
            };

            (
                session.id,
                session.profile_id.clone(),
                session.shared.clone(),
                session.runtime.take(),
                next_active,
            )
        };

        emit_exited_once(
            &self.app_handle,
            removed_session_id,
            &removed_shared,
            "killed",
            Some(0),
        );
        if let Some(runtime) = removed_runtime {
            close_runtime(runtime);
        }

        if self.current_profile_id().as_deref() == Some(removed_profile_id.as_str()) {
            self.set_active_session(next_active);
        }
        if let Some(persistence) = &self.persistence {
            let _ = persistence.delete_session(&session_id.to_string());
        }

        Ok(())
    }

    pub fn clear_session_history(&self, payload: SessionActionPayload) -> Result<(), String> {
        let session_id = parse_session_id(&payload.session_id)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "sessions lock poisoned".to_string())?;
        let Some(session) = sessions.get_mut(&session_id) else {
            return Err("session not found".to_string());
        };
        let redraw_tx = session
            .runtime
            .as_ref()
            .map(|runtime| runtime.input_tx.clone());

        if let Ok(mut shared) = session.shared.lock() {
            shared.output.clear();
            shared.seq = 0;
        }
        drop(sessions);

        if let Some(persistence) = &self.persistence {
            persistence.clear_session_history(&session_id.to_string())?;
        }

        if let Some(tx) = redraw_tx {
            let _ = tx.try_send(vec![0x0c]);
        }

        Ok(())
    }

    pub fn clear_all_history(&self) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "sessions lock poisoned".to_string())?;
        let mut redraw_txs = Vec::new();
        for session in sessions.values_mut() {
            if let Ok(mut shared) = session.shared.lock() {
                shared.output.clear();
                shared.seq = 0;
            }
            if let Some(runtime) = session.runtime.as_ref() {
                redraw_txs.push(runtime.input_tx.clone());
            }
        }
        drop(sessions);

        if let Some(persistence) = &self.persistence {
            persistence.clear_all_history()?;
        }

        for tx in redraw_txs {
            let _ = tx.try_send(vec![0x0c]);
        }
        Ok(())
    }

    pub fn shutdown_graceful(&self) -> Result<(), String> {
        let removed = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "sessions lock poisoned".to_string())?;
            let targets: Vec<Uuid> = sessions.keys().copied().collect();
            let mut removed = Vec::with_capacity(targets.len());

            for session_id in targets {
                if let Some(session) = sessions.shift_remove(&session_id) {
                    removed.push(session);
                }
            }
            removed
        };

        for mut session in removed {
            emit_exited_once(
                &self.app_handle,
                session.id,
                &session.shared,
                "killed",
                Some(0),
            );
            if let Some(runtime) = session.runtime.take() {
                close_runtime(runtime);
            }

            if let Some(persistence) = &self.persistence {
                let _ = persistence
                    .set_session_status(&session.id.to_string(), SessionStatus::Disconnected);
                if let Ok(record) = session_to_record(&session) {
                    let _ = persistence.upsert_session(&record);
                }
            }
        }

        self.set_active_session(None);
        let _ = self.explorer_watch_tx.send(ExplorerWatchControl::Shutdown);
        Ok(())
    }

    pub fn get_session_snapshot(
        &self,
        payload: SessionActionPayload,
    ) -> Result<SessionSnapshot, String> {
        let session_id = parse_session_id(&payload.session_id)?;
        let (shared, has_runtime) = {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "sessions lock poisoned".to_string())?;
            let Some(session) = sessions.get(&session_id) else {
                return Err("session not found".to_string());
            };
            (session.shared.clone(), session.runtime.is_some())
        };

        let (mut content, seq, persist_history) = shared
            .lock()
            .map(|state| (state.output.clone(), state.seq, state.persist_history))
            .map_err(|_| "shared lock poisoned".to_string())?;

        if !has_runtime && persist_history {
            let requested_lines = payload.preview_lines.unwrap_or(self.settings.preview_lines);
            let normalized_lines = requested_lines.clamp(10, 50_000);
            if let Some(persistence) = &self.persistence {
                let persisted_preview =
                    persistence.load_session_preview(&session_id.to_string(), normalized_lines)?;
                if !persisted_preview.is_empty() {
                    content = persisted_preview;
                }
            }
        }

        Ok(SessionSnapshot { content, seq })
    }

    pub fn get_session_explorer_state(
        &self,
        payload: SessionActionPayload,
    ) -> Result<SessionExplorerState, String> {
        let session_id = parse_session_id(&payload.session_id)?;
        if !self.session_exists(session_id) {
            return Err("session not found".to_string());
        }

        let Some(persistence) = &self.persistence else {
            return Err("file explorer persistence is disabled".to_string());
        };

        let state = persistence.get_session_explorer_state(&session_id.to_string())?;
        Ok(explorer_state_to_model(session_id, state))
    }

    pub fn set_session_explorer_root(
        &self,
        payload: SessionExplorerRootPayload,
    ) -> Result<SessionExplorerState, String> {
        let session_id = parse_session_id(&payload.session_id)?;
        if !self.session_exists(session_id) {
            return Err("session not found".to_string());
        }

        let Some(persistence) = &self.persistence else {
            return Err("file explorer persistence is disabled".to_string());
        };

        let root_path = resolve_explorer_root_path(&payload.root_path)?
            .to_string_lossy()
            .to_string();
        let persisted =
            persistence.set_session_explorer_root(&session_id.to_string(), &root_path)?;
        self.refresh_explorer_watch_target();
        Ok(explorer_state_to_model(session_id, Some(persisted)))
    }

    pub fn update_session_explorer_state(
        &self,
        payload: SessionExplorerUpdateStatePayload,
    ) -> Result<SessionExplorerState, String> {
        let session_id = parse_session_id(&payload.session_id)?;
        if !self.session_exists(session_id) {
            return Err("session not found".to_string());
        }

        let Some(persistence) = &self.persistence else {
            return Err("file explorer persistence is disabled".to_string());
        };

        let session_id_text = session_id.to_string();
        let Some(current_state) = persistence.get_session_explorer_state(&session_id_text)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&current_state.root_path)?;
        let current_dir = normalize_relative_path(&payload.current_dir)?;
        let current_dir_path = resolve_explorer_target(&root, &current_dir)?;
        if !current_dir_path.is_dir() {
            return Err("current explorer directory is not valid".to_string());
        }

        let selected_path = match payload.selected_path.as_deref() {
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

        let open_file_path = match payload.open_file_path.as_deref() {
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

        let persisted = persistence.update_session_explorer_state(
            &session_id_text,
            &current_dir,
            selected_path.as_deref(),
            open_file_path.as_deref(),
        )?;
        Ok(explorer_state_to_model(session_id, Some(persisted)))
    }

    pub fn list_session_explorer_entries(
        &self,
        payload: SessionExplorerListPayload,
    ) -> Result<Vec<SessionExplorerEntry>, String> {
        let session_id = parse_session_id(&payload.session_id)?;
        if !self.session_exists(session_id) {
            return Err("session not found".to_string());
        }

        let Some(persistence) = &self.persistence else {
            return Err("file explorer persistence is disabled".to_string());
        };

        let session_id_text = session_id.to_string();
        let Some(state) = persistence.get_session_explorer_state(&session_id_text)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&state.root_path)?;
        let relative_path = match payload.relative_path.as_deref() {
            Some(value) => normalize_relative_path(value)?,
            None => state.current_dir,
        };

        let target_dir = resolve_explorer_target(&root, &relative_path)?;
        if !target_dir.is_dir() {
            return Err("explorer target is not a directory".to_string());
        }

        let mut entries = Vec::new();
        let read_dir = std::fs::read_dir(&target_dir)
            .map_err(|err| format!("read explorer directory failed: {err}"))?;

        for item in read_dir {
            if entries.len() >= MAX_EXPLORER_ENTRIES_PER_DIR {
                break;
            }

            let entry = match item {
                Ok(value) => value,
                Err(_) => continue,
            };

            let canonical = match std::fs::canonicalize(entry.path()) {
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
            let relative = match canonical.strip_prefix(&root) {
                Ok(path) => normalize_relative_path(&path.to_string_lossy())?,
                Err(_) => continue,
            };
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

    pub fn read_session_explorer_file(
        &self,
        payload: SessionExplorerReadFilePayload,
    ) -> Result<SessionExplorerFileContent, String> {
        let session_id = parse_session_id(&payload.session_id)?;
        if !self.session_exists(session_id) {
            return Err("session not found".to_string());
        }

        let Some(persistence) = &self.persistence else {
            return Err("file explorer persistence is disabled".to_string());
        };

        let session_id_text = session_id.to_string();
        let Some(state) = persistence.get_session_explorer_state(&session_id_text)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&state.root_path)?;
        let relative_path = normalize_relative_path(&payload.relative_path)?;
        let target = resolve_explorer_target(&root, &relative_path)?;
        if !target.is_file() {
            return Err("explorer target is not a file".to_string());
        }

        let max_bytes = payload
            .max_bytes
            .unwrap_or(256 * 1024)
            .clamp(1_024, MAX_EXPLORER_FILE_PREVIEW_BYTES);

        let file =
            File::open(&target).map_err(|err| format!("open explorer file failed: {err}"))?;
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

        let content = String::from_utf8_lossy(&buffer).to_string();

        Ok(SessionExplorerFileContent {
            relative_path,
            content,
            truncated,
            byte_len: buffer.len(),
        })
    }

    fn pty_size(cols: usize, rows: usize) -> PtySize {
        PtySize {
            cols: cols.max(1).min(u16::MAX as usize) as u16,
            rows: rows.max(1).min(u16::MAX as usize) as u16,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    fn session_exists(&self, session_id: Uuid) -> bool {
        self.sessions
            .lock()
            .map(|sessions| sessions.contains_key(&session_id))
            .unwrap_or(false)
    }

    fn resolve_shell_path(&self, config_shell: Option<&str>) -> Result<String, String> {
        let mut candidates = Vec::new();

        if let Some(shell) = config_shell {
            candidates.push(shell.to_string());
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(shell) = std::env::var("SHELL") {
                candidates.push(shell);
            }

            candidates.push("/bin/zsh".to_string());
            candidates.push("/bin/bash".to_string());
            candidates.push("/bin/sh".to_string());
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(shell) = std::env::var("COMSPEC") {
                candidates.push(shell);
            }
            candidates.push("pwsh.exe".to_string());
            candidates.push("powershell.exe".to_string());
            candidates.push("cmd.exe".to_string());
        }

        for candidate in candidates {
            if self.validate_shell_path(&candidate).is_ok() {
                return Ok(candidate);
            }
        }

        Err("no valid shell found in config, env, or defaults".to_string())
    }

    fn validate_shell_path(&self, raw_path: &str) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            let normalized = raw_path.trim().trim_matches('"');
            if normalized.is_empty() {
                return Err("shell path is empty".to_string());
            }
            if Self::resolve_windows_shell_candidate(normalized).is_none() {
                return Err(format!("shell is not resolvable: {normalized}"));
            }
            return Ok(());
        }

        #[cfg(not(target_os = "windows"))]
        {
            let raw = PathBuf::from(raw_path);

            std::fs::symlink_metadata(&raw).map_err(|err| format!("invalid shell path: {err}"))?;

            let canonical = std::fs::canonicalize(&raw)
                .map_err(|err| format!("cannot canonicalize shell: {err}"))?;

            if !self.is_allowed_shell(&raw, &canonical)? {
                return Err(format!("shell is not in /etc/shells: {}", raw.display()));
            }

            let meta =
                std::fs::metadata(&canonical).map_err(|err| format!("cannot stat shell: {err}"))?;

            if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
                return Err("shell is not executable".to_string());
            }

            Ok(())
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn is_allowed_shell(&self, raw: &Path, canonical: &Path) -> Result<bool, String> {
        let shells = std::fs::read_to_string("/etc/shells")
            .map_err(|err| format!("cannot read /etc/shells: {err}"))?;

        for line in shells.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let candidate = PathBuf::from(line);
            if candidate == raw {
                return Ok(true);
            }

            if let Ok(candidate_canonical) = std::fs::canonicalize(&candidate)
                && candidate_canonical == canonical
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[cfg(target_os = "windows")]
    fn validate_windows_path_candidate(path: &Path) -> bool {
        std::fs::metadata(path)
            .map(|meta| meta.is_file())
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    fn windows_pathexts() -> Vec<String> {
        let default = vec![
            ".com".to_string(),
            ".exe".to_string(),
            ".bat".to_string(),
            ".cmd".to_string(),
        ];

        let Some(raw) = std::env::var_os("PATHEXT") else {
            return default;
        };

        let value = raw.to_string_lossy();
        let mut parsed = Vec::new();
        for token in value.split(';') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if token.starts_with('.') {
                parsed.push(token.to_ascii_lowercase());
            } else {
                parsed.push(format!(".{}", token.to_ascii_lowercase()));
            }
        }

        if parsed.is_empty() { default } else { parsed }
    }

    #[cfg(target_os = "windows")]
    fn resolve_windows_shell_candidate(raw: &str) -> Option<PathBuf> {
        let candidate = PathBuf::from(raw);
        if candidate.components().count() > 1 || candidate.is_absolute() {
            let canonical = std::fs::canonicalize(candidate).ok()?;
            if Self::validate_windows_path_candidate(&canonical) {
                return Some(canonical);
            }
            return None;
        }

        let path_var = std::env::var_os("PATH")?;
        let has_extension = Path::new(raw).extension().is_some();
        let pathexts = Self::windows_pathexts();

        for dir in std::env::split_paths(&path_var) {
            if has_extension {
                let full = dir.join(raw);
                if Self::validate_windows_path_candidate(&full) {
                    return Some(full);
                }
                continue;
            }

            for ext in &pathexts {
                let full = dir.join(format!("{raw}{ext}"));
                if Self::validate_windows_path_candidate(&full) {
                    return Some(full);
                }
            }
        }

        None
    }

    fn validate_session_name(&self, raw_name: &str) -> Result<String, String> {
        let normalized = raw_name.trim();
        if normalized.is_empty() {
            return Err("session name cannot be empty".to_string());
        }

        if normalized.chars().count() > MAX_SESSION_NAME_CHARS {
            return Err(format!(
                "session name is too long (max {} chars)",
                MAX_SESSION_NAME_CHARS
            ));
        }

        Ok(normalized.to_string())
    }

    fn restore_from_persistence(&self) {
        let Some(persistence) = &self.persistence else {
            return;
        };

        let workspace = match persistence.load_workspace(self.settings.preview_lines) {
            Ok(workspace) => workspace,
            Err(err) => {
                log::warn!("cannot load workspace: {err}");
                return;
            }
        };

        hydrate_sessions_from_workspace(self, workspace);
    }

    fn ensure_active_profile_id(&self) -> Result<String, String> {
        if let Some(active_profile_id) = self
            .active_profile_id
            .lock()
            .ok()
            .and_then(|value| value.clone())
        {
            return Ok(active_profile_id);
        }

        let Some(persistence) = &self.persistence else {
            let fallback = "default".to_string();
            self.set_active_profile(Some(fallback.clone()));
            return Ok(fallback);
        };

        let workspace = persistence.load_workspace(self.settings.preview_lines)?;
        let Some(active_profile_id) = workspace.active_profile_id else {
            return Err("cannot resolve active profile".to_string());
        };

        self.set_active_profile(Some(active_profile_id.clone()));
        Ok(active_profile_id)
    }

    fn current_profile_id(&self) -> Option<String> {
        self.active_profile_id
            .lock()
            .ok()
            .and_then(|value| value.clone())
    }

    fn persist_session_state(&self, session_id: Uuid) -> Result<(), String> {
        let Some(persistence) = &self.persistence else {
            return Ok(());
        };

        let record = {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "sessions lock poisoned".to_string())?;
            let Some(session) = sessions.get(&session_id) else {
                return Ok(());
            };
            session_to_record(session)?
        };

        persistence.upsert_session(&record)
    }

    fn set_active_profile(&self, profile_id: Option<String>) {
        if let Ok(mut active) = self.active_profile_id.lock() {
            *active = profile_id.clone();
        }

        if let Some(persistence) = &self.persistence
            && let Err(err) = persistence.set_active_profile(profile_id.as_deref())
        {
            log::warn!("persist active profile failed: {err}");
        }
    }

    fn set_active_session(&self, session_id: Option<Uuid>) {
        if let Ok(mut active) = self.active_session_id.lock() {
            *active = session_id;
        }

        if let Some(persistence) = &self.persistence {
            let Some(profile_id) = self.current_profile_id() else {
                return;
            };
            let id = session_id.map(|value| value.to_string());
            if let Err(err) = persistence.set_active_session(&profile_id, id.as_deref()) {
                log::warn!("persist active session failed: {err}");
            }
        }

        self.refresh_explorer_watch_target();
    }

    fn refresh_explorer_watch_target(&self) {
        let active_session_id = self
            .active_session_id
            .lock()
            .ok()
            .and_then(|active| *active);

        let Some(session_id) = active_session_id else {
            let _ = self.explorer_watch_tx.send(ExplorerWatchControl::Stop);
            return;
        };

        let Some(persistence) = &self.persistence else {
            let _ = self.explorer_watch_tx.send(ExplorerWatchControl::Stop);
            return;
        };

        let session_state = match persistence.get_session_explorer_state(&session_id.to_string()) {
            Ok(value) => value,
            Err(err) => {
                log::warn!("load explorer state for watch failed: {err}");
                let _ = self.explorer_watch_tx.send(ExplorerWatchControl::Stop);
                return;
            }
        };

        let Some(state) = session_state else {
            let _ = self.explorer_watch_tx.send(ExplorerWatchControl::Stop);
            return;
        };

        let root = match resolve_explorer_root_path(&state.root_path) {
            Ok(value) => value,
            Err(err) => {
                log::warn!("resolve explorer watch root failed: {err}");
                let _ = self.explorer_watch_tx.send(ExplorerWatchControl::Stop);
                return;
            }
        };

        let revision = self
            .explorer_watch_revision
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);

        let _ = self
            .explorer_watch_tx
            .send(ExplorerWatchControl::Set(ExplorerWatchRequest {
                session_id,
                root_path: root.to_string_lossy().to_string(),
                root,
                revision,
            }));
    }
}

fn hydrate_sessions_from_workspace(service: &PtyService, workspace: PersistedWorkspace) {
    let PersistedWorkspace {
        active_profile_id,
        sessions: persisted_sessions,
        active_session_id,
    } = workspace;
    let active_profile = active_profile_id.clone();

    service.set_active_profile(active_profile_id);

    if let Ok(mut sessions) = service.sessions.lock() {
        for persisted in persisted_sessions {
            let Ok(session_id) = Uuid::parse_str(&persisted.id) else {
                continue;
            };

            if let Some(existing) = sessions.get_mut(&session_id) {
                existing.profile_id = persisted.profile_id.clone();
                existing.name = persisted.name.clone();
                existing.cwd = persisted.cwd.clone();
                existing.shell = persisted.shell.clone();

                let is_disconnected_runtime = existing.runtime.is_none();
                if let Ok(mut shared) = existing.shared.lock() {
                    shared.persist_history = persisted.persist_history;
                    if is_disconnected_runtime {
                        shared.output = persisted.preview.clone();
                        shared.seq = persisted.last_seq;
                        shared.status = SessionStatus::Disconnected;
                        shared.exited_emitted = false;
                    }
                }

                if is_disconnected_runtime && let Some(persistence) = &service.persistence {
                    let _ = persistence
                        .set_session_status(&session_id.to_string(), SessionStatus::Disconnected);
                }
                continue;
            }

            let shared = Arc::new(Mutex::new(SessionShared {
                output: persisted.preview,
                seq: persisted.last_seq,
                status: SessionStatus::Disconnected,
                exited_emitted: false,
                persist_history: persisted.persist_history,
            }));

            sessions.insert(
                session_id,
                Session {
                    id: session_id,
                    profile_id: persisted.profile_id,
                    name: persisted.name,
                    cwd: persisted.cwd,
                    shell: persisted.shell,
                    shared,
                    runtime: None,
                },
            );

            if let Some(persistence) = &service.persistence {
                let _ = persistence
                    .set_session_status(&session_id.to_string(), SessionStatus::Disconnected);
            }
        }
    }

    let active = active_session_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .filter(|id| {
            service
                .sessions
                .lock()
                .ok()
                .and_then(|sessions| sessions.get(id).map(|session| session.profile_id.clone()))
                .is_some_and(|profile_id| {
                    active_profile
                        .as_ref()
                        .is_none_or(|value| value == &profile_id)
                })
        })
        .or_else(|| {
            service.sessions.lock().ok().and_then(|sessions| {
                sessions
                    .values()
                    .find(|session| {
                        active_profile
                            .as_ref()
                            .is_none_or(|profile_id| &session.profile_id == profile_id)
                    })
                    .map(|session| session.id)
            })
        });

    service.set_active_session(active);
}

fn session_to_info(session: &Session) -> SessionInfo {
    let (status, persist_history, seq) = session
        .shared
        .lock()
        .map(|shared| (shared.status.clone(), shared.persist_history, shared.seq))
        .unwrap_or((SessionStatus::Disconnected, false, 0));

    SessionInfo {
        session_id: session.id.to_string(),
        name: session.name.clone(),
        cwd: session.cwd.clone(),
        status,
        persist_history,
        seq,
    }
}

fn profile_to_info(profile: &PersistedProfile) -> ProfileInfo {
    ProfileInfo {
        profile_id: profile.id.clone(),
        name: profile.name.clone(),
    }
}

fn explorer_state_to_model(
    session_id: Uuid,
    state: Option<PersistedSessionExplorerState>,
) -> SessionExplorerState {
    if let Some(state) = state {
        return SessionExplorerState {
            session_id: state.session_id,
            root_path: Some(state.root_path),
            current_dir: state.current_dir,
            selected_path: state.selected_path,
            open_file_path: state.open_file_path,
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

fn session_to_record(session: &Session) -> Result<SessionRecord, String> {
    let (status, persist_history, seq) = session
        .shared
        .lock()
        .map(|shared| (shared.status.clone(), shared.persist_history, shared.seq))
        .map_err(|_| "shared lock poisoned".to_string())?;

    Ok(SessionRecord {
        id: session.id.to_string(),
        profile_id: session.profile_id.clone(),
        name: session.name.clone(),
        cwd: session.cwd.clone(),
        shell: session.shell.clone(),
        status,
        persist_history,
        last_seq: seq,
    })
}

fn parse_session_id(session_id: &str) -> Result<Uuid, String> {
    Uuid::parse_str(session_id).map_err(|err| format!("invalid session id: {err}"))
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
            std::path::Component::Normal(part) => normalized.push(part),
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

fn resolve_cwd(raw_cwd: Option<&str>) -> Result<String, String> {
    let candidate = match raw_cwd {
        Some(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
    };

    let canonical = std::fs::canonicalize(&candidate)
        .map_err(|err| format!("invalid cwd '{}': {err}", candidate.display()))?;
    if !canonical.is_dir() {
        return Err("cwd is not a directory".to_string());
    }

    Ok(canonical.to_string_lossy().to_string())
}

fn read_process_cwd(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let link = PathBuf::from(format!("/proc/{pid}/cwd"));
        let path = std::fs::read_link(link).ok()?;
        return Some(path.to_string_lossy().to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let mut info: libc::proc_vnodepathinfo = unsafe { mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
        let bytes = unsafe {
            libc::proc_pidinfo(
                pid as libc::c_int,
                libc::PROC_PIDVNODEPATHINFO,
                0,
                (&mut info as *mut libc::proc_vnodepathinfo).cast(),
                size,
            )
        };

        if bytes != size {
            return None;
        }

        let path_ptr = info.pvi_cdir.vip_path.as_ptr() as *const libc::c_char;
        let path = unsafe { CStr::from_ptr(path_ptr) }
            .to_string_lossy()
            .to_string();
        if path.is_empty() {
            return None;
        }
        return Some(path);
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

fn spawn_runtime(
    session_id: Uuid,
    shell: &str,
    cwd: &str,
    cols: usize,
    rows: usize,
    shared: Arc<Mutex<SessionShared>>,
    app_handle: AppHandle,
    cleanup_tx: std_mpsc::Sender<CleanupMessage>,
    history_tx: std_mpsc::Sender<HistoryWriteRequest>,
) -> Result<SessionRuntime, String> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtyService::pty_size(cols, rows))
        .map_err(|err| format!("openpty failed: {err}"))?;

    let mut command = CommandBuilder::new(shell);
    command.env("TERM", "xterm-256color");
    command.cwd(cwd);

    let child = pty_pair
        .slave
        .spawn_command(command)
        .map_err(|err| format!("spawn shell failed: {err}"))?;

    let reader = pty_pair
        .master
        .try_clone_reader()
        .map_err(|err| format!("clone reader failed: {err}"))?;
    let writer = pty_pair
        .master
        .take_writer()
        .map_err(|err| format!("take writer failed: {err}"))?;

    let (input_tx, input_rx) = mpsc::channel::<Vec<u8>>(INPUT_QUEUE_SIZE);

    let reader_handle = thread::Builder::new()
        .name(format!("chatminal-reader-{session_id}"))
        .spawn(move || {
            reader_thread(
                reader, app_handle, session_id, shared, cleanup_tx, history_tx,
            );
        })
        .map_err(|err| format!("spawn reader thread failed: {err}"))?;

    let writer_handle = thread::Builder::new()
        .name(format!("chatminal-writer-{session_id}"))
        .spawn(move || {
            writer_thread(writer, input_rx);
        })
        .map_err(|err| format!("spawn writer thread failed: {err}"))?;

    Ok(SessionRuntime {
        child,
        master: pty_pair.master,
        input_tx,
        reader_handle: Some(reader_handle),
        writer_handle: Some(writer_handle),
    })
}

fn spawn_cleanup_worker(
    sessions: Arc<Mutex<IndexMap<Uuid, Session>>>,
    app_handle: AppHandle,
    cleanup_rx: std_mpsc::Receiver<CleanupMessage>,
    persistence: Option<Persistence>,
) {
    let _ = thread::Builder::new()
        .name("chatminal-cleanup".to_string())
        .spawn(move || {
            while let Ok(message) = cleanup_rx.recv() {
                let mut sessions_guard = match sessions.lock() {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let Some(session) = sessions_guard.get_mut(&message.session_id) else {
                    continue;
                };

                emit_exited_once(
                    &app_handle,
                    session.id,
                    &session.shared,
                    &message.reason,
                    message.exit_code,
                );

                if let Some(runtime) = session.runtime.take() {
                    close_runtime(runtime);
                }

                if let Some(db) = &persistence {
                    let _ =
                        db.set_session_status(&session.id.to_string(), SessionStatus::Disconnected);
                    if let Ok(record) = session_to_record(session) {
                        let _ = db.upsert_session(&record);
                    }
                }
            }
        });
}

fn spawn_history_writer(
    history_rx: std_mpsc::Receiver<HistoryWriteRequest>,
    persistence: Option<Persistence>,
    settings: UserSettings,
) {
    let _ = thread::Builder::new()
        .name("chatminal-history-writer".to_string())
        .spawn(move || {
            let mut buffer = Vec::<HistoryChunk>::new();

            let flush = |batch: &mut Vec<HistoryChunk>| {
                if batch.is_empty() {
                    return;
                }
                if let Some(db) = &persistence {
                    if let Err(err) = db.append_history_batch(
                        batch,
                        settings.max_lines_per_session,
                        settings.auto_delete_after_days,
                    ) {
                        log::warn!("history write batch failed: {err}");
                    }
                }
                batch.clear();
            };

            loop {
                match history_rx.recv_timeout(HISTORY_FLUSH_INTERVAL) {
                    Ok(message) => {
                        buffer.push(message.chunk);
                        if buffer.len() >= HISTORY_BATCH_SIZE {
                            flush(&mut buffer);
                        }
                    }
                    Err(std_mpsc::RecvTimeoutError::Timeout) => flush(&mut buffer),
                    Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                        flush(&mut buffer);
                        break;
                    }
                }
            }
        });
}

fn spawn_cwd_sync_worker(
    sessions: Arc<Mutex<IndexMap<Uuid, Session>>>,
    persistence: Option<Persistence>,
) {
    let _ = thread::Builder::new()
        .name("chatminal-cwd-sync".to_string())
        .spawn(move || {
            loop {
                thread::sleep(CWD_SYNC_INTERVAL);

                let session_snapshots = {
                    let sessions_guard = match sessions.lock() {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    let mut snapshots = Vec::<(Uuid, String, u32)>::new();
                    for session in sessions_guard.values() {
                        let Some(runtime) = session.runtime.as_ref() else {
                            continue;
                        };
                        let Some(pid) = runtime.child.process_id() else {
                            continue;
                        };

                        snapshots.push((session.id, session.cwd.clone(), pid));
                    }
                    snapshots
                };

                if session_snapshots.is_empty() {
                    continue;
                }

                let mut cwd_updates = Vec::<(Uuid, String)>::new();
                for (session_id, current_cwd, pid) in session_snapshots {
                    let Some(next_cwd) = read_process_cwd(pid) else {
                        continue;
                    };
                    if next_cwd != current_cwd {
                        cwd_updates.push((session_id, next_cwd));
                    }
                }

                if cwd_updates.is_empty() {
                    continue;
                }

                let changed_cwds = {
                    let mut sessions_guard = match sessions.lock() {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    let mut changed = Vec::<(String, String)>::new();
                    for (session_id, next_cwd) in cwd_updates {
                        let Some(session) = sessions_guard.get_mut(&session_id) else {
                            continue;
                        };
                        if session.cwd == next_cwd {
                            continue;
                        }

                        session.cwd = next_cwd;
                        changed.push((session.id.to_string(), session.cwd.clone()));
                    }
                    changed
                };

                if let Some(db) = &persistence {
                    for (session_id, cwd) in &changed_cwds {
                        if let Err(err) = db.set_session_cwd(session_id, cwd) {
                            log::warn!("persist cwd update failed: {err}");
                        }
                    }
                }
            }
        });
}

fn spawn_explorer_watch_worker(
    app_handle: AppHandle,
    control_rx: std_mpsc::Receiver<ExplorerWatchControl>,
) {
    let _ = thread::Builder::new()
        .name("chatminal-explorer-watch".to_string())
        .spawn(move || {
            let (watch_event_tx, watch_event_rx) =
                std_mpsc::channel::<notify::Result<notify::Event>>();
            let mut _watcher: Option<RecommendedWatcher> = None;
            let mut current_watch: Option<ExplorerWatchRequest> = None;
            let mut pending_paths = std::collections::BTreeSet::<String>::new();
            let mut pending_full_resync = false;
            let mut pending_since: Option<Instant> = None;
            let mut running = true;

            while running {
                loop {
                    match control_rx.try_recv() {
                        Ok(ExplorerWatchControl::Set(request)) => {
                            pending_paths.clear();
                            pending_full_resync = false;
                            pending_since = None;
                            _watcher = None;
                            current_watch = Some(request);

                            while watch_event_rx.try_recv().is_ok() {}

                            if let Some(target) = current_watch.as_ref() {
                                match create_explorer_watcher(
                                    target.root.as_path(),
                                    watch_event_tx.clone(),
                                ) {
                                    Ok(new_watcher) => _watcher = Some(new_watcher),
                                    Err(err) => {
                                        log::warn!("start explorer watcher failed: {err}");
                                        pending_full_resync = true;
                                        pending_since = Some(Instant::now());
                                    }
                                }
                            }
                        }
                        Ok(ExplorerWatchControl::Stop) => {
                            _watcher = None;
                            current_watch = None;
                            pending_paths.clear();
                            pending_full_resync = false;
                            pending_since = None;
                        }
                        Ok(ExplorerWatchControl::Shutdown) => {
                            running = false;
                            break;
                        }
                        Err(std_mpsc::TryRecvError::Empty) => break,
                        Err(std_mpsc::TryRecvError::Disconnected) => {
                            running = false;
                            break;
                        }
                    }
                }

                if !running {
                    break;
                }

                while let Ok(message) = watch_event_rx.try_recv() {
                    let Some(target) = current_watch.as_ref() else {
                        continue;
                    };

                    match message {
                        Ok(event) => {
                            if !should_queue_explorer_notify_event(&event.kind) {
                                continue;
                            }

                            if event.paths.is_empty() {
                                pending_full_resync = true;
                            } else {
                                for changed_path in &event.paths {
                                    if !push_explorer_changed_path(
                                        changed_path,
                                        &target.root,
                                        &mut pending_paths,
                                    ) {
                                        pending_full_resync = true;
                                    }

                                    if pending_paths.len() >= EXPLORER_WATCH_MAX_CHANGED_PATHS {
                                        pending_full_resync = true;
                                        break;
                                    }
                                }
                            }

                            pending_since.get_or_insert_with(Instant::now);
                        }
                        Err(err) => {
                            log::warn!("explorer watcher event error: {err}");
                            pending_full_resync = true;
                            pending_since.get_or_insert_with(Instant::now);
                        }
                    }
                }

                if let Some(queued_at) = pending_since
                    && queued_at.elapsed() >= EXPLORER_WATCH_DEBOUNCE
                {
                    if let Some(target) = current_watch.as_ref() {
                        let mut changed_paths = pending_paths.iter().cloned().collect::<Vec<_>>();
                        changed_paths.sort();
                        let full_resync = pending_full_resync || changed_paths.is_empty();

                        emit_explorer_fs_changed(
                            &app_handle,
                            target,
                            changed_paths,
                            full_resync,
                        );
                    }

                    pending_paths.clear();
                    pending_full_resync = false;
                    pending_since = None;
                }

                thread::sleep(Duration::from_millis(30));
            }
        });
}

fn create_explorer_watcher(
    root: &Path,
    watch_event_tx: std_mpsc::Sender<notify::Result<notify::Event>>,
) -> Result<RecommendedWatcher, String> {
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            let _ = watch_event_tx.send(event);
        },
        NotifyConfig::default(),
    )
    .map_err(|err| format!("init notify watcher failed: {err}"))?;

    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|err| format!("watch path '{}' failed: {err}", root.display()))?;

    Ok(watcher)
}

fn should_queue_explorer_notify_event(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_) | EventKind::Other)
}

fn push_explorer_changed_path(
    changed_path: &Path,
    root: &Path,
    output: &mut std::collections::BTreeSet<String>,
) -> bool {
    let relative = match changed_path.strip_prefix(root) {
        Ok(path) => path,
        Err(_) => return false,
    };

    match normalize_relative_path(&relative.to_string_lossy()) {
        Ok(value) => {
            output.insert(value);
            true
        }
        Err(_) => false,
    }
}

fn emit_explorer_fs_changed(
    app_handle: &AppHandle,
    watch: &ExplorerWatchRequest,
    changed_paths: Vec<String>,
    full_resync: bool,
) {
    let payload = SessionExplorerFsChangedEvent {
        session_id: watch.session_id.to_string(),
        root_path: watch.root_path.clone(),
        changed_paths,
        full_resync,
        revision: watch.revision,
    };
    let _ = app_handle.emit("explorer/fs-changed", payload);
}

fn reader_thread(
    mut reader: Box<dyn Read + Send>,
    app_handle: AppHandle,
    session_id: Uuid,
    shared: Arc<Mutex<SessionShared>>,
    cleanup_tx: std_mpsc::Sender<CleanupMessage>,
    history_tx: std_mpsc::Sender<HistoryWriteRequest>,
) {
    let mut buffer = [0u8; 4096];
    let mut pending = String::new();
    let mut utf8_carry = Vec::new();

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                flush_utf8_tail(&mut utf8_carry, &mut pending);
                flush_output(&app_handle, &history_tx, session_id, &shared, &mut pending);
                let _ = cleanup_tx.send(CleanupMessage {
                    session_id,
                    reason: "eof".to_string(),
                    exit_code: None,
                });
                break;
            }
            Ok(n) => {
                decode_utf8_streaming_chunk(&mut utf8_carry, &buffer[..n], &mut pending);
                flush_output(&app_handle, &history_tx, session_id, &shared, &mut pending);
            }
            Err(err) => {
                flush_utf8_tail(&mut utf8_carry, &mut pending);
                flush_output(&app_handle, &history_tx, session_id, &shared, &mut pending);
                emit_error(&app_handle, session_id, &format!("reader error: {err}"));
                let _ = cleanup_tx.send(CleanupMessage {
                    session_id,
                    reason: "error".to_string(),
                    exit_code: None,
                });
                break;
            }
        }
    }
}

fn writer_thread(mut writer: Box<dyn Write + Send>, mut input_rx: mpsc::Receiver<Vec<u8>>) {
    while let Some(bytes) = input_rx.blocking_recv() {
        if writer.write_all(&bytes).is_err() {
            break;
        }
        if writer.flush().is_err() {
            break;
        }
    }
}

fn close_runtime(mut runtime: SessionRuntime) {
    let _ = runtime.child.kill();
    drop(runtime.input_tx);
    drop(runtime.master);

    if let Some(handle) = runtime.reader_handle.take() {
        let _ = handle.join();
    }

    let _ = runtime.child.wait();

    if let Some(handle) = runtime.writer_handle.take() {
        let _ = handle.join();
    }
}

fn emit_exited_once(
    app_handle: &AppHandle,
    session_id: Uuid,
    shared: &Arc<Mutex<SessionShared>>,
    reason: &str,
    exit_code: Option<i32>,
) {
    let should_emit = match shared.lock() {
        Ok(mut state) => {
            state.status = SessionStatus::Disconnected;
            if state.exited_emitted {
                false
            } else {
                state.exited_emitted = true;
                true
            }
        }
        Err(_) => false,
    };

    if should_emit {
        emit_exited(app_handle, session_id, reason, exit_code);
    }
}

fn flush_output(
    app_handle: &AppHandle,
    history_tx: &std_mpsc::Sender<HistoryWriteRequest>,
    session_id: Uuid,
    shared: &Arc<Mutex<SessionShared>>,
    pending: &mut String,
) {
    if pending.is_empty() {
        return;
    }

    let chunk = std::mem::take(pending);

    let (seq, persist_history) = match shared.lock() {
        Ok(mut state) => {
            state.seq += 1;
            state.output.push_str(&chunk);
            if state.output.len() > MAX_SNAPSHOT_BYTES {
                let overflow = state.output.len() - MAX_SNAPSHOT_BYTES;
                let drain_to = utf8_safe_drain_index(&state.output, overflow);
                state.output.drain(..drain_to);
            }
            (state.seq, state.persist_history)
        }
        Err(_) => (0, false),
    };

    if persist_history {
        let _ = history_tx.send(HistoryWriteRequest {
            chunk: HistoryChunk {
                session_id: session_id.to_string(),
                seq,
                line_count: chunk.chars().filter(|c| *c == '\n').count().max(1) as u64,
                chunk_text: chunk.clone(),
                ts: now_ts_millis(),
            },
        });
    }

    let payload = PtyOutputEvent {
        session_id: session_id.to_string(),
        chunk,
        seq,
        ts: now_ts_millis(),
    };

    let _ = app_handle.emit("pty/output", payload);
}

fn emit_exited(app_handle: &AppHandle, session_id: Uuid, reason: &str, exit_code: Option<i32>) {
    let payload = PtyExitedEvent {
        session_id: session_id.to_string(),
        exit_code,
        reason: reason.to_string(),
    };
    let _ = app_handle.emit("pty/exited", payload);
}

fn emit_error(app_handle: &AppHandle, session_id: Uuid, message: &str) {
    let payload = PtyErrorEvent {
        session_id: session_id.to_string(),
        message: message.to_string(),
    };
    let _ = app_handle.emit("pty/error", payload);
}

fn decode_utf8_streaming_chunk(utf8_carry: &mut Vec<u8>, bytes: &[u8], output: &mut String) {
    utf8_carry.extend_from_slice(bytes);

    loop {
        match std::str::from_utf8(utf8_carry) {
            Ok(valid) => {
                output.push_str(valid);
                utf8_carry.clear();
                break;
            }
            Err(err) => {
                let valid_up_to = err.valid_up_to();
                if valid_up_to > 0 {
                    let valid_prefix = std::str::from_utf8(&utf8_carry[..valid_up_to])
                        .expect("utf8 prefix must be valid");
                    output.push_str(valid_prefix);
                }

                match err.error_len() {
                    Some(error_len) => {
                        output.push('\u{FFFD}');
                        let drain_to = (valid_up_to + error_len).min(utf8_carry.len());
                        utf8_carry.drain(..drain_to);
                        if utf8_carry.is_empty() {
                            break;
                        }
                    }
                    None => {
                        utf8_carry.drain(..valid_up_to);
                        break;
                    }
                }
            }
        }
    }
}

fn flush_utf8_tail(utf8_carry: &mut Vec<u8>, output: &mut String) {
    if utf8_carry.is_empty() {
        return;
    }

    output.push_str(&String::from_utf8_lossy(utf8_carry));
    utf8_carry.clear();
}

fn utf8_safe_drain_index(value: &str, minimum_bytes: usize) -> usize {
    let mut index = minimum_bytes.min(value.len());
    while index < value.len() && !value.is_char_boundary(index) {
        index += 1;
    }
    index
}
