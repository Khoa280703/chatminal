use chatminal_terminal_core::TerminalSize;
use portable_pty::CommandBuilder;

use crate::{
    SessionEventBus, SessionLayoutSnapshot, SessionRuntimeEvent, SessionSurfaceState,
    StatefulSessionEngine, SurfaceId,
};

impl<A> StatefulSessionEngine<A> {
    pub fn spawn_detached_surface(
        &self,
        session_id: impl Into<String>,
        generation: u64,
        command: CommandBuilder,
        size: TerminalSize,
    ) -> Result<SessionSurfaceState, String> {
        let session_id = session_id.into();
        let surface_id = self.core_id_allocator().next_surface_id();
        let leaf_id = self.core_id_allocator().next_leaf_id();
        let layout = SessionLayoutSnapshot::single_leaf(
            self.core_id_allocator().next_layout_node_id(),
            leaf_id,
            None,
        );
        {
            let core_state = self.core_state_handle();
            core_state
                .lock()
                .unwrap()
                .sync_surface_layout(session_id.clone(), surface_id, &layout);
        }
        self.leaf_runtime_registry().spawn_for_surface(
            &self.core_state_handle(),
            session_id.clone(),
            generation,
            surface_id,
            leaf_id,
            command,
            size,
            self.leaf_runtime_events_tx(),
        )?;
        self.event_hub()
            .publish(SessionRuntimeEvent::SurfaceAttached {
                session_id: session_id.clone(),
                surface_id,
            });

        let mut state = SessionSurfaceState::detached(session_id, surface_id);
        state.attach_layout(layout);
        Ok(state)
    }

    pub fn close_detached_surface(&self, surface_id: SurfaceId) -> bool {
        let leaf_ids = self
            .core_state_handle()
            .lock()
            .unwrap()
            .surface(surface_id)
            .map(|surface| surface.leaves.keys().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        for leaf_id in leaf_ids {
            let _ = self.leaf_runtime_registry().remove_for_surface(
                &self.core_state_handle(),
                surface_id,
                leaf_id,
            );
        }
        self.core_state_handle()
            .lock()
            .unwrap()
            .remove_surface(surface_id)
            .is_some()
    }
}

#[cfg(test)]
#[path = "session_engine_core_tests.rs"]
mod tests;
