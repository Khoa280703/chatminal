use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::workspace_ids::{SessionViewId, WorkspaceNodeId};
use crate::workspace_layout::{WorkspaceLayoutState, WorkspaceSplitAxis};
use chatminal_store::StoredSessionSnapshot;
use chatminal_store::{Store, StoredSession, StoredSessionStatus};

use crate::api::{
    RuntimeCreatedSession, RuntimeEvent, RuntimeProfile, RuntimeSessionLaunchSpec,
    RuntimeSessionSnapshot, RuntimeWorkspace,
};
use crate::config::{RuntimeConfig, resolve_session_cwd};
use crate::metrics::{RuntimeMetrics, RuntimeMetricsSnapshot};
use crate::session::{SessionEvent, WriteInputError};
use crate::state::runtime_bridge::{RuntimeExecutionAdapter, RuntimeHandle};

mod canonical_scrollback;
mod explorer_utils;
mod native_api;
mod persist_worker;
pub mod runtime_bridge;
mod runtime_lifecycle;
mod session_event_processor;
mod session_explorer;
mod startup_recipe;
#[cfg(test)]
pub mod test_bridge;

const MAX_INPUT_BYTES: usize = 65_536;
const KEEP_ALIVE_ON_CLOSE_KEY: &str = "keep_alive_on_close";
const START_IN_TRAY_KEY: &str = "start_in_tray";
const WORKSPACE_LAYOUT_PREFIX: &str = "workspace_layout:";
const DEFAULT_KEEP_ALIVE_ON_CLOSE: bool = true;
const DEFAULT_START_IN_TRAY: bool = false;

struct SessionEntry {
    session: StoredSession,
    runtime: Option<RuntimeHandle>,
    live_output: String,
    canonical_open_fragment: String,
    canonical_cursor_col: usize,
    canonical_pending_carriage_return: bool,
    generation: u64,
    prepend_run_boundary_on_next_output: bool,
    restored_trailing_fragment: Option<String>,
    restored_prompt_redraw_buffer: String,
    recent_output_tail: String,
}

struct SessionSpawnPlan {
    session_id: String,
    profile_id: String,
    expected_active_session_id: Option<String>,
    expected_generation: u64,
    next_generation: u64,
    shell: String,
    cwd: String,
    cols: usize,
    rows: usize,
}

struct StateInner {
    config: RuntimeConfig,
    store: Store,
    metrics: RuntimeMetrics,
    sessions: HashMap<String, SessionEntry>,
    subscribers: HashMap<u64, std_mpsc::SyncSender<RuntimeEvent>>,
    next_subscriber_id: u64,
    shutdown_requested: bool,
    persist_thread: Option<thread::JoinHandle<()>>,
}

#[derive(Clone)]
pub struct RuntimeState {
    // `RuntimeState` owns business/workspace state.
    // Live execution state is delegated to the `execution` adapter (concrete impl in
    // `desktop_host_runtime`). No session_engine types leak into this struct.
    inner: Arc<Mutex<StateInner>>,
    metrics: RuntimeMetrics,
    execution: Arc<dyn RuntimeExecutionAdapter>,
    persist_tx: std_mpsc::SyncSender<persist_worker::PersistJob>,
}

pub struct RuntimeSubscription {
    state: RuntimeState,
    subscriber_id: u64,
    rx: std_mpsc::Receiver<RuntimeEvent>,
}

impl RuntimeSubscription {
    fn new(state: RuntimeState, subscriber_id: u64, rx: std_mpsc::Receiver<RuntimeEvent>) -> Self {
        Self {
            state,
            subscriber_id,
            rx,
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<RuntimeEvent>, String> {
        match self.rx.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(std_mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                Err("runtime event channel disconnected".to_string())
            }
        }
    }
}

impl Drop for RuntimeSubscription {
    fn drop(&mut self) {
        self.state.unsubscribe(self.subscriber_id);
    }
}

impl RuntimeState {
    pub fn initialize_default(
        execution: Arc<dyn RuntimeExecutionAdapter>,
    ) -> Result<(Self, RuntimeConfig), String> {
        let config = RuntimeConfig::from_env()?;
        let store = Store::initialize_default()?;
        let state = Self::new(config.clone(), store, execution)?;
        Ok((state, config))
    }

    pub fn new(
        config: RuntimeConfig,
        store: Store,
        execution: Arc<dyn RuntimeExecutionAdapter>,
    ) -> Result<Self, String> {
        let (events_tx, events_rx) = std_mpsc::sync_channel::<SessionEvent>(4096);
        execution.connect_session_events(events_tx.clone());
        let metrics = RuntimeMetrics::new();
        let mut sessions = HashMap::new();
        let workspace = store.load_workspace()?;

        if workspace.profiles.is_empty() {
            return Err("store has no profiles".to_string());
        }

        for profile in workspace.profiles {
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
                        canonical_open_fragment: String::new(),
                        canonical_cursor_col: 0,
                        canonical_pending_carriage_return: false,
                        generation: 0,
                        prepend_run_boundary_on_next_output: false,
                        restored_trailing_fragment: None,
                        restored_prompt_redraw_buffer: String::new(),
                        recent_output_tail: String::new(),
                    },
                );
            }
        }

        // Keep disconnected state at startup; clients will reactivate when needed.
        for (session_id, entry) in sessions.iter_mut() {
            entry.session.status = StoredSessionStatus::Disconnected;
            let _ = store.set_session_status(session_id, StoredSessionStatus::Disconnected);
        }

        let (persist_tx, persist_thread) = persist_worker::spawn_persist_worker(store.clone());

        let state = Self {
            inner: Arc::new(Mutex::new(StateInner {
                config,
                store,
                metrics: metrics.clone(),
                sessions,
                subscribers: HashMap::new(),
                next_subscriber_id: 1,
                shutdown_requested: false,
                persist_thread: Some(persist_thread),
            })),
            metrics,
            execution,
            persist_tx,
        };

        let cloned = state.clone();
        std::thread::spawn(move || {
            while let Ok(event) = events_rx.recv() {
                cloned.apply_session_event(event);
            }
        });

        Ok(state)
    }

    pub fn subscribe(&self) -> Result<RuntimeSubscription, String> {
        let (tx, rx) = std_mpsc::sync_channel::<RuntimeEvent>(1024);
        let subscriber_id = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let subscriber_id = inner.next_subscriber_id;
            inner.next_subscriber_id = inner.next_subscriber_id.saturating_add(1);
            inner.subscribers.insert(subscriber_id, tx);
            subscriber_id
        };

        Ok(RuntimeSubscription::new(self.clone(), subscriber_id, rx))
    }

    pub(crate) fn unsubscribe(&self, subscriber_id: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.subscribers.remove(&subscriber_id);
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

    pub fn metrics_snapshot(&self) -> RuntimeMetricsSnapshot {
        self.metrics.snapshot()
    }

    pub fn workspace_load(&self) -> Result<RuntimeWorkspace, String> {
        self.ensure_active_session_runtime()?;
        self.workspace_load_passive()
    }

    pub fn workspace_load_passive(&self) -> Result<RuntimeWorkspace, String> {
        let (store, session_overrides) = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let session_overrides = inner
                .sessions
                .iter()
                .map(|(session_id, entry)| {
                    (
                        session_id.clone(),
                        (
                            entry.session.status.clone(),
                            entry.session.seq,
                            entry.session.persist_history,
                            entry.session.cwd.clone(),
                            entry.session.name.clone(),
                            entry.session.startup_command.clone(),
                        ),
                    )
                })
                .collect::<HashMap<_, _>>();
            (inner.store.clone(), session_overrides)
        };

        let mut workspace = store.load_workspace()?;
        for session in &mut workspace.sessions {
            if let Some((status, seq, persist_history, cwd, name, startup_command)) =
                session_overrides.get(&session.session_id)
            {
                session.status = status.clone();
                session.seq = *seq;
                session.persist_history = *persist_history;
                session.cwd = cwd.clone();
                session.name = name.clone();
                session.startup_command = startup_command.clone();
            }
        }

        Ok(RuntimeWorkspace {
            profiles: workspace.profiles.into_iter().map(Into::into).collect(),
            active_profile_id: Some(workspace.active_profile_id),
            sessions: workspace.sessions.into_iter().map(Into::into).collect(),
            active_session_id: workspace.active_session_id,
        })
    }

    pub fn profile_create(&self, name: Option<String>) -> Result<RuntimeProfile, String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.profile_create(name)
    }

    pub fn profile_switch(&self, profile_id: &str) -> Result<RuntimeWorkspace, String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.profile_switch(profile_id)
    }

    pub fn session_move_to_profile(
        &self,
        session_id: &str,
        profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<RuntimeWorkspace, String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.session_move_to_profile(session_id, profile_id, target_index)
    }

    pub fn sessions_move_to_profile(
        &self,
        session_ids: &[String],
        profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<RuntimeWorkspace, String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.sessions_move_to_profile(session_ids, profile_id, target_index)
    }

    pub fn session_rename(&self, session_id: &str, name: &str) -> Result<RuntimeWorkspace, String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.session_rename(session_id, name)
    }

    pub fn session_create(
        &self,
        name: Option<String>,
        cols: usize,
        rows: usize,
        cwd: Option<String>,
        persist_history: Option<bool>,
    ) -> Result<RuntimeCreatedSession, String> {
        let (store, active_profile_id, default_shell) = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            (
                inner.store.clone(),
                inner.store.load_workspace()?.active_profile_id,
                inner.config.default_shell.clone(),
            )
        };

        let created = store.create_session(
            &active_profile_id,
            name,
            resolve_session_cwd(cwd),
            default_shell,
            persist_history.unwrap_or(false),
        )?;
        let runtime = match self.spawn_runtime_handle(
            &created.session_id,
            0,
            &created.shell,
            &created.cwd,
            cols,
            rows,
        ) {
            Ok(runtime) => runtime,
            Err(err) => {
                let _ = store.delete_session(&created.session_id);
                return Err(err);
            }
        };

        {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            inner.insert_running_session_and_publish(created.clone(), runtime)?;
        }

        Ok(RuntimeCreatedSession {
            session_id: created.session_id,
            name: created.name,
        })
    }

    /// Activate using config default terminal size. Convenience for callers that
    /// don't have an explicit size (e.g. the `SessionWorkspaceHost` bridge adapter).
    pub fn session_activate_with_default_size(&self, session_id: &str) -> Result<(), String> {
        let (cols, rows) = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            (inner.config.default_cols, inner.config.default_rows)
        };
        self.session_activate(session_id, cols, rows)
    }

    pub fn session_activate(
        &self,
        session_id: &str,
        cols: usize,
        rows: usize,
    ) -> Result<(), String> {
        enum Activation {
            Existing(RuntimeHandle, String),
            Recover(Option<RuntimeHandle>, SessionSpawnPlan),
            Spawn(SessionSpawnPlan),
        }

        let activation = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let attachment = self.session_runtime_attachment(session_id);
            let Some(entry) = inner.sessions.get_mut(session_id) else {
                return Err("session not found".to_string());
            };
            if let Some(runtime) = entry.runtime.clone() {
                if attachment.is_some() {
                    Activation::Existing(runtime, entry.session.profile_id.clone())
                } else {
                    log::warn!(
                        "session_activate: recovering stale runtime handle for session {session_id}"
                    );
                    let stale_runtime = entry.runtime.take();
                    entry.generation = entry.generation.saturating_add(1);
                    entry.session.status = StoredSessionStatus::Disconnected;
                    entry.prepend_run_boundary_on_next_output = false;
                    entry.restored_trailing_fragment = None;
                    entry.restored_prompt_redraw_buffer.clear();
                    Activation::Recover(
                        stale_runtime,
                        SessionSpawnPlan {
                            session_id: entry.session.session_id.clone(),
                            profile_id: entry.session.profile_id.clone(),
                            expected_active_session_id: None,
                            expected_generation: entry.generation,
                            next_generation: entry.generation.saturating_add(1),
                            shell: entry.session.shell.clone(),
                            cwd: entry.session.cwd.clone(),
                            cols,
                            rows,
                        },
                    )
                }
            } else {
                Activation::Spawn(SessionSpawnPlan {
                    session_id: entry.session.session_id.clone(),
                    profile_id: entry.session.profile_id.clone(),
                    expected_active_session_id: None,
                    expected_generation: entry.generation,
                    next_generation: entry.generation.saturating_add(1),
                    shell: entry.session.shell.clone(),
                    cwd: entry.session.cwd.clone(),
                    cols,
                    rows,
                })
            }
        };

        match activation {
            Activation::Existing(runtime, profile_id) => {
                log::warn!(
                    "session_activate(existing): session={} size={}x{}",
                    session_id,
                    cols,
                    rows
                );
                runtime
                    .lock()
                    .map_err(|_| "session runtime lock poisoned".to_string())?
                    .resize(cols, rows)?;

                let mut inner = self
                    .inner
                    .lock()
                    .map_err(|_| "state lock poisoned".to_string())?;
                if !inner.sessions.contains_key(session_id) {
                    return Err("session not found".to_string());
                }
                inner.set_active_session_and_publish(&profile_id, session_id)?;
                Ok(())
            }
            Activation::Recover(stale_runtime, plan) => {
                kill_runtime_handle(stale_runtime);
                self.commit_spawned_session(plan)
            }
            Activation::Spawn(plan) => self.commit_spawned_session(plan),
        }
    }

    pub fn session_focus(&self, session_id: &str) -> Result<(), String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        let Some(entry) = inner.sessions.get(session_id) else {
            return Err("session not found".to_string());
        };
        let profile_id = entry.session.profile_id.clone();
        inner.set_active_session_and_publish(&profile_id, session_id)
    }

    pub fn session_snapshot_get(
        &self,
        session_id: &str,
        preview_lines: Option<usize>,
    ) -> Result<RuntimeSessionSnapshot, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.session_snapshot_get(session_id, preview_lines)
    }

    pub fn session_restore_snapshot_get(
        &self,
        session_id: &str,
    ) -> Result<RuntimeSessionSnapshot, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.session_restore_snapshot_get(session_id)
    }

    pub fn session_launch_spec(
        &self,
        session_id: &str,
    ) -> Result<RuntimeSessionLaunchSpec, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        let Some(entry) = inner.sessions.get(session_id) else {
            return Err("session not found".to_string());
        };
        Ok(RuntimeSessionLaunchSpec {
            session_id: entry.session.session_id.clone(),
            shell: entry.session.shell.clone(),
            cwd: entry.session.cwd.clone(),
        })
    }

    pub fn session_set_persist(
        &self,
        session_id: &str,
        persist_history: bool,
    ) -> Result<(), String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.session_set_persist(session_id, persist_history)
    }

    pub fn session_set_startup_command(
        &self,
        session_id: &str,
        startup_command: Option<&str>,
    ) -> Result<RuntimeWorkspace, String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.session_set_startup_command(session_id, startup_command)
    }

    pub fn session_run_startup_command(&self, session_id: &str) -> Result<(), String> {
        self.spawn_startup_recipe_runner(session_id)
    }

    pub fn get_lifecycle_preferences(
        &self,
    ) -> Result<crate::api::RuntimeLifecyclePreferences, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.get_lifecycle_preferences()
    }

    pub fn set_lifecycle_preferences(
        &self,
        keep_alive_on_close: Option<bool>,
        start_in_tray: Option<bool>,
    ) -> Result<crate::api::RuntimeLifecyclePreferences, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.set_lifecycle_preferences(keep_alive_on_close, start_in_tray)
    }

    pub fn profile_delete(&self, profile_id: &str) -> Result<RuntimeWorkspace, String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.store.delete_profile(profile_id)?;
        inner.close_profile_runtimes(profile_id);
        inner.publish_workspace_updated();
        inner.load_workspace_snapshot()
    }

    pub fn session_explorer_state_get(
        &self,
        session_id: &str,
    ) -> Result<crate::api::RuntimeSessionExplorerState, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.get_session_explorer_state(session_id)
    }

    pub fn session_explorer_root_set(
        &self,
        session_id: &str,
        root_path: &str,
    ) -> Result<crate::api::RuntimeSessionExplorerState, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.set_session_explorer_root(session_id, root_path)
    }

    pub fn session_explorer_state_update(
        &self,
        session_id: &str,
        current_dir: &str,
        selected_path: Option<&str>,
        open_file_path: Option<&str>,
    ) -> Result<crate::api::RuntimeSessionExplorerState, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.update_session_explorer_state(session_id, current_dir, selected_path, open_file_path)
    }

    pub fn session_explorer_list(
        &self,
        session_id: &str,
        relative_path: Option<&str>,
    ) -> Result<Vec<crate::api::RuntimeSessionExplorerEntry>, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.list_session_explorer_entries(session_id, relative_path)
    }

    pub fn session_explorer_read_file(
        &self,
        session_id: &str,
        relative_path: &str,
        max_bytes: Option<usize>,
    ) -> Result<crate::api::RuntimeSessionExplorerFileContent, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.read_session_explorer_file(session_id, relative_path, max_bytes)
    }

    pub fn workspace_layout_load(
        &self,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.workspace_layout_load(workspace_id)
    }

    pub fn workspace_layout_save(
        &self,
        workspace_id: &str,
        layout: &WorkspaceLayoutState,
    ) -> Result<(), String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.workspace_layout_save(workspace_id, layout)
    }

    pub fn workspace_layout_clear(&self, workspace_id: &str) -> Result<(), String> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.workspace_layout_clear(workspace_id)
    }

    pub fn workspace_layout_restore_persisted(
        &self,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let persisted = self.workspace_layout_load(workspace_id)?;
        let layouts = self.execution.workspace_layouts();
        let mut layouts = layouts
            .lock()
            .map_err(|_| "workspace layout lock poisoned".to_string())?;
        if let Some(layout) = persisted.clone() {
            layouts.replace_layout(workspace_id.to_string(), layout);
        } else {
            layouts.remove_layout(workspace_id);
        }
        Ok(persisted)
    }

    pub fn workspace_layout_snapshot(
        &self,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let layouts = self.execution.workspace_layouts();
        let layouts = layouts
            .lock()
            .map_err(|_| "workspace layout lock poisoned".to_string())?;
        Ok(layouts.layout(workspace_id).cloned())
    }

    pub fn workspace_layout_remove(&self, workspace_id: &str) -> Result<(), String> {
        let layouts = self.execution.workspace_layouts();
        layouts
            .lock()
            .map_err(|_| "workspace layout lock poisoned".to_string())?
            .remove_layout(workspace_id);
        self.workspace_layout_clear(workspace_id)
    }

    pub fn workspace_layout_replace(
        &self,
        workspace_id: &str,
        layout: WorkspaceLayoutState,
    ) -> Result<WorkspaceLayoutState, String> {
        let layout = {
            let layouts = self.execution.workspace_layouts();
            layouts
                .lock()
                .map_err(|_| "workspace layout lock poisoned".to_string())?
                .replace_layout(workspace_id.to_string(), layout)
        };
        self.workspace_layout_save(workspace_id, &layout)?;
        Ok(layout)
    }

    pub fn workspace_layout_ensure_session(
        &self,
        workspace_id: &str,
        session_id: &str,
    ) -> Result<WorkspaceLayoutState, String> {
        let layout = {
            let layouts = self.execution.workspace_layouts();
            let mut layouts = layouts
                .lock()
                .map_err(|_| "workspace layout lock poisoned".to_string())?;
            let layout = layouts.ensure_layout(workspace_id.to_string(), session_id.to_string());
            layouts
                .ensure_session_view(workspace_id, session_id.to_string())
                .unwrap_or(layout)
        };
        self.workspace_layout_save(workspace_id, &layout)?;
        Ok(layout)
    }

    pub fn workspace_layout_split_view(
        &self,
        workspace_id: &str,
        view_id: SessionViewId,
        axis: WorkspaceSplitAxis,
        session_id: &str,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let layout = {
            let layouts = self.execution.workspace_layouts();
            layouts
                .lock()
                .map_err(|_| "workspace layout lock poisoned".to_string())?
                .split_view(workspace_id, view_id, axis, session_id.to_string())
        };
        if let Some(layout_ref) = layout.as_ref() {
            self.workspace_layout_save(workspace_id, layout_ref)?;
        }
        Ok(layout)
    }

    pub fn workspace_layout_attach_session(
        &self,
        workspace_id: &str,
        view_id: SessionViewId,
        session_id: &str,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let layout = {
            let layouts = self.execution.workspace_layouts();
            layouts
                .lock()
                .map_err(|_| "workspace layout lock poisoned".to_string())?
                .attach_session(workspace_id, view_id, session_id.to_string())
        };
        if let Some(layout_ref) = layout.as_ref() {
            self.workspace_layout_save(workspace_id, layout_ref)?;
        }
        Ok(layout)
    }

    pub fn workspace_layout_focus_view(
        &self,
        workspace_id: &str,
        view_id: SessionViewId,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let layout = {
            let layouts = self.execution.workspace_layouts();
            layouts
                .lock()
                .map_err(|_| "workspace layout lock poisoned".to_string())?
                .focus_view(workspace_id, view_id)
        };
        if let Some(layout_ref) = layout.as_ref() {
            self.workspace_layout_save(workspace_id, layout_ref)?;
        }
        Ok(layout)
    }

    pub fn workspace_layout_close_view(
        &self,
        workspace_id: &str,
        view_id: SessionViewId,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let layout = {
            let layouts = self.execution.workspace_layouts();
            layouts
                .lock()
                .map_err(|_| "workspace layout lock poisoned".to_string())?
                .close_view(workspace_id, view_id)
        };
        if let Some(layout_ref) = layout.as_ref() {
            self.workspace_layout_save(workspace_id, layout_ref)?;
        }
        Ok(layout)
    }

    pub fn workspace_layout_resize_split(
        &self,
        workspace_id: &str,
        node_id: WorkspaceNodeId,
        ratio: u16,
    ) -> Result<Option<WorkspaceLayoutState>, String> {
        let layout = {
            let layouts = self.execution.workspace_layouts();
            layouts
                .lock()
                .map_err(|_| "workspace layout lock poisoned".to_string())?
                .resize_split(workspace_id, node_id, ratio)
        };
        if let Some(layout_ref) = layout.as_ref() {
            self.workspace_layout_save(workspace_id, layout_ref)?;
        }
        Ok(layout)
    }

    pub fn workspace_layout_view_id_for_session(
        &self,
        workspace_id: &str,
        session_id: &str,
    ) -> Result<Option<SessionViewId>, String> {
        let layouts = self.execution.workspace_layouts();
        let layouts = layouts
            .lock()
            .map_err(|_| "workspace layout lock poisoned".to_string())?;
        Ok(layouts.view_id_for_session(workspace_id, session_id))
    }

    pub fn session_resize(&self, session_id: &str, cols: usize, rows: usize) -> Result<(), String> {
        let runtime = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let Some(entry) = inner.sessions.get(session_id) else {
                return Err("session not found".to_string());
            };
            entry
                .runtime
                .clone()
                .ok_or_else(|| "session is not running".to_string())?
        };

        let mut runtime = runtime
            .lock()
            .map_err(|_| "session runtime lock poisoned".to_string())?;
        runtime.resize(cols, rows)
    }

    pub fn session_input_write(&self, session_id: &str, data: &str) -> Result<(), String> {
        if data.len() > MAX_INPUT_BYTES {
            return Err(format!(
                "input payload too large ({} bytes > {} bytes)",
                data.len(),
                MAX_INPUT_BYTES
            ));
        }

        let runtime = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let Some(entry) = inner.sessions.get(session_id) else {
                return Err("session not found".to_string());
            };
            entry
                .runtime
                .clone()
                .ok_or_else(|| "session is not running".to_string())?
        };

        let runtime = runtime
            .lock()
            .map_err(|_| "session runtime lock poisoned".to_string())?;
        match runtime.write_input(data) {
            Ok(stats) => {
                self.metrics
                    .add_input_queue_full_total(stats.queue_full_hits);
                self.metrics.add_input_retry_total(stats.retries);
                self.metrics.add_input_drop_total(stats.drops);
                Ok(())
            }
            Err(WriteInputError::QueueFullDropped(stats)) => {
                self.metrics
                    .add_input_queue_full_total(stats.queue_full_hits);
                self.metrics.add_input_retry_total(stats.retries);
                self.metrics.add_input_drop_total(stats.drops);
                Err(WriteInputError::QueueFullDropped(stats).to_string())
            }
            Err(err) => Err(err.to_string()),
        }
    }

    pub fn session_close(&self, session_id: &str) -> Result<(), String> {
        let runtime = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            inner.remove_session_and_publish_workspace(session_id)?
        };

        kill_runtime_handle(runtime);
        Ok(())
    }

    pub fn session_history_clear(&self, session_id: &str) -> Result<(), String> {
        let runtime = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            inner.clear_session_history_and_publish(session_id)?
        };

        kill_runtime_handle(runtime);
        Ok(())
    }

    pub fn workspace_history_clear_all(&self) -> Result<(), String> {
        let runtimes = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            inner.store.clear_all_history()?;
            inner.disconnect_all_sessions_and_publish(runtime_lifecycle::DisconnectOptions {
                reset_history: true,
                bump_generation: true,
            })
        };

        kill_runtime_handles(runtimes);
        Ok(())
    }

    pub fn app_shutdown(&self) {
        // Signal persist worker to flush remaining jobs and stop.
        let _ = self.persist_tx.send(persist_worker::PersistJob::Shutdown);

        let runtimes = {
            let mut inner = match self.inner.lock() {
                Ok(value) => value,
                Err(_) => return,
            };
            inner.shutdown_requested = true;

            // Join persist thread for orderly shutdown.
            if let Some(thread) = inner.persist_thread.take() {
                let _ = thread.join();
            }

            inner.disconnect_all_sessions_and_publish(runtime_lifecycle::DisconnectOptions {
                reset_history: false,
                bump_generation: false,
            })
        };

        kill_runtime_handles(runtimes);
    }

    /// Wait for all pending persist jobs to flush. Test-only.
    #[cfg(test)]
    pub(super) fn flush_persist(&self) {
        let (ack_tx, ack_rx) = std_mpsc::sync_channel::<()>(1);
        let _ = self
            .persist_tx
            .send(persist_worker::PersistJob::Flush { ack: ack_tx });
        let _ = ack_rx.recv();
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
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
fn snapshot_requires_run_boundary(snapshot: &StoredSessionSnapshot) -> bool {
    !snapshot.content.is_empty()
        && !snapshot.content.ends_with('\n')
        && !snapshot.content.ends_with('\r')
}

#[cfg(test)]
fn snapshot_trailing_fragment(snapshot: &StoredSessionSnapshot) -> Option<String> {
    if !snapshot_requires_run_boundary(snapshot) {
        return None;
    }

    let cut = snapshot
        .content
        .rfind(['\n', '\r'])
        .map(|index| index.saturating_add(1))
        .unwrap_or(0);
    let fragment = snapshot.content[cut..].to_string();
    (!fragment.is_empty()).then_some(fragment)
}

fn normalize_restore_replay_snapshot(mut snapshot: StoredSessionSnapshot) -> StoredSessionSnapshot {
    snapshot.content = strip_volatile_terminal_control_sequences(&snapshot.content);
    snapshot.content = collapse_trailing_restored_prompt_fragment_dup(&snapshot.content);
    snapshot
}

#[cfg(test)]
fn normalize_session_snapshot(mut snapshot: StoredSessionSnapshot) -> StoredSessionSnapshot {
    snapshot.content = strip_volatile_terminal_control_sequences(&snapshot.content);
    snapshot.content = collapse_trailing_duplicate_prompt_redraws(&snapshot.content);
    snapshot.content = collapse_trailing_prompt_only_lines(&snapshot.content);
    snapshot
}

fn strip_volatile_terminal_control_sequences(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != 0x1b {
            out.push(bytes[i]);
            i += 1;
            continue;
        }

        let Some(next) = bytes.get(i + 1).copied() else {
            out.push(bytes[i]);
            break;
        };

        match next {
            b']' => {
                let start = i;
                i += 2;
                let command_start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let command = &bytes[command_start..i];
                let has_separator = bytes.get(i) == Some(&b';');
                if has_separator {
                    i += 1;
                }

                while i < bytes.len() {
                    match bytes[i] {
                        0x07 => {
                            i += 1;
                            break;
                        }
                        0x1b if bytes.get(i + 1) == Some(&b'\\') => {
                            i += 2;
                            break;
                        }
                        _ => i += 1,
                    }
                }

                let is_title_command = matches!(command, b"0" | b"1" | b"2");
                if !(has_separator && is_title_command) {
                    out.extend_from_slice(&bytes[start..i]);
                }
            }
            b'[' => {
                let start = i;
                i += 2;
                let private_mode = bytes.get(i) == Some(&b'?');
                if private_mode {
                    i += 1;
                }
                while i < bytes.len() {
                    let byte = bytes[i];
                    i += 1;
                    if (0x40..=0x7e).contains(&byte) {
                        let is_mode_toggle = private_mode && matches!(byte, b'h' | b'l');
                        if !is_mode_toggle {
                            out.extend_from_slice(&bytes[start..i]);
                        }
                        break;
                    }
                }
            }
            _ => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).to_string()
}

fn visible_terminal_fragment(value: &str) -> String {
    crate::terminal_text_utils::visible_terminal_fragment(value)
}

#[cfg(test)]
fn normalize_terminal_fragment_for_compare(value: &str) -> String {
    visible_terminal_fragment(value).trim().to_string()
}

fn looks_like_shell_prompt_fragment(value: &str) -> bool {
    let visible = visible_terminal_fragment(value);
    if !visible.ends_with(' ') {
        return false;
    }
    let trimmed = visible.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    match trimmed.chars().last() {
        Some('%') => trimmed.contains('@') || trimmed.contains('~'),
        Some('$') | Some('#') => {
            trimmed.contains('@')
                || trimmed.starts_with("PS ")
                || trimmed.contains(":~")
                || trimmed.contains(":/")
                || trimmed.contains(":\\")
        }
        Some('>') => trimmed.starts_with("PS ") || trimmed.contains('@'),
        _ => false,
    }
}

#[cfg(test)]
fn is_duplicate_restored_prompt_fragment(restored: &str, chunk: &str) -> bool {
    strip_duplicate_restored_prompt_prefix(restored, chunk)
        .is_some_and(|remainder| remainder.is_empty())
}

fn visible_prefix_end_for_fragment(chunk: &str, expected_visible: &str) -> Option<usize> {
    let expected_chars: Vec<char> = expected_visible.chars().collect();
    if expected_chars.is_empty() {
        return None;
    }

    let chars: Vec<(usize, char)> = chunk.char_indices().collect();
    let mut idx = 0usize;
    let mut matched = 0usize;
    while idx < chars.len() {
        let (byte_idx, ch) = chars[idx];
        if ch == '\u{1b}' {
            idx += 1;
            if idx >= chars.len() {
                break;
            }
            match chars[idx].1 {
                '[' => {
                    idx += 1;
                    while idx < chars.len() {
                        let c = chars[idx].1;
                        idx += 1;
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                ']' => {
                    idx += 1;
                    while idx < chars.len() {
                        let c = chars[idx].1;
                        idx += 1;
                        if c == '\u{7}' {
                            break;
                        }
                        if c == '\u{1b}' && idx < chars.len() && chars[idx].1 == '\\' {
                            idx += 1;
                            break;
                        }
                    }
                }
                'P' | '^' | '_' | 'X' => {
                    idx += 1;
                    while idx < chars.len() {
                        let c = chars[idx].1;
                        idx += 1;
                        if c == '\u{1b}' && idx < chars.len() && chars[idx].1 == '\\' {
                            idx += 1;
                            break;
                        }
                    }
                }
                _ => {
                    idx += 1;
                }
            }
            continue;
        }

        idx += 1;
        if ch == '\r' || ch == '\n' || ch.is_control() {
            continue;
        }
        if expected_chars.get(matched) != Some(&ch) {
            return None;
        }
        matched += 1;
        if matched == expected_chars.len() {
            return Some(byte_idx + ch.len_utf8());
        }
    }

    None
}

fn strip_duplicate_restored_prompt_prefix(restored: &str, chunk: &str) -> Option<String> {
    let expected_visible = visible_terminal_fragment(restored);
    if !looks_like_shell_prompt_fragment(restored) {
        return None;
    }
    let prefix_end = visible_prefix_end_for_fragment(chunk, &expected_visible)?;
    Some(chunk[prefix_end..].to_string())
}

enum RestoredPromptRedrawDecision {
    AwaitMore,
    Strip(String),
    Mismatch,
}

fn classify_restored_prompt_redraw(restored: &str, buffered_chunk: &str) -> RestoredPromptRedrawDecision {
    let expected_visible = visible_terminal_fragment(restored);
    if !looks_like_shell_prompt_fragment(restored) {
        return RestoredPromptRedrawDecision::Mismatch;
    }

    let visible = visible_terminal_fragment(buffered_chunk);
    if visible.is_empty() {
        return RestoredPromptRedrawDecision::AwaitMore;
    }

    if let Some(stripped) = strip_duplicate_restored_prompt_prefix(restored, buffered_chunk) {
        return RestoredPromptRedrawDecision::Strip(stripped);
    }

    if expected_visible.starts_with(&visible) {
        return RestoredPromptRedrawDecision::AwaitMore;
    }

    RestoredPromptRedrawDecision::Mismatch
}

fn collapse_trailing_restored_prompt_fragment_dup(content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }

    let lines = split_lines_with_endings(content);
    let Some(last) = lines.last() else {
        return String::new();
    };
    if last.raw.ends_with('\n') || last.raw.ends_with('\r') {
        return content.to_string();
    }

    let last_visible = visible_terminal_fragment(last.content);
    if last_visible.is_empty() || !looks_like_shell_prompt_fragment(last.content) {
        return content.to_string();
    }

    let mut duplicate_start = lines.len();
    for index in (0..lines.len().saturating_sub(1)).rev() {
        let line = &lines[index];
        let visible = visible_terminal_fragment(line.content);
        if visible != last_visible || !looks_like_shell_prompt_fragment(line.content) {
            break;
        }
        duplicate_start = index;
    }

    if duplicate_start == lines.len() {
        return content.to_string();
    }

    let prefix = lines[..duplicate_start]
        .iter()
        .map(|line| line.raw)
        .collect::<String>();
    format!("{prefix}{}", last.raw)
}

#[cfg(test)]
fn collapse_trailing_duplicate_prompt_redraws(content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }

    let line_start = content
        .rfind('\n')
        .map(|index| index.saturating_add(1))
        .unwrap_or(0);
    let tail_raw = &content[line_start..];
    let tail_visible = visible_terminal_fragment(tail_raw);
    let Some((cluster_visible_start, prompt_visible)) =
        find_repeated_trailing_prompt_suffix(&tail_visible)
    else {
        return content.to_string();
    };

    let Some(cluster_raw_start) = raw_index_for_visible_offset(tail_raw, cluster_visible_start)
    else {
        return content.to_string();
    };
    let last_prompt_visible_start = tail_visible.len().saturating_sub(prompt_visible.len());
    let Some(last_prompt_raw_start) =
        raw_index_for_visible_offset(tail_raw, last_prompt_visible_start)
    else {
        return content.to_string();
    };

    format!(
        "{}{}{}",
        &content[..line_start],
        &tail_raw[..cluster_raw_start],
        &tail_raw[last_prompt_raw_start..]
    )
}

#[cfg(test)]
fn collapse_trailing_prompt_only_lines(content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }

    let lines = split_lines_with_endings(content);
    if lines.is_empty() {
        return content.to_string();
    }

    let mut cluster_start = lines.len();
    let mut prompt_count = 0usize;
    let mut last_prompt_index = None;

    for index in (0..lines.len()).rev() {
        let line = &lines[index];
        let visible = visible_terminal_fragment(line.content.as_ref());
        if visible.trim().is_empty() {
            cluster_start = index;
            continue;
        }
        if looks_like_shell_prompt_fragment(line.content.as_ref()) {
            cluster_start = index;
            prompt_count += 1;
            if last_prompt_index.is_none() {
                last_prompt_index = Some(index);
            }
            continue;
        }
        break;
    }

    if prompt_count < 2 {
        return content.to_string();
    }

    let Some(last_prompt_index) = last_prompt_index else {
        return content.to_string();
    };

    let prefix = lines[..cluster_start]
        .iter()
        .map(|line| line.raw)
        .collect::<String>();
    let preserved_prompt = lines[last_prompt_index].raw;
    format!("{prefix}{preserved_prompt}")
}

struct SnapshotLine<'a> {
    content: &'a str,
    raw: &'a str,
}

fn split_lines_with_endings(content: &str) -> Vec<SnapshotLine<'_>> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\r' => {
                let mut end = idx + 1;
                if bytes.get(end) == Some(&b'\n') {
                    end += 1;
                }
                out.push(SnapshotLine {
                    content: &content[start..idx],
                    raw: &content[start..end],
                });
                start = end;
                idx = end;
            }
            b'\n' => {
                let end = idx + 1;
                out.push(SnapshotLine {
                    content: &content[start..idx],
                    raw: &content[start..end],
                });
                start = end;
                idx = end;
            }
            _ => idx += 1,
        }
    }

    if start < content.len() {
        out.push(SnapshotLine {
            content: &content[start..],
            raw: &content[start..],
        });
    }

    out
}

#[cfg(test)]
fn find_repeated_trailing_prompt_suffix(line: &str) -> Option<(usize, String)> {
    if line.is_empty() {
        return None;
    }

    let mut boundaries = line
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    boundaries.push(line.len());

    for &start in boundaries.iter().rev().skip(1) {
        let candidate = &line[start..];
        if !looks_like_shell_prompt_fragment(candidate) {
            continue;
        }
        if !line[..start].ends_with(candidate) {
            continue;
        }

        let mut cluster_start = start;
        while let Some(prefix) = line[..cluster_start].strip_suffix(candidate) {
            cluster_start = prefix.len();
        }
        return Some((cluster_start, candidate.to_string()));
    }

    None
}

#[cfg(test)]
fn raw_index_for_visible_offset(raw: &str, target_visible_offset: usize) -> Option<usize> {
    if target_visible_offset == 0 {
        return Some(0);
    }

    let chars: Vec<(usize, char)> = raw.char_indices().collect();
    let mut idx = 0usize;
    let mut visible_offset = 0usize;

    while idx < chars.len() {
        let (byte_idx, ch) = chars[idx];
        if ch == '\u{1b}' {
            idx += 1;
            if idx >= chars.len() {
                break;
            }
            match chars[idx].1 {
                '[' => {
                    idx += 1;
                    while idx < chars.len() {
                        let next = chars[idx].1;
                        idx += 1;
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                ']' => {
                    idx += 1;
                    while idx < chars.len() {
                        let next = chars[idx].1;
                        idx += 1;
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' && idx < chars.len() && chars[idx].1 == '\\' {
                            idx += 1;
                            break;
                        }
                    }
                }
                'P' | '^' | '_' | 'X' => {
                    idx += 1;
                    while idx < chars.len() {
                        let next = chars[idx].1;
                        idx += 1;
                        if next == '\u{1b}' && idx < chars.len() && chars[idx].1 == '\\' {
                            idx += 1;
                            break;
                        }
                    }
                }
                _ => {
                    idx += 1;
                }
            }
            continue;
        }

        idx += 1;
        if ch == '\r' || ch == '\n' || ch.is_control() {
            continue;
        }

        if visible_offset == target_visible_offset {
            return Some(byte_idx);
        }
        visible_offset += ch.len_utf8();
    }

    (visible_offset == target_visible_offset).then_some(raw.len())
}

fn prepend_run_boundary(chunk: &str) -> String {
    if chunk.is_empty() || chunk.starts_with('\n') || chunk.starts_with("\r\n") {
        return chunk.to_string();
    }
    format!("\r\n{chunk}")
}

fn logicalize_prepended_run_boundary(chunk: &str) -> String {
    chunk
        .strip_prefix("\r\n")
        .map(|rest| format!("\n{rest}"))
        .unwrap_or_else(|| chunk.to_string())
}

fn append_disconnected_restore_cleanup(chunk: &str) -> String {
    let mut out = String::with_capacity(chunk.len() + 32);
    out.push_str(chunk);
    out.push_str("\x1b]1;Chatminal\x07");
    out.push_str("\x1b]2;Chatminal\x07");
    out.push_str("\x1b[?25h");
    out
}

fn strip_zsh_prompt_spacer_artifact(value: &str) -> String {
    const PREFIX: &str = "\u{1b}[1m\u{1b}[7m%\u{1b}[27m\u{1b}[1m\u{1b}[0m";
    let bytes = value.as_bytes();
    let prefix = PREFIX.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(prefix) {
            let mut j = i + prefix.len();
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            if bytes.get(j) == Some(&b'\r') {
                j += 1;
                if bytes.get(j) == Some(&b' ') {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'\r') {
                    i = j + 1;
                    continue;
                }
            }
        }

        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).to_string()
}

fn kill_runtime_handle(runtime: Option<RuntimeHandle>) {
    if let Some(runtime) = runtime {
        kill_runtime_handles(vec![runtime]);
    }
}

fn kill_runtime_handles(runtimes: Vec<RuntimeHandle>) {
    for runtime in runtimes {
        if let Ok(mut runtime) = runtime.lock() {
            runtime.kill();
        }
    }
}

#[cfg(test)]
mod tests;
