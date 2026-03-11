use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chatminal_session_runtime::{
    SessionBridgeAction, SessionCoreState, SessionEngineShared, SessionEventBus,
    SessionRuntimeBridge, SessionRuntimeEvent, SessionSurfaceLookup, SessionWorkspaceHost,
    StatefulSessionEngine, SurfaceId,
};
use chatminal_terminal_core::TerminalSize;
use portable_pty::CommandBuilder;

use crate::session::{InputWriteStats, SessionEvent, WriteInputError};

use super::{DaemonState, StateInner};

const EXECUTION_EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(50);

pub(super) type RuntimeHandle = Arc<Mutex<RuntimeSessionHandle>>;

#[derive(Debug)]
pub(super) struct RuntimeSessionHandle {
    shared: Arc<SessionEngineShared>,
    surface_id: chatminal_session_runtime::SurfaceId,
    leaf_id: chatminal_session_runtime::LeafId,
    size: (usize, usize),
    closed: bool,
}

impl RuntimeSessionHandle {
    fn new(
        shared: Arc<SessionEngineShared>,
        surface_id: chatminal_session_runtime::SurfaceId,
        leaf_id: chatminal_session_runtime::LeafId,
        cols: usize,
        rows: usize,
    ) -> Self {
        Self {
            shared,
            surface_id,
            leaf_id,
            size: (cols, rows),
            closed: false,
        }
    }

    pub(super) fn write_input(&self, data: &str) -> Result<InputWriteStats, WriteInputError> {
        let Some(runtime) = self.shared.leaf_runtimes().runtime(self.leaf_id) else {
            return Err(if self.closed {
                WriteInputError::Closing
            } else {
                WriteInputError::Disconnected
            });
        };
        runtime
            .write_input(data.as_bytes())
            .map_err(|_| WriteInputError::Disconnected)?;
        Ok(InputWriteStats::default())
    }

    pub(super) fn resize(&mut self, cols: usize, rows: usize) -> Result<(), String> {
        let Some(runtime) = self.shared.leaf_runtimes().runtime(self.leaf_id) else {
            return Err("session is not running".to_string());
        };
        runtime.resize(TerminalSize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 96,
        })?;
        self.size = (cols, rows);
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn size(&self) -> Result<(usize, usize), String> {
        if self.closed {
            return Err("session runtime is closed".to_string());
        }
        Ok(self.size)
    }

    pub(super) fn kill(&mut self) {
        if self.closed {
            return;
        }
        let engine = StatefulSessionEngine::with_shared((), Arc::clone(&self.shared));
        let _ = engine.close_detached_surface(self.surface_id);
        self.closed = true;
    }
}

#[derive(Debug)]
pub(super) struct RuntimeExecutionBridge {
    shared: Arc<SessionEngineShared>,
}

impl RuntimeExecutionBridge {
    pub(super) fn new(events: std_mpsc::SyncSender<SessionEvent>) -> Self {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let subscription = shared.event_hub().subscribe();
        thread::spawn(move || loop {
            match subscription.recv_timeout(EXECUTION_EVENT_POLL_TIMEOUT) {
                Ok(Some(event)) => {
                    if let Some(mapped) = map_execution_event(event) {
                        let _ = events.send(mapped);
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    log::warn!("runtime execution bridge stopped: {err}");
                    break;
                }
            }
        });
        Self { shared }
    }

    pub(super) fn spawn_handle(
        &self,
        session_id: &str,
        generation: u64,
        shell: &str,
        cwd: &str,
        cols: usize,
        rows: usize,
    ) -> Result<RuntimeHandle, String> {
        let mut command = CommandBuilder::new(shell);
        command.cwd(cwd);
        let engine = StatefulSessionEngine::with_shared((), Arc::clone(&self.shared));
        let state = engine.spawn_detached_surface(
            session_id.to_string(),
            generation,
            command,
            TerminalSize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
        )?;
        let leaf_id = state
            .snapshot
            .active_leaf_id
            .ok_or_else(|| "spawned session surface missing active leaf".to_string())?;
        Ok(Arc::new(Mutex::new(RuntimeSessionHandle::new(
            Arc::clone(&self.shared),
            state.snapshot.surface_id,
            leaf_id,
            cols,
            rows,
        ))))
    }

    pub(super) fn shared(&self) -> Arc<SessionEngineShared> {
        Arc::clone(&self.shared)
    }

    pub(super) fn attachment(
        &self,
        session_id: &str,
    ) -> Option<(
        chatminal_session_runtime::SurfaceId,
        chatminal_session_runtime::LeafId,
    )> {
        let core = self.shared.core_state();
        let core = core.lock().ok()?;
        let surface_id = core.surface_id_for_session(session_id)?;
        let leaf_id = core.surface(surface_id)?.active_leaf_id?;
        Some((surface_id, leaf_id))
    }
}

fn map_execution_event(event: SessionRuntimeEvent) -> Option<SessionEvent> {
    match event {
        SessionRuntimeEvent::LeafOutput {
            session_id,
            generation,
            chunk,
            ..
        } => Some(SessionEvent::Output {
            session_id,
            generation,
            chunk,
            ts: now_millis(),
        }),
        SessionRuntimeEvent::LeafExited {
            session_id,
            generation,
            exit_code,
            ..
        } => Some(SessionEvent::Exited {
            session_id,
            generation,
            exit_code,
            reason: "eof".to_string(),
        }),
        SessionRuntimeEvent::LeafError {
            session_id,
            generation,
            message,
            ..
        } => Some(SessionEvent::Error {
            session_id,
            generation,
            message,
        }),
        _ => None,
    }
}

#[derive(Default)]
pub(super) struct RuntimeSessionEventBus;

impl SessionEventBus for RuntimeSessionEventBus {
    fn publish(&self, event: SessionRuntimeEvent) {
        log::trace!("session-runtime-bridge event: {:?}", event);
    }
}

impl SessionWorkspaceHost for DaemonState {
    fn active_session_id(&self) -> Option<String> {
        self.workspace_load_passive()
            .ok()
            .and_then(|workspace| workspace.active_session_id)
    }

    fn activate_session(&self, session_id: &str) -> Result<(), String> {
        let (cols, rows) = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            (inner.config.default_cols, inner.config.default_rows)
        };
        self.session_activate(session_id, cols, rows)
    }

    fn close_session(&self, session_id: &str) -> Result<(), String> {
        self.session_close(session_id)
    }
}

impl DaemonState {
    pub(super) fn spawn_runtime_handle(
        &self,
        session_id: &str,
        generation: u64,
        shell: &str,
        cwd: &str,
        cols: usize,
        rows: usize,
    ) -> Result<RuntimeHandle, String> {
        self.execution
            .spawn_handle(session_id, generation, shell, cwd, cols, rows)
    }

    pub fn reconcile_session_surface_lookup(
        &self,
        lookup: &SessionSurfaceLookup,
    ) -> Result<SessionBridgeAction, String> {
        let bus = RuntimeSessionEventBus;
        SessionRuntimeBridge::new(self, &bus).reconcile_lookup(lookup)
    }

    pub fn notify_session_surface_focused(
        &self,
        session_id: &str,
        surface_id: SurfaceId,
    ) -> Result<(), String> {
        let bus = RuntimeSessionEventBus;
        SessionRuntimeBridge::new(self, &bus).on_surface_focused(session_id, surface_id)
    }

    pub fn notify_session_surface_closed(
        &self,
        session_id: &str,
        surface_id: SurfaceId,
        lookup_after_close: &SessionSurfaceLookup,
    ) -> Result<(), String> {
        let bus = RuntimeSessionEventBus;
        SessionRuntimeBridge::new(self, &bus).on_surface_closed(
            session_id,
            surface_id,
            lookup_after_close,
        )
    }

    pub fn session_engine_shared(&self) -> Arc<SessionEngineShared> {
        self.execution.shared()
    }

    pub fn session_runtime_attachment(
        &self,
        session_id: &str,
    ) -> Option<(
        chatminal_session_runtime::SurfaceId,
        chatminal_session_runtime::LeafId,
    )> {
        self.execution.attachment(session_id)
    }
}

impl StateInner {
    pub(super) fn publish_session_and_workspace_updated(&mut self, session_id: &str) {
        self.publish_session_updated_for(session_id);
        self.publish_workspace_updated();
    }

    pub(super) fn set_active_session_and_publish(
        &mut self,
        profile_id: &str,
        session_id: &str,
    ) -> Result<(), String> {
        self.store
            .set_active_session(profile_id, Some(session_id))?;
        self.publish_session_and_workspace_updated(session_id);
        Ok(())
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}
