use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use portable_pty::CommandBuilder;
use store::StoredSessionStatus;
use terminal_emulator::TerminalSize;

use crate::api::{RuntimeSessionBridgeAction, RuntimeSessionLookup as RuntimeOwnedSessionLookup};
use crate::execution::{
    SessionBridgeAction, SessionCoreState, SessionEngineShared, SessionEventBus,
    SessionRuntimeBridge, SessionRuntimeEvent, SessionRuntimeLookup, SessionWorkspaceHost,
    StatefulSessionEngine,
};
use crate::session::{InputWriteStats, SessionEvent, WriteInputError};
use crate::workspace_ids::{RuntimeId, TerminalInstanceId};
use crate::workspace_layout::WorkspaceLayoutRegistry;

use super::{RuntimeState, StateInner, canonical_scrollback::build_logical_snapshot};

pub(super) type RuntimeHandle = Arc<Mutex<dyn RuntimeSessionHandle>>;

pub(super) trait RuntimeSessionHandle: Send + Sync + std::fmt::Debug {
    fn write_input(&self, data: &str) -> Result<InputWriteStats, WriteInputError>;
    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), String>;
    fn kill(&mut self);
    #[cfg_attr(not(test), allow(dead_code))]
    fn size(&self) -> Result<(usize, usize), String>;
}

#[derive(Default)]
struct RuntimeExecutionEventBus;

impl SessionEventBus for RuntimeExecutionEventBus {
    fn publish(&self, event: SessionRuntimeEvent) {
        log::trace!("runtime execution event: {:?}", event);
    }
}

struct RuntimeStateHost<'a>(&'a RuntimeState);

impl SessionWorkspaceHost for RuntimeStateHost<'_> {
    fn active_session_id(&self) -> Option<String> {
        self.0
            .workspace_load_passive()
            .ok()
            .and_then(|workspace| workspace.active_session_id)
    }

    fn activate_session(&self, session_id: &str) -> Result<(), String> {
        self.0.session_focus(session_id)
    }
}

#[derive(Debug)]
struct ExecutionSessionHandle {
    shared: Arc<SessionEngineShared>,
    runtime_id: RuntimeId,
    terminal_instance_id: TerminalInstanceId,
    size: (usize, usize),
    closed: bool,
}

impl ExecutionSessionHandle {
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

impl RuntimeSessionHandle for ExecutionSessionHandle {
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

#[derive(Clone, Debug)]
pub(super) struct RuntimeExecution {
    shared: Arc<SessionEngineShared>,
}

impl RuntimeExecution {
    pub(super) fn new() -> Self {
        Self {
            shared: Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
                SessionCoreState::default(),
            )))),
        }
    }

    pub(super) fn shared(&self) -> Arc<SessionEngineShared> {
        Arc::clone(&self.shared)
    }

    pub(super) fn connect_session_events(&self, events_tx: std_mpsc::SyncSender<SessionEvent>) {
        let subscription = self.shared.subscribe();
        thread::spawn(move || {
            while let Ok(event) = subscription.recv_timeout(std::time::Duration::from_millis(50)) {
                match event {
                    Some(event) => {
                        if let Some(mapped) = map_execution_event(event) {
                            let _ = events_tx.send(mapped);
                        }
                    }
                    None => {}
                }
            }
        });
    }

    pub(super) fn spawn_handle(
        &self,
        session_id: &str,
        generation: u64,
        shell: &str,
        cwd: &str,
        cols: usize,
        rows: usize,
        initial_scrollback: Option<String>,
    ) -> Result<RuntimeHandle, String> {
        let mut command = CommandBuilder::new(shell);
        command.cwd(cwd);
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
        Ok(Arc::new(Mutex::new(ExecutionSessionHandle::new(
            Arc::clone(&self.shared),
            state.snapshot.runtime_id,
            terminal_instance_id,
            cols,
            rows,
        ))))
    }

    pub(super) fn workspace_layouts(&self) -> Arc<Mutex<WorkspaceLayoutRegistry>> {
        self.shared.workspace_layouts()
    }

    pub(super) fn attachment(&self, session_id: &str) -> Option<(RuntimeId, TerminalInstanceId)> {
        let core = self.shared.core_state();
        let core = core.lock().ok()?;
        let runtime_id = core.runtime_id_for_session(session_id)?;
        let terminal_instance_id = core.runtime(runtime_id)?.active_terminal_instance_id?;
        Some((runtime_id, terminal_instance_id))
    }

    pub(super) fn reconcile_session_lookup(
        &self,
        host: &RuntimeState,
        lookup: &RuntimeOwnedSessionLookup,
    ) -> Result<RuntimeSessionBridgeAction, String> {
        let bus = RuntimeExecutionEventBus;
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

    pub(super) fn notify_session_activated(
        &self,
        host: &RuntimeState,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Result<(), String> {
        let bus = RuntimeExecutionEventBus;
        SessionRuntimeBridge::new(&RuntimeStateHost(host), &bus)
            .on_session_activated(session_id, runtime_id)?;
        host.mark_session_running_and_publish(session_id)
    }

    pub(super) fn notify_session_closed(
        &self,
        host: &RuntimeState,
        session_id: &str,
        runtime_id: RuntimeId,
        lookup_after_close: &RuntimeOwnedSessionLookup,
    ) -> Result<(), String> {
        let bus = RuntimeExecutionEventBus;
        SessionRuntimeBridge::new(&RuntimeStateHost(host), &bus).on_session_closed(
            session_id,
            runtime_id,
            &into_execution_lookup(lookup_after_close),
        )
    }
}

fn into_execution_lookup(lookup: &RuntimeOwnedSessionLookup) -> SessionRuntimeLookup {
    SessionRuntimeLookup {
        active_session_id: lookup.active_session_id.clone(),
        last_active_session_id: lookup.last_active_session_id.clone(),
        runtime_ids_by_session: lookup.runtime_ids_by_session.clone(),
    }
}

fn map_execution_event(event: SessionRuntimeEvent) -> Option<SessionEvent> {
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
            ts: event_timestamp_millis(),
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
        } => Some(SessionEvent::Exited {
            session_id,
            generation,
            exit_code: None,
            reason: message,
        }),
        _ => None,
    }
}

fn event_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

impl RuntimeState {
    pub(super) fn spawn_runtime_handle(
        &self,
        session_id: &str,
        generation: u64,
        shell: &str,
        cwd: &str,
        cols: usize,
        rows: usize,
    ) -> Result<RuntimeHandle, String> {
        let initial_scrollback = self
            .session_restore_snapshot_get(session_id)
            .ok()
            .map(|snapshot| snapshot.content)
            .filter(|content| !content.is_empty());
        self.execution.spawn_handle(
            session_id,
            generation,
            shell,
            cwd,
            cols,
            rows,
            initial_scrollback,
        )
    }

    pub fn reconcile_session_lookup(
        &self,
        lookup: &RuntimeOwnedSessionLookup,
    ) -> Result<RuntimeSessionBridgeAction, String> {
        self.execution.reconcile_session_lookup(self, lookup)
    }

    pub fn notify_session_activated(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Result<(), String> {
        self.execution
            .notify_session_activated(self, session_id, runtime_id)
    }

    pub fn notify_session_closed(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        lookup_after_close: &RuntimeOwnedSessionLookup,
    ) -> Result<(), String> {
        self.execution
            .notify_session_closed(self, session_id, runtime_id, lookup_after_close)
    }

    pub fn session_runtime_attachment(
        &self,
        session_id: &str,
    ) -> Option<(RuntimeId, TerminalInstanceId)> {
        self.execution.attachment(session_id)
    }

    pub fn mark_session_running_and_publish(&self, session_id: &str) -> Result<(), String> {
        let store = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let Some(entry) = inner.sessions.get(session_id) else {
                return Err("session not found".to_string());
            };
            if entry.session.status == StoredSessionStatus::Running {
                return Ok(());
            }
            inner.store.clone()
        };
        let logical_snapshot = build_logical_snapshot(&store, session_id)?;

        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;
        inner.apply_logical_snapshot_for_running(session_id, logical_snapshot)
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

    pub(super) fn apply_logical_snapshot_for_running(
        &mut self,
        session_id: &str,
        logical_snapshot: super::canonical_scrollback::LogicalSnapshot,
    ) -> Result<(), String> {
        let prepend_run_boundary_on_next_output = !logical_snapshot.open_fragment.is_empty();
        let restored_trailing_fragment = (!logical_snapshot.open_fragment.is_empty())
            .then(|| logical_snapshot.open_fragment.clone());
        let Some(entry) = self.sessions.get_mut(session_id) else {
            return Err("session not found".to_string());
        };
        if entry.session.status == StoredSessionStatus::Running {
            return Ok(());
        }
        let canonical_open_fragment = logical_snapshot.open_fragment;
        let canonical_cursor_col = canonical_open_fragment.chars().count();
        entry.session.status = StoredSessionStatus::Running;
        entry.canonical_open_fragment = canonical_open_fragment;
        entry.canonical_cursor_col = canonical_cursor_col;
        entry.canonical_pending_carriage_return = false;
        entry.prepend_run_boundary_on_next_output = prepend_run_boundary_on_next_output;
        entry.restored_trailing_fragment = restored_trailing_fragment;
        self.store
            .set_session_status(session_id, StoredSessionStatus::Running)?;
        self.publish_session_updated_for(session_id);
        Ok(())
    }
}
