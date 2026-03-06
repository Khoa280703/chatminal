use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use chatminal_protocol::{
    ClientFrame, CreateSessionResponse, LifecyclePreferences, PingResponse, Request, Response,
    ServerFrame, SessionInfo, SessionSnapshot, SessionStatus, WorkspaceState,
};
use chatminal_store::{Store, StoredSession};

use crate::config::{DaemonConfig, resolve_session_cwd};
use crate::metrics::{RuntimeMetrics, RuntimeMetricsSnapshot};
use crate::session::{SessionEvent, SessionRuntime};

mod explorer_utils;
mod request_handler;
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
    runtime: Option<SessionRuntime>,
    live_output: String,
    generation: u64,
}

struct StateInner {
    config: DaemonConfig,
    store: Store,
    metrics: RuntimeMetrics,
    sessions: HashMap<String, SessionEntry>,
    clients: HashMap<u64, std_mpsc::SyncSender<ServerFrame>>,
    shutdown_requested: bool,
}

#[derive(Clone)]
pub struct DaemonState {
    inner: Arc<Mutex<StateInner>>,
    events: std_mpsc::SyncSender<SessionEvent>,
    metrics: RuntimeMetrics,
}

impl DaemonState {
    pub fn new(config: DaemonConfig, store: Store) -> Result<Self, String> {
        let (events_tx, events_rx) = std_mpsc::sync_channel::<SessionEvent>(4096);
        let metrics = RuntimeMetrics::new();
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
                metrics: metrics.clone(),
                sessions,
                clients: HashMap::new(),
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

    pub fn handle_request(&self, frame: ClientFrame) -> ServerFrame {
        self.metrics.inc_requests_total();
        let id = frame.id;
        let mut inner = match self.inner.lock() {
            Ok(value) => value,
            Err(_) => {
                self.metrics.inc_request_errors_total();
                return ServerFrame::err(id, "state lock poisoned".to_string());
            }
        };

        let result = inner.handle_request(frame.request, self.events.clone());
        match result {
            Ok(response) => ServerFrame::ok(id, response),
            Err(err) => {
                self.metrics.inc_request_errors_total();
                ServerFrame::err(id, err)
            }
        }
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
mod tests;
