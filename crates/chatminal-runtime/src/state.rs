use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::workspace_ids::{SessionViewId, WorkspaceNodeId};
use crate::workspace_layout::{WorkspaceLayoutState, WorkspaceSplitAxis};
use chatminal_store::{Store, StoredSession, StoredSessionSnapshot, StoredSessionStatus};

use crate::api::{
    RuntimeCreatedSession, RuntimeEvent, RuntimeProfile, RuntimeSessionLaunchSpec,
    RuntimeSessionSnapshot, RuntimeWorkspace,
};
use crate::config::{RuntimeConfig, resolve_session_cwd};
use crate::metrics::{RuntimeMetrics, RuntimeMetricsSnapshot};
use crate::session::{SessionEvent, WriteInputError};
use crate::state::runtime_bridge::{RuntimeExecutionAdapter, RuntimeHandle};

mod explorer_utils;
mod native_api;
pub mod runtime_bridge;
mod runtime_lifecycle;
mod session_event_processor;
mod session_explorer;
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
    generation: u64,
    prepend_run_boundary_on_next_output: bool,
    restored_trailing_fragment: Option<String>,
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
}

#[derive(Clone)]
pub struct RuntimeState {
    // `RuntimeState` owns business/workspace state.
    // Live execution state is delegated to the `execution` adapter (concrete impl in
    // `desktop_host_runtime`). No session_engine types leak into this struct.
    inner: Arc<Mutex<StateInner>>,
    metrics: RuntimeMetrics,
    execution: Arc<dyn RuntimeExecutionAdapter>,
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
                        generation: 0,
                        prepend_run_boundary_on_next_output: false,
                        restored_trailing_fragment: None,
                    },
                );
            }
        }

        // Keep disconnected state at startup; clients will reactivate when needed.
        for session_id in sessions.keys() {
            let _ = store.set_session_status(session_id, StoredSessionStatus::Disconnected);
        }

        let state = Self {
            inner: Arc::new(Mutex::new(StateInner {
                config,
                store,
                metrics: metrics.clone(),
                sessions,
                subscribers: HashMap::new(),
                next_subscriber_id: 1,
                shutdown_requested: false,
            })),
            metrics,
            execution,
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
                        ),
                    )
                })
                .collect::<HashMap<_, _>>();
            (inner.store.clone(), session_overrides)
        };

        let mut workspace = store.load_workspace()?;
        for session in &mut workspace.sessions {
            if let Some((status, seq, persist_history, cwd, name)) =
                session_overrides.get(&session.session_id)
            {
                session.status = status.clone();
                session.seq = *seq;
                session.persist_history = *persist_history;
                session.cwd = cwd.clone();
                session.name = name.clone();
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

    pub fn get_lifecycle_preferences(&self) -> Result<crate::api::RuntimeLifecyclePreferences, String> {
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
        let runtimes = {
            let mut inner = match self.inner.lock() {
                Ok(value) => value,
                Err(_) => return,
            };
            inner.shutdown_requested = true;
            inner.disconnect_all_sessions_and_publish(runtime_lifecycle::DisconnectOptions {
                reset_history: false,
                bump_generation: false,
            })
        };

        kill_runtime_handles(runtimes);
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

fn snapshot_requires_run_boundary(snapshot: &StoredSessionSnapshot) -> bool {
    !snapshot.content.is_empty()
        && !snapshot.content.ends_with('\n')
        && !snapshot.content.ends_with('\r')
}

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

fn prepend_run_boundary(chunk: &str) -> String {
    if chunk.is_empty() || chunk.starts_with('\n') || chunk.starts_with("\r\n") {
        return chunk.to_string();
    }
    format!("\r\n{chunk}")
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
