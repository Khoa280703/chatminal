use std::sync::{Arc, Mutex};

use config::keyassignment::PaneDirection;
use mux::Mux;
use mux::tab::Tab;
use window::Window;

use crate::{
    ChatminalEngineSurfaceAdapter, EngineSurfaceAdapter, EnsureSurfaceResult, LeafId,
    LeafRuntimeRegistry, MoveLeafTarget, SessionCoreState, SessionEngineShared, SessionEventHub,
    SessionFocusManager, SessionSpawnManager, SessionSurfaceLookup, SessionSurfaceState,
    SpawnSessionSurfaceRequest, SurfaceId,
};

pub trait SessionEngine {
    type Error;
    fn collect_session_surface_lookup(&self) -> SessionSurfaceLookup;
    fn focus_session_state(&self, session_id: &str) -> Result<SessionSurfaceState, Self::Error>;
    fn focus_surface_state(
        &self,
        surface_id: SurfaceId,
    ) -> Result<SessionSurfaceState, Self::Error>;
    fn remove_session_surface(&self, session_id: &str) -> Result<(), Self::Error>;
    fn surface_id_for_session(&self, session_id: &str) -> Option<SurfaceId>;
    fn active_leaf_id(&self, session_id: &str) -> Option<LeafId>;
    fn focus_session_leaf(
        &self,
        session_id: &str,
        leaf_id: LeafId,
    ) -> Result<SessionSurfaceState, Self::Error>;
    fn swap_active_with_session_leaf(
        &self,
        session_id: &str,
        leaf_id: LeafId,
        keep_focus: bool,
    ) -> Result<SessionSurfaceState, Self::Error>;
    fn move_session_leaf(
        &self,
        session_id: &str,
        leaf_id: LeafId,
        target: MoveLeafTarget,
    ) -> Result<(), Self::Error>;
    fn activate_session_direction(
        &self,
        session_id: &str,
        direction: PaneDirection,
    ) -> Result<Option<SessionSurfaceState>, Self::Error>;
    fn ensure_session_surface(
        &self,
        session_id: &str,
        request: SpawnSessionSurfaceRequest,
        window: Option<Window>,
    ) -> Result<EnsureSurfaceResult, Self::Error>;
}

#[derive(Clone, Debug)]
pub struct StatefulSessionEngine<A> {
    adapter: A,
    shared: Arc<SessionEngineShared>,
}

impl<A> StatefulSessionEngine<A> {
    pub fn new(adapter: A, core_state: Arc<Mutex<SessionCoreState>>) -> Self {
        Self {
            adapter,
            shared: Arc::new(SessionEngineShared::new(core_state)),
        }
    }

    pub fn with_shared(adapter: A, shared: Arc<SessionEngineShared>) -> Self {
        Self { adapter, shared }
    }

    pub fn core_state_handle(&self) -> Arc<Mutex<SessionCoreState>> {
        self.shared.core_state()
    }

    pub fn leaf_runtime_registry(&self) -> Arc<LeafRuntimeRegistry> {
        self.shared.leaf_runtimes()
    }

    pub(crate) fn core_id_allocator(&self) -> Arc<crate::session_core_ids::SessionCoreIdAllocator> {
        self.shared.core_ids()
    }

    pub fn shared(&self) -> Arc<SessionEngineShared> {
        Arc::clone(&self.shared)
    }

    pub fn event_hub(&self) -> Arc<SessionEventHub> {
        self.shared.event_hub()
    }

    pub fn replay_leaf_output(&self, leaf_id: LeafId) -> Option<String> {
        self.leaf_runtime_registry().replay_output(leaf_id)
    }

    pub(crate) fn leaf_runtime_events_tx(
        &self,
    ) -> std::sync::mpsc::SyncSender<crate::LeafRuntimeEvent> {
        self.shared.leaf_runtime_events_tx()
    }
}

impl<A: EngineSurfaceAdapter> StatefulSessionEngine<A> {
    fn record_surface_state(&self, state: &SessionSurfaceState) {
        let core_handle = self.core_state_handle();
        let mut core = core_handle.lock().unwrap();
        let surface =
            core.register_surface(state.snapshot.session_id.clone(), state.snapshot.surface_id);
        surface.root_layout_node_id = state.snapshot.root_layout_node_id;
        surface.active_leaf_id = state.snapshot.active_leaf_id;
        if let Some(layout) = &state.layout {
            surface.sync_layout(layout);
        }
    }

    fn remove_surface_from_state(&self, surface_id: SurfaceId) {
        self.core_state_handle()
            .lock()
            .unwrap()
            .remove_surface(surface_id);
    }
}

impl<A: EngineSurfaceAdapter> SessionEngine for StatefulSessionEngine<A> {
    type Error = A::Error;

    fn collect_session_surface_lookup(&self) -> SessionSurfaceLookup {
        let lookup = self.adapter.collect_session_surface_lookup();
        self.core_state_handle()
            .lock()
            .unwrap()
            .reconcile_lookup(&lookup);
        lookup
    }

    fn focus_session_state(&self, session_id: &str) -> Result<SessionSurfaceState, Self::Error> {
        let state = SessionFocusManager.focus_session(&self.adapter, session_id)?;
        self.record_surface_state(&state);
        Ok(state)
    }

    fn focus_surface_state(
        &self,
        surface_id: SurfaceId,
    ) -> Result<SessionSurfaceState, Self::Error> {
        let state = SessionFocusManager.focus_surface(&self.adapter, surface_id)?;
        self.record_surface_state(&state);
        Ok(state)
    }

    fn remove_session_surface(&self, session_id: &str) -> Result<(), Self::Error> {
        let surface_id = self.surface_id_for_session(session_id).or_else(|| {
            self.adapter
                .attach_surface(session_id)
                .map(|surface| surface.surface_id)
                .ok()
        });
        if let Some(surface_id) = surface_id {
            self.adapter.close_surface(surface_id)?;
            self.remove_surface_from_state(surface_id);
        }
        Ok(())
    }

    fn surface_id_for_session(&self, session_id: &str) -> Option<SurfaceId> {
        if let Some(surface_id) = self
            .core_state_handle()
            .lock()
            .unwrap()
            .surface_id_for_session(session_id)
        {
            return Some(surface_id);
        }
        let surface = self.adapter.attach_surface(session_id).ok()?;
        self.core_state_handle()
            .lock()
            .unwrap()
            .register_surface(surface.session_id, surface.surface_id);
        Some(surface.surface_id)
    }

    fn active_leaf_id(&self, session_id: &str) -> Option<LeafId> {
        if let Some(surface_id) = self
            .core_state_handle()
            .lock()
            .unwrap()
            .surface_id_for_session(session_id)
        {
            let active_leaf_id = self
                .core_state_handle()
                .lock()
                .unwrap()
                .surface(surface_id)
                .and_then(|surface| surface.active_leaf_id);
            if active_leaf_id.is_some() {
                return active_leaf_id;
            }
        }
        let surface_id = self.adapter.attach_surface(session_id).ok()?.surface_id;
        let state = self.adapter.snapshot_surface(surface_id).ok()?;
        let active_leaf_id = state.snapshot.active_leaf_id;
        self.record_surface_state(&state);
        active_leaf_id
    }

    fn focus_session_leaf(
        &self,
        session_id: &str,
        leaf_id: LeafId,
    ) -> Result<SessionSurfaceState, Self::Error> {
        let surface = self.adapter.attach_surface(session_id)?;
        let state = SessionFocusManager.focus_leaf(&self.adapter, surface.surface_id, leaf_id)?;
        self.record_surface_state(&state);
        Ok(state)
    }

    fn swap_active_with_session_leaf(
        &self,
        session_id: &str,
        leaf_id: LeafId,
        keep_focus: bool,
    ) -> Result<SessionSurfaceState, Self::Error> {
        let surface = self.adapter.attach_surface(session_id)?;
        let state = SessionFocusManager.swap_active_leaf(
            &self.adapter,
            surface.surface_id,
            leaf_id,
            keep_focus,
        )?;
        self.record_surface_state(&state);
        Ok(state)
    }

    fn move_session_leaf(
        &self,
        session_id: &str,
        leaf_id: LeafId,
        target: MoveLeafTarget,
    ) -> Result<(), Self::Error> {
        let surface = self.adapter.attach_surface(session_id)?;
        self.adapter.move_leaf(surface.surface_id, leaf_id, target)
    }

    fn activate_session_direction(
        &self,
        session_id: &str,
        direction: PaneDirection,
    ) -> Result<Option<SessionSurfaceState>, Self::Error> {
        let surface = self.adapter.attach_surface(session_id)?;
        let state =
            SessionFocusManager.focus_direction(&self.adapter, surface.surface_id, direction)?;
        if let Some(state) = &state {
            self.record_surface_state(state);
        }
        Ok(state)
    }

    fn ensure_session_surface(
        &self,
        session_id: &str,
        request: SpawnSessionSurfaceRequest,
        window: Option<Window>,
    ) -> Result<EnsureSurfaceResult, Self::Error> {
        let result =
            SessionSpawnManager.ensure_surface(&self.adapter, session_id, request, window)?;
        if let EnsureSurfaceResult::FocusedExisting(state) = &result {
            self.record_surface_state(state);
        }
        Ok(result)
    }
}

pub type ChatminalMuxSessionEngine = StatefulSessionEngine<ChatminalEngineSurfaceAdapter>;

impl ChatminalMuxSessionEngine {
    pub fn host_surface_for_session(&self, session_id: &str) -> Option<Arc<Tab>> {
        let host_surface_id = self.adapter.host_surface_id_for_session(session_id)?;
        Mux::get().get_tab(host_surface_id)
    }

    pub fn host_surface_session_id(host_surface: &Arc<Tab>) -> Option<String> {
        ChatminalEngineSurfaceAdapter::host_surface_session_id(host_surface)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use engine_term::TerminalSize;
    use portable_pty::CommandBuilder;
    use window::Window;

    use super::{SessionEngine, StatefulSessionEngine};
    use crate::{
        EngineSurfaceAdapter, EngineSurfaceRef, EnsureSurfaceResult, LeafId, MoveLeafTarget,
        SessionCoreState, SessionLayoutSnapshot, SessionSurfaceState, SpawnSessionSurfaceRequest,
        SurfaceId,
    };

    struct TestAdapter {
        attach_missing: bool,
    }

    impl EngineSurfaceAdapter for TestAdapter {
        type Error = &'static str;
        fn attach_surface(&self, _: &str) -> Result<EngineSurfaceRef, Self::Error> {
            if self.attach_missing {
                Err("surface missing")
            } else {
                Ok(EngineSurfaceRef {
                    surface_id: SurfaceId::new(7),
                    session_id: "session-a".into(),
                })
            }
        }
        fn focus_surface(&self, _: SurfaceId) -> Result<(), Self::Error> {
            Ok(())
        }
        fn focus_leaf(&self, _: SurfaceId, _: LeafId) -> Result<(), Self::Error> {
            Ok(())
        }
        fn adjacent_active_leaf(
            &self,
            _: SurfaceId,
            _: config::keyassignment::PaneDirection,
        ) -> Result<Option<LeafId>, Self::Error> {
            Ok(None)
        }
        fn swap_active_leaf(&self, _: SurfaceId, _: LeafId, _: bool) -> Result<(), Self::Error> {
            Ok(())
        }
        fn move_leaf(&self, _: SurfaceId, _: LeafId, _: MoveLeafTarget) -> Result<(), Self::Error> {
            Ok(())
        }
        fn close_surface(&self, _: SurfaceId) -> Result<(), Self::Error> {
            Ok(())
        }
        fn spawn_surface(
            &self,
            _: SpawnSessionSurfaceRequest,
            _: Option<Window>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
        fn snapshot_surface(
            &self,
            surface_id: SurfaceId,
        ) -> Result<SessionSurfaceState, Self::Error> {
            let mut state = SessionSurfaceState::detached("session-a", surface_id);
            state.attach_layout(SessionLayoutSnapshot::single_leaf(
                crate::LayoutNodeId::new(1),
                LeafId::new(2),
                None,
            ));
            Ok(state)
        }
    }

    fn request() -> SpawnSessionSurfaceRequest {
        SpawnSessionSurfaceRequest {
            session_id: "session-a".into(),
            terminal_size: TerminalSize::default(),
            current_host_leaf_id: None,
            workspace: "default".into(),
            domain: config::keyassignment::SpawnTabDomain::CurrentPaneDomain,
            command: CommandBuilder::new_default_prog(),
        }
    }

    #[test]
    fn facade_records_surface_state_in_core_store() {
        let engine = StatefulSessionEngine::new(
            TestAdapter {
                attach_missing: false,
            },
            Arc::new(Mutex::new(SessionCoreState::default())),
        );
        let state = engine.focus_session_state("session-a").unwrap();
        let core = engine.core_state_handle();
        let core = core.lock().unwrap();
        assert_eq!(state.snapshot.surface_id, SurfaceId::new(7));
        assert_eq!(
            core.surface_id_for_session("session-a"),
            Some(SurfaceId::new(7))
        );
        assert_eq!(
            core.surface(SurfaceId::new(7))
                .and_then(|surface| surface.active_leaf_id),
            Some(LeafId::new(2))
        );
    }

    #[test]
    fn facade_keeps_spawn_scheduled_behavior() {
        let engine = StatefulSessionEngine::new(
            TestAdapter {
                attach_missing: true,
            },
            Arc::new(Mutex::new(SessionCoreState::default())),
        );
        assert!(matches!(
            engine.ensure_session_surface("session-a", request(), None),
            Ok(EnsureSurfaceResult::SpawnScheduled)
        ));
    }
}
