// Runtime execution adapter — abstraction boundary between `chatminal-runtime`
// and the actual session execution engine (`chatminal-session-runtime`).
//
// `DaemonState` depends only on this trait. The concrete implementation lives in
// `desktop_host_runtime` (which depends on `chatminal-session-runtime`).
//
// Phase 07: this file was rewritten to remove the direct `chatminal-session-runtime`
// dependency. All engine types are hidden behind `RuntimeExecutionAdapter`.

use std::sync::{Arc, Mutex};

use crate::api::{RuntimeSessionBridgeAction, RuntimeSessionLookup as RuntimeOwnedSessionLookup};
use crate::session::{InputWriteStats, WriteInputError};
use crate::workspace_ids::{RuntimeId, TerminalInstanceId};
use crate::workspace_layout::WorkspaceLayoutRegistry;

use super::{DaemonState, StateInner};

// ─── handle ────────────────────────────────────────────────────────────────

pub(super) type RuntimeHandle = Arc<Mutex<dyn RuntimeSessionHandleTrait>>;

/// Interface for a live session PTY handle owned by `SessionEntry`.
/// The concrete type is `RuntimeSessionHandle` in `desktop_host_runtime`.
pub trait RuntimeSessionHandleTrait: Send + Sync + std::fmt::Debug {
    fn write_input(&self, data: &str) -> Result<InputWriteStats, WriteInputError>;
    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), String>;
    fn kill(&mut self);
    fn size(&self) -> Result<(usize, usize), String>;
}

// ─── execution adapter trait ────────────────────────────────────────────────

/// Adapter between `DaemonState` and the session execution engine.
///
/// This trait is implemented by `DesktopRuntimeExecutionBridge` in
/// `desktop_host_runtime`. It is the only gateway through which
/// `chatminal-runtime` reaches session engine internals.
pub trait RuntimeExecutionAdapter: Send + Sync + std::fmt::Debug {
    /// Called by `DaemonState::new` to give the adapter a sender it can use to
    /// forward PTY output events back into the state event loop.
    fn connect_session_events(
        &self,
        events_tx: std::sync::mpsc::SyncSender<crate::session::SessionEvent>,
    );

    /// Spawn a new session PTY and return a handle.
    fn spawn_handle(
        &self,
        session_id: &str,
        generation: u64,
        shell: &str,
        cwd: &str,
        cols: usize,
        rows: usize,
    ) -> Result<RuntimeHandle, String>;

    /// Return the shared workspace layout registry.
    fn workspace_layouts(&self) -> Arc<Mutex<WorkspaceLayoutRegistry>>;

    /// Return (runtime_id, terminal_instance_id) for a running session, if any.
    fn attachment(&self, session_id: &str) -> Option<(RuntimeId, TerminalInstanceId)>;

    /// Reconcile the desktop session lookup with the runtime's authoritative state.
    fn reconcile_session_lookup(
        &self,
        host: &DaemonState,
        lookup: &RuntimeOwnedSessionLookup,
    ) -> Result<RuntimeSessionBridgeAction, String>;

    /// Notify the engine that a session was activated.
    fn notify_session_activated(
        &self,
        host: &DaemonState,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Result<(), String>;

    /// Notify the engine that a session was closed.
    fn notify_session_closed(
        &self,
        host: &DaemonState,
        session_id: &str,
        runtime_id: RuntimeId,
        lookup_after_close: &RuntimeOwnedSessionLookup,
    ) -> Result<(), String>;
}

// ─── DaemonState impl ──────────────────────────────────────────────────────

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
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(entry) = inner.sessions.get_mut(session_id) {
                entry.execution_status = crate::state::SessionExecutionStatus::Running {
                    runtime_id: runtime_id.as_u64(),
                };
            }
        }
        self.execution
            .notify_session_activated(self, session_id, runtime_id)
    }

    pub fn notify_session_closed(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        lookup_after_close: &RuntimeOwnedSessionLookup,
    ) -> Result<(), String> {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(entry) = inner.sessions.get_mut(session_id) {
                entry.execution_status = crate::state::SessionExecutionStatus::Stopped;
            }
        }
        self.execution
            .notify_session_closed(self, session_id, runtime_id, lookup_after_close)
    }

    pub fn session_runtime_attachment(
        &self,
        session_id: &str,
    ) -> Option<(RuntimeId, TerminalInstanceId)> {
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
