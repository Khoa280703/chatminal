// DesktopRuntimeExecutionBridge — concrete implementation of RuntimeExecutionAdapter.
//
// Phase 07: execution engine code moved out of `chatminal-runtime/state/runtime_bridge.rs`
// into this file. `chatminal-runtime` now holds only the trait definition; this crate
// holds the engine-coupled implementation.

use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use super::session_engine::{
    SessionBridgeAction, SessionCoreState, SessionEngineShared, SessionEventBus,
    SessionRuntimeBridge, SessionRuntimeEvent, SessionRuntimeLookup, SessionWorkspaceHost,
    StatefulSessionEngine,
};
use chatminal_runtime::workspace_ids::{RuntimeId, TerminalInstanceId};
use chatminal_runtime::workspace_layout::WorkspaceLayoutRegistry;
use chatminal_runtime::{
    InputWriteStats, RuntimeExecutionAdapter, RuntimeSessionBridgeAction,
    RuntimeSessionHandleTrait, RuntimeSessionLookup, RuntimeState, WriteInputError,
};
use chatminal_terminal_core::TerminalSize;
use portable_pty::CommandBuilder;

use crate::chatminal_runtime::read_session_restore_snapshot;

const EXECUTION_EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(50);

// ─── session event bus ──────────────────────────────────────────────────────

#[derive(Default)]
struct DesktopSessionEventBus;

impl SessionEventBus for DesktopSessionEventBus {
    fn publish(&self, event: SessionRuntimeEvent) {
        log::trace!("desktop-execution-bridge event: {:?}", event);
    }
}

// ─── RuntimeStateHost newtype ───────────────────────────────────────────────
// Orphan rule: SessionWorkspaceHost is from desktop_host_runtime::session_engine and
// RuntimeState is from chatminal-runtime — neither is defined here. Use a newtype.

struct RuntimeStateHost<'a>(&'a RuntimeState);

impl SessionWorkspaceHost for RuntimeStateHost<'_> {
    fn active_session_id(&self) -> Option<String> {
        self.0
            .workspace_load_passive()
            .ok()
            .and_then(|workspace| workspace.active_session_id)
    }

    fn activate_session(&self, session_id: &str) -> Result<(), String> {
        // Desktop host already owns live runtime focus. On this bridge path we only
        // need to sync workspace metadata, never spawn or resize the PTY again.
        self.0.session_focus(session_id)
    }
}

// ─── RuntimeSessionHandle impl ─────────────────────────────────────────────

#[derive(Debug)]
pub(crate) struct DesktopSessionHandle {
    shared: Arc<SessionEngineShared>,
    runtime_id: RuntimeId,
    terminal_instance_id: TerminalInstanceId,
    size: (usize, usize),
    closed: bool,
}

impl DesktopSessionHandle {
    fn new(
        shared: Arc<SessionEngineShared>,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        cols: usize,
        rows: usize,
    ) -> Self {
        Self {
            shared,
            runtime_id,
            terminal_instance_id,
            size: (cols, rows),
            closed: false,
        }
    }
}

impl RuntimeSessionHandleTrait for DesktopSessionHandle {
    fn write_input(&self, data: &str) -> Result<InputWriteStats, WriteInputError> {
        self.shared
            .write_terminal_input(self.terminal_instance_id, data.as_bytes())
            .map_err(|_| {
                if self.closed {
                    WriteInputError::Closing
                } else {
                    WriteInputError::Disconnected
                }
            })?;
        Ok(InputWriteStats::default())
    }

    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), String> {
        self.shared.resize_terminal_instance(
            self.terminal_instance_id,
            TerminalSize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
        )?;
        self.size = (cols, rows);
        Ok(())
    }

    fn kill(&mut self) {
        if self.closed {
            return;
        }
        let engine = StatefulSessionEngine::with_shared(Arc::clone(&self.shared));
        let _ = engine.close_detached_runtime(self.runtime_id);
        self.closed = true;
    }

    fn size(&self) -> Result<(usize, usize), String> {
        if self.closed {
            return Err("session runtime is closed".to_string());
        }
        Ok(self.size)
    }
}

// ─── DesktopRuntimeExecutionBridge ─────────────────────────────────────────

#[derive(Debug)]
pub(crate) struct DesktopRuntimeExecutionBridge {
    shared: Arc<SessionEngineShared>,
    workspace_layouts: Arc<Mutex<WorkspaceLayoutRegistry>>,
    /// Populated by `connect_session_events` called from `RuntimeState::new`.
    events_tx: OnceLock<std_mpsc::SyncSender<chatminal_runtime::SessionEvent>>,
}

impl DesktopRuntimeExecutionBridge {
    pub(crate) fn new() -> Self {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let workspace_layouts = Arc::new(Mutex::new(WorkspaceLayoutRegistry::default()));
        Self {
            shared,
            workspace_layouts,
            events_tx: OnceLock::new(),
        }
    }

    pub(crate) fn shared(&self) -> Arc<SessionEngineShared> {
        Arc::clone(&self.shared)
    }
}

impl RuntimeExecutionAdapter for DesktopRuntimeExecutionBridge {
    fn connect_session_events(
        &self,
        events_tx: std_mpsc::SyncSender<chatminal_runtime::SessionEvent>,
    ) {
        // OnceLock: only set once — safe in multi-threaded context.
        let _ = self.events_tx.set(events_tx.clone());
        let subscription = self.shared.subscribe();
        thread::spawn(move || loop {
            match subscription.recv_timeout(EXECUTION_EVENT_POLL_TIMEOUT) {
                Ok(Some(event)) => {
                    if let Some(mapped) = map_execution_event(event) {
                        let _ = events_tx.send(mapped);
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    log::warn!("desktop execution bridge stopped: {err}");
                    break;
                }
            }
        });
    }

    fn spawn_handle(
        &self,
        session_id: &str,
        generation: u64,
        shell: &str,
        cwd: &str,
        cols: usize,
        rows: usize,
    ) -> Result<Arc<Mutex<dyn RuntimeSessionHandleTrait>>, String> {
        let mut command = CommandBuilder::new(shell);
        command.cwd(cwd);
        let initial_scrollback = read_session_restore_snapshot(session_id)
            .map(|snapshot| snapshot.content)
            .ok()
            .filter(|content| !content.is_empty());
        let engine = StatefulSessionEngine::with_shared(Arc::clone(&self.shared));
        let state = engine.spawn_detached_runtime(
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
            initial_scrollback,
        )?;
        let terminal_instance_id = state.snapshot.active_terminal_instance_id.ok_or_else(|| {
            "spawned session runtime missing active terminal instance".to_string()
        })?;
        Ok(Arc::new(Mutex::new(DesktopSessionHandle::new(
            Arc::clone(&self.shared),
            state.snapshot.runtime_id,
            terminal_instance_id,
            cols,
            rows,
        ))))
    }

    fn workspace_layouts(&self) -> Arc<Mutex<WorkspaceLayoutRegistry>> {
        Arc::clone(&self.workspace_layouts)
    }

    fn attachment(&self, session_id: &str) -> Option<(RuntimeId, TerminalInstanceId)> {
        let core = self.shared.core_state();
        let core = core.lock().ok()?;
        let runtime_id = core.runtime_id_for_session(session_id)?;
        let terminal_instance_id = core.runtime(runtime_id)?.active_terminal_instance_id?;
        Some((runtime_id, terminal_instance_id))
    }

    fn reconcile_session_lookup(
        &self,
        host: &RuntimeState,
        lookup: &RuntimeSessionLookup,
    ) -> Result<RuntimeSessionBridgeAction, String> {
        let bus = DesktopSessionEventBus;
        Ok(
            match SessionRuntimeBridge::new(&RuntimeStateHost(host), &bus)
                .reconcile_session_lookup(&into_execution_lookup(lookup))?
            {
                SessionBridgeAction::Noop => RuntimeSessionBridgeAction::Noop,
                SessionBridgeAction::FocusSession { session_id } => {
                    RuntimeSessionBridgeAction::FocusSession { session_id }
                }
            },
        )
    }

    fn notify_session_activated(
        &self,
        host: &RuntimeState,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Result<(), String> {
        let bus = DesktopSessionEventBus;
        SessionRuntimeBridge::new(&RuntimeStateHost(host), &bus)
            .on_session_activated(session_id, runtime_id)?;
        host.mark_session_running_and_publish(session_id)
    }

    fn notify_session_closed(
        &self,
        host: &RuntimeState,
        session_id: &str,
        runtime_id: RuntimeId,
        lookup_after_close: &RuntimeSessionLookup,
    ) -> Result<(), String> {
        let bus = DesktopSessionEventBus;
        SessionRuntimeBridge::new(&RuntimeStateHost(host), &bus).on_session_closed(
            session_id,
            runtime_id,
            &into_execution_lookup(lookup_after_close),
        )
    }
}

fn into_execution_lookup(lookup: &RuntimeSessionLookup) -> SessionRuntimeLookup {
    SessionRuntimeLookup {
        active_session_id: lookup.active_session_id.clone(),
        last_active_session_id: lookup.last_active_session_id.clone(),
        runtime_ids_by_session: lookup.runtime_ids_by_session.clone(),
    }
}

fn map_execution_event(event: SessionRuntimeEvent) -> Option<chatminal_runtime::SessionEvent> {
    use chatminal_runtime::SessionEvent;
    match event {
        SessionRuntimeEvent::TerminalInstanceOutput {
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
        SessionRuntimeEvent::TerminalInstanceExited {
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
        SessionRuntimeEvent::TerminalInstanceError {
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

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}
