use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use chatminal_store::{Store, StoredSession, StoredSessionSnapshot, StoredSessionStatus};

use crate::api::{
    RuntimeCreatedSession, RuntimeEvent, RuntimeProfile, RuntimeSessionSnapshot, RuntimeWorkspace,
};
use crate::config::{DaemonConfig, resolve_session_cwd};
use crate::metrics::{RuntimeMetrics, RuntimeMetricsSnapshot};
use crate::session::{SessionEvent, SessionRuntime, WriteInputError};

mod explorer_utils;
mod native_api;
mod protocol_adapter;
mod protocol_clients;
mod runtime_lifecycle;
mod session_event_processor;
mod session_explorer;

const MAX_INPUT_BYTES: usize = 65_536;
const KEEP_ALIVE_ON_CLOSE_KEY: &str = "keep_alive_on_close";
const START_IN_TRAY_KEY: &str = "start_in_tray";
const DEFAULT_KEEP_ALIVE_ON_CLOSE: bool = true;
const DEFAULT_START_IN_TRAY: bool = false;

struct SessionEntry {
    session: StoredSession,
    runtime: Option<RuntimeHandle>,
    live_output: String,
    generation: u64,
}

type RuntimeHandle = Arc<Mutex<SessionRuntime>>;

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
    config: DaemonConfig,
    store: Store,
    metrics: RuntimeMetrics,
    sessions: HashMap<String, SessionEntry>,
    protocol_clients: protocol_clients::ProtocolClients,
    subscribers: HashMap<u64, std_mpsc::SyncSender<RuntimeEvent>>,
    next_subscriber_id: u64,
    shutdown_requested: bool,
}

#[derive(Clone)]
pub struct DaemonState {
    inner: Arc<Mutex<StateInner>>,
    events: std_mpsc::SyncSender<SessionEvent>,
    metrics: RuntimeMetrics,
}

pub struct RuntimeSubscription {
    state: DaemonState,
    subscriber_id: u64,
    rx: std_mpsc::Receiver<RuntimeEvent>,
}

impl RuntimeSubscription {
    fn new(state: DaemonState, subscriber_id: u64, rx: std_mpsc::Receiver<RuntimeEvent>) -> Self {
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

impl DaemonState {
    pub fn initialize_default() -> Result<(Self, DaemonConfig), String> {
        let config = DaemonConfig::from_env()?;
        let store = Store::initialize_default()?;
        let state = Self::new(config.clone(), store)?;
        Ok((state, config))
    }

    pub fn new(config: DaemonConfig, store: Store) -> Result<Self, String> {
        let (events_tx, events_rx) = std_mpsc::sync_channel::<SessionEvent>(4096);
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
                protocol_clients: protocol_clients::ProtocolClients::new(),
                subscribers: HashMap::new(),
                next_subscriber_id: 1,
                shutdown_requested: false,
            })),
            events: events_tx.clone(),
            metrics,
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

    pub fn broadcast_daemon_health(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.broadcast_daemon_health();
        }
    }

    pub fn metrics_snapshot(&self) -> RuntimeMetricsSnapshot {
        self.metrics.snapshot()
    }

    pub fn log_runtime_metrics(&self) {
        let snapshot = self.metrics_snapshot();
        log::info!(
            "runtime-metrics requests_total={} request_errors_total={} session_events_total={} broadcast_frames_total={} dropped_clients_full_total={} dropped_clients_disconnected_total={} input_queue_full_total={} input_retry_total={} input_drop_total={}",
            snapshot.requests_total,
            snapshot.request_errors_total,
            snapshot.session_events_total,
            snapshot.broadcast_frames_total,
            snapshot.dropped_clients_full_total,
            snapshot.dropped_clients_disconnected_total,
            snapshot.input_queue_full_total,
            snapshot.input_retry_total,
            snapshot.input_drop_total
        );
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
        store.set_active_session(&active_profile_id, Some(&created.session_id))?;

        let runtime = match spawn_runtime_handle(
            &created.session_id,
            0,
            &created.shell,
            &created.cwd,
            cols,
            rows,
            self.events.clone(),
        ) {
            Ok(runtime) => runtime,
            Err(err) => {
                let _ = store.delete_session(&created.session_id);
                return Err(err);
            }
        };

        let mut entry = SessionEntry {
            session: created.clone(),
            runtime: Some(runtime),
            live_output: String::new(),
            generation: 0,
        };
        entry.session.status = StoredSessionStatus::Running;

        {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            inner
                .store
                .set_session_status(&created.session_id, StoredSessionStatus::Running)?;
            inner.sessions.insert(created.session_id.clone(), entry);
            inner.publish_session_updated_for(&created.session_id);
            inner.publish_workspace_updated();
        }

        Ok(RuntimeCreatedSession {
            session_id: created.session_id,
            name: created.name,
        })
    }

    pub fn session_activate(
        &self,
        session_id: &str,
        cols: usize,
        rows: usize,
    ) -> Result<(), String> {
        enum Activation {
            Existing(RuntimeHandle, String),
            Spawn(SessionSpawnPlan),
        }

        let activation = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let Some(entry) = inner.sessions.get(session_id) else {
                return Err("session not found".to_string());
            };
            if let Some(runtime) = entry.runtime.clone() {
                Activation::Existing(runtime, entry.session.profile_id.clone())
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
                inner
                    .store
                    .set_active_session(&profile_id, Some(session_id))?;
                inner.publish_session_updated_for(session_id);
                inner.publish_workspace_updated();
                Ok(())
            }
            Activation::Spawn(plan) => self.commit_spawned_session(plan),
        }
    }

    pub fn session_snapshot_get(
        &self,
        session_id: &str,
        preview_lines: Option<usize>,
    ) -> Result<RuntimeSessionSnapshot, String> {
        let (store, default_preview_lines, live_overlay) = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let Some(entry) = inner.sessions.get(session_id) else {
                return Err("session not found".to_string());
            };

            let live_overlay = if entry.live_output.is_empty() || entry.session.persist_history {
                None
            } else {
                Some((entry.live_output.clone(), entry.session.seq))
            };

            (
                inner.store.clone(),
                inner.config.default_preview_lines,
                live_overlay,
            )
        };

        let from_store =
            store.session_snapshot(session_id, preview_lines.unwrap_or(default_preview_lines))?;
        let merged = if let Some((live_output, seq)) = live_overlay {
            StoredSessionSnapshot {
                content: format!("{}{}", from_store.content, live_output),
                seq: seq.max(from_store.seq),
            }
        } else {
            from_store
        };

        Ok(merged.into())
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

        let runtime = runtime
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
            inner.store.delete_session(session_id)?;
            let runtime = inner
                .sessions
                .remove(session_id)
                .and_then(|mut entry| entry.runtime.take());
            inner.publish_workspace_updated();
            runtime
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
            inner.store.clear_session_history(session_id)?;
            let mut runtime = None;
            if let Some(entry) = inner.sessions.get_mut(session_id) {
                entry.generation = entry.generation.saturating_add(1);
                entry.session.seq = 0;
                entry.live_output.clear();
                runtime = entry.runtime.take();
                entry.session.status = StoredSessionStatus::Disconnected;
                inner
                    .store
                    .set_session_status(session_id, StoredSessionStatus::Disconnected)?;
            }
            inner.publish_session_updated_for(session_id);
            inner.publish_workspace_updated();
            runtime
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
            let mut updated_ids = Vec::new();
            let mut status_ids = Vec::new();
            let mut runtimes = Vec::new();
            for entry in inner.sessions.values_mut() {
                entry.generation = entry.generation.saturating_add(1);
                entry.session.seq = 0;
                entry.live_output.clear();
                if let Some(runtime) = entry.runtime.take() {
                    runtimes.push(runtime);
                }
                entry.session.status = StoredSessionStatus::Disconnected;
                status_ids.push(entry.session.session_id.clone());
                updated_ids.push(entry.session.session_id.clone());
            }
            for session_id in &status_ids {
                let _ = inner
                    .store
                    .set_session_status(session_id, StoredSessionStatus::Disconnected);
            }
            for session_id in updated_ids {
                inner.publish_session_updated_for(&session_id);
            }
            inner.publish_workspace_updated();
            runtimes
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
            let mut updated_ids = Vec::new();
            let mut status_ids = Vec::new();
            let mut runtimes = Vec::new();
            for entry in inner.sessions.values_mut() {
                if let Some(runtime) = entry.runtime.take() {
                    runtimes.push(runtime);
                }
                entry.session.status = StoredSessionStatus::Disconnected;
                status_ids.push(entry.session.session_id.clone());
                updated_ids.push(entry.session.session_id.clone());
            }
            for session_id in &status_ids {
                let _ = inner
                    .store
                    .set_session_status(session_id, StoredSessionStatus::Disconnected);
            }
            for session_id in updated_ids {
                inner.publish_session_updated_for(&session_id);
            }
            inner.publish_workspace_updated();
            runtimes
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

fn spawn_runtime_handle(
    session_id: &str,
    generation: u64,
    shell: &str,
    cwd: &str,
    cols: usize,
    rows: usize,
    events: std_mpsc::SyncSender<SessionEvent>,
) -> Result<RuntimeHandle, String> {
    let runtime = SessionRuntime::spawn(
        session_id.to_string(),
        generation,
        shell.to_string(),
        cwd.to_string(),
        cols,
        rows,
        events,
    )?;
    Ok(Arc::new(Mutex::new(runtime)))
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
