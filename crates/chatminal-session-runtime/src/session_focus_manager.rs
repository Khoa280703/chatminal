use config::keyassignment::PaneDirection;

use crate::LeafId;
use crate::{EngineSurfaceAdapter, SessionSurfaceState, SurfaceId};

#[derive(Clone, Copy, Debug, Default)]
pub struct SessionFocusManager;

impl SessionFocusManager {
    pub fn focus_surface<A: EngineSurfaceAdapter>(
        &self,
        adapter: &A,
        surface_id: SurfaceId,
    ) -> Result<SessionSurfaceState, A::Error> {
        adapter.focus_surface(surface_id)?;
        adapter.snapshot_surface(surface_id)
    }

    pub fn focus_session<A: EngineSurfaceAdapter>(
        &self,
        adapter: &A,
        session_id: &str,
    ) -> Result<SessionSurfaceState, A::Error> {
        let surface = adapter.attach_surface(session_id)?;
        self.focus_surface(adapter, surface.surface_id)
    }

    pub fn focus_leaf<A: EngineSurfaceAdapter>(
        &self,
        adapter: &A,
        surface_id: SurfaceId,
        leaf_id: LeafId,
    ) -> Result<SessionSurfaceState, A::Error> {
        adapter.focus_leaf(surface_id, leaf_id)?;
        adapter.snapshot_surface(surface_id)
    }

    pub fn focus_direction<A: EngineSurfaceAdapter>(
        &self,
        adapter: &A,
        surface_id: SurfaceId,
        direction: PaneDirection,
    ) -> Result<Option<SessionSurfaceState>, A::Error> {
        let Some(target_leaf_id) = adapter.adjacent_active_leaf(surface_id, direction)? else {
            return Ok(None);
        };
        self.focus_leaf(adapter, surface_id, target_leaf_id)
            .map(Some)
    }

    pub fn swap_active_leaf<A: EngineSurfaceAdapter>(
        &self,
        adapter: &A,
        surface_id: SurfaceId,
        leaf_id: LeafId,
        keep_focus: bool,
    ) -> Result<SessionSurfaceState, A::Error> {
        adapter.swap_active_leaf(surface_id, leaf_id, keep_focus)?;
        adapter.snapshot_surface(surface_id)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use config::keyassignment::PaneDirection;
    use window::Window;

    use crate::{
        EngineSurfaceAdapter, EngineSurfaceRef, LeafId, MoveLeafTarget, SessionFocusManager,
        SessionLayoutSnapshot, SessionSurfaceState, SpawnSessionSurfaceRequest, SurfaceId,
    };

    #[derive(Default)]
    struct TestAdapter {
        focused: Mutex<Vec<SurfaceId>>,
    }

    impl EngineSurfaceAdapter for TestAdapter {
        type Error = &'static str;

        fn attach_surface(&self, session_id: &str) -> Result<EngineSurfaceRef, Self::Error> {
            Ok(EngineSurfaceRef {
                surface_id: SurfaceId::new(9),
                session_id: session_id.to_string(),
            })
        }

        fn focus_surface(&self, surface_id: SurfaceId) -> Result<(), Self::Error> {
            self.focused.lock().expect("lock focused").push(surface_id);
            Ok(())
        }

        fn focus_leaf(&self, surface_id: SurfaceId, _leaf_id: LeafId) -> Result<(), Self::Error> {
            self.focused.lock().expect("lock focused").push(surface_id);
            Ok(())
        }

        fn adjacent_active_leaf(
            &self,
            _surface_id: SurfaceId,
            _direction: PaneDirection,
        ) -> Result<Option<LeafId>, Self::Error> {
            Ok(Some(LeafId::new(2)))
        }

        fn swap_active_leaf(
            &self,
            surface_id: SurfaceId,
            _leaf_id: LeafId,
            _keep_focus: bool,
        ) -> Result<(), Self::Error> {
            self.focused.lock().expect("lock focused").push(surface_id);
            Ok(())
        }

        fn move_leaf(
            &self,
            _surface_id: SurfaceId,
            _leaf_id: LeafId,
            _target: MoveLeafTarget,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn close_surface(&self, _surface_id: SurfaceId) -> Result<(), Self::Error> {
            Ok(())
        }

        fn spawn_surface(
            &self,
            _request: SpawnSessionSurfaceRequest,
            _window: Option<Window>,
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
                crate::LeafId::new(2),
                None,
            ));
            Ok(state)
        }
    }

    #[test]
    fn focus_session_returns_latest_snapshot() {
        let adapter = TestAdapter::default();
        let state = SessionFocusManager
            .focus_session(&adapter, "session-a")
            .expect("focus session");

        assert_eq!(
            adapter.focused.lock().expect("lock focused").as_slice(),
            &[SurfaceId::new(9)]
        );
        assert_eq!(state.snapshot.surface_id, SurfaceId::new(9));
        assert_eq!(state.snapshot.active_leaf_id, Some(crate::LeafId::new(2)));
    }

    #[test]
    fn focus_direction_uses_adapter_target_leaf() {
        let adapter = TestAdapter::default();
        let state = SessionFocusManager
            .focus_direction(&adapter, SurfaceId::new(9), PaneDirection::Right)
            .expect("focus direction")
            .expect("direction target");

        assert_eq!(state.snapshot.active_leaf_id, Some(crate::LeafId::new(2)));
        assert_eq!(
            adapter.focused.lock().expect("lock focused").as_slice(),
            &[SurfaceId::new(9)]
        );
    }
}
