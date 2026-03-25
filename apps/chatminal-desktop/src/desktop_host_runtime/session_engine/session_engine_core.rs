use chatminal_terminal_core::TerminalSize;
use portable_pty::CommandBuilder;

use super::{
    RuntimeId, SessionEventBus, SessionLayoutSnapshot, SessionRuntimeEvent, SessionRuntimeState,
    StatefulSessionEngine, TerminalInstanceId,
};

impl StatefulSessionEngine {
    /// Build a `SessionRuntimeState` snapshot purely from `SessionCoreState`.
    /// Returns `None` if the runtime is not registered in core state.
    pub fn snapshot_runtime_from_core(&self, runtime_id: RuntimeId) -> Option<SessionRuntimeState> {
        let core_handle = self.core_state_handle();
        let core = core_handle.lock().unwrap();
        let runtime = core.runtime(runtime_id)?;
        let mut state = SessionRuntimeState::detached(runtime.session_id.clone(), runtime_id);
        if let Some(layout) = runtime.layout.clone() {
            state.attach_layout(layout);
        }
        Some(state)
    }

    /// Focus a specific leaf natively: updates `SessionCoreState.active_terminal_instance_id` and
    /// publishes `TerminalInstanceFocused` event. Returns the updated runtime snapshot.
    pub fn focus_terminal_instance_native(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<SessionRuntimeState> {
        {
            let core_handle = self.core_state_handle();
            let mut core = core_handle.lock().unwrap();
            let runtime = core.runtime_mut(runtime_id)?;
            if !runtime.leaves.contains_key(&terminal_instance_id) {
                return None;
            }
            // Update layout snapshot's active leaf if present
            if let Some(layout) = runtime.layout.as_mut() {
                layout.active_terminal_instance_id = terminal_instance_id;
            }
            runtime.active_terminal_instance_id = Some(terminal_instance_id);
        }
        self.event_hub()
            .publish(SessionRuntimeEvent::TerminalInstanceFocused {
                session_id: session_id.to_string(),
                runtime_id,
                terminal_instance_id,
            });
        self.snapshot_runtime_from_core(runtime_id)
    }

    /// Focus a runtime natively: restores focus to the known active leaf and
    /// publishes `RuntimeFocused` event. Returns the updated runtime snapshot.
    pub fn focus_runtime_native(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Option<SessionRuntimeState> {
        self.event_hub()
            .publish(SessionRuntimeEvent::RuntimeFocused {
                session_id: session_id.to_string(),
                runtime_id,
            });
        self.snapshot_runtime_from_core(runtime_id)
    }

    /// Close a runtime natively: removes all terminal instance runtimes and runtime from core state,
    /// then publishes `RuntimeClosed` event. Returns `true` if runtime existed.
    pub fn close_runtime_native(&self, session_id: &str, runtime_id: RuntimeId) -> bool {
        let terminal_instance_ids = self
            .core_state_handle()
            .lock()
            .unwrap()
            .runtime(runtime_id)
            .map(|runtime| runtime.leaves.keys().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        for terminal_instance_id in terminal_instance_ids {
            let _ = self.leaf_runtime_registry().remove_for_runtime(
                &self.core_state_handle(),
                runtime_id,
                terminal_instance_id,
            );
        }
        let removed = self
            .core_state_handle()
            .lock()
            .unwrap()
            .remove_runtime(runtime_id)
            .is_some();
        if removed {
            self.event_hub()
                .publish(SessionRuntimeEvent::RuntimeClosed {
                    session_id: session_id.to_string(),
                    runtime_id,
                });
        }
        removed
    }

    /// Ensure a session runtime exists. If it already exists in core state, focus it
    /// natively and return the snapshot. If not, spawn a new detached runtime and
    /// return the initial snapshot. The caller must listen for `RuntimeAttached` to
    /// create render objects (e.g. ChatminalSessionPane) in response.
    pub fn ensure_session_runtime_native(
        &self,
        session_id: &str,
        generation: u64,
        command: CommandBuilder,
        size: TerminalSize,
        initial_scrollback: Option<String>,
    ) -> Result<SessionRuntimeState, String> {
        // Check if runtime already exists in core state
        let existing_runtime_id = self
            .core_state_handle()
            .lock()
            .unwrap()
            .runtime_id_for_session(session_id);

        if let Some(runtime_id) = existing_runtime_id {
            // Focus existing runtime natively
            return self
                .focus_runtime_native(session_id, runtime_id)
                .ok_or_else(|| format!("runtime {runtime_id} missing from core after lookup"));
        }

        // No existing runtime - spawn a new one (native path)
        self.spawn_detached_runtime(session_id, generation, command, size, initial_scrollback)
    }

    pub fn spawn_detached_runtime(
        &self,
        session_id: impl Into<String>,
        generation: u64,
        command: CommandBuilder,
        size: TerminalSize,
        initial_scrollback: Option<String>,
    ) -> Result<SessionRuntimeState, String> {
        let session_id = session_id.into();
        let runtime_id = self.core_id_allocator().next_runtime_id();
        let terminal_instance_id = self.core_id_allocator().next_terminal_instance_id();
        let layout = SessionLayoutSnapshot::single_terminal_instance(
            self.core_id_allocator().next_layout_node_id(),
            terminal_instance_id,
            None,
        );
        {
            let core_state = self.core_state_handle();
            core_state
                .lock()
                .unwrap()
                .sync_runtime_layout(session_id.clone(), runtime_id, &layout);
        }
        self.leaf_runtime_registry().spawn_for_runtime(
            &self.core_state_handle(),
            session_id.clone(),
            generation,
            runtime_id,
            terminal_instance_id,
            command,
            size,
            initial_scrollback,
            self.leaf_runtime_events_tx(),
        )?;
        self.event_hub()
            .publish(SessionRuntimeEvent::RuntimeAttached {
                session_id: session_id.clone(),
                runtime_id,
            });

        let mut state = SessionRuntimeState::detached(session_id, runtime_id);
        state.attach_layout(layout);
        Ok(state)
    }

    pub fn close_detached_runtime(&self, runtime_id: RuntimeId) -> bool {
        let terminal_instance_ids = self
            .core_state_handle()
            .lock()
            .unwrap()
            .runtime(runtime_id)
            .map(|runtime| runtime.leaves.keys().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        for terminal_instance_id in terminal_instance_ids {
            let _ = self.leaf_runtime_registry().remove_for_runtime(
                &self.core_state_handle(),
                runtime_id,
                terminal_instance_id,
            );
        }
        self.core_state_handle()
            .lock()
            .unwrap()
            .remove_runtime(runtime_id)
            .is_some()
    }
}

#[cfg(test)]
#[path = "session_engine_core_tests.rs"]
mod tests;
