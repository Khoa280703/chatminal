// Runtime execution adapter — abstraction boundary between `chatminal-runtime`
// and the actual session execution engine (`desktop_host_runtime::session_engine`).
//
// `RuntimeState` depends only on this trait. The concrete implementation lives in
// `desktop_host_runtime` (which depends on `desktop_host_runtime::session_engine`).
//
// Phase 07: this file was rewritten to remove the direct `desktop_host_runtime::session_engine`
// dependency. All engine types are hidden behind `RuntimeExecutionAdapter`.
//
// ID ownership boundary:
// - Runtime (`chatminal-runtime`) only needs: `session_id` (String) + `RuntimeId`
// - Engine/desktop IDs (`PaneId`, `TabId`, `WindowId`, `TerminalInstanceId`) are
//   desktop-only concerns and MUST NOT leak into this crate.
//   See `DesktopSessionHost::session_tab_shim` for the desktop-local mapping.

use std::sync::{Arc, Mutex};

use crate::api::{RuntimeSessionBridgeAction, RuntimeSessionLookup as RuntimeOwnedSessionLookup};
use crate::session::{InputWriteStats, WriteInputError};
use crate::workspace_ids::{RuntimeId, TerminalInstanceId};
use crate::workspace_layout::WorkspaceLayoutRegistry;
use chatminal_store::StoredSessionStatus;

use super::{RuntimeState, StateInner, canonical_scrollback::build_logical_snapshot};

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

/// Adapter between `RuntimeState` and the session execution engine.
///
/// This trait is implemented by `DesktopRuntimeExecutionBridge` in
/// `desktop_host_runtime`. It is the only gateway through which
/// `chatminal-runtime` reaches session engine internals.
pub trait RuntimeExecutionAdapter: Send + Sync + std::fmt::Debug {
    /// Called by `RuntimeState::new` to give the adapter a sender it can use to
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
        host: &RuntimeState,
        lookup: &RuntimeOwnedSessionLookup,
    ) -> Result<RuntimeSessionBridgeAction, String>;

    /// Notify the engine that a session was activated.
    fn notify_session_activated(
        &self,
        host: &RuntimeState,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Result<(), String>;

    /// Notify the engine that a session was closed.
    fn notify_session_closed(
        &self,
        host: &RuntimeState,
        session_id: &str,
        runtime_id: RuntimeId,
        lookup_after_close: &RuntimeOwnedSessionLookup,
    ) -> Result<(), String>;
}

// ─── RuntimeState impl ──────────────────────────────────────────────────────

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
        // Phase 5: moved build_logical_snapshot OUTSIDE global lock.
        // Step 1: briefly acquire lock to check status + clone store.
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
        // Lock released — build snapshot without blocking other operations.
        let logical_snapshot = build_logical_snapshot(&store, session_id)?;

        // Step 2: re-acquire lock to apply state mutations.
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
