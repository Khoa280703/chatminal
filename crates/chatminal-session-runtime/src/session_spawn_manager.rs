use window::Window;

use crate::{
    EngineSurfaceAdapter, SessionFocusManager, SessionSurfaceState, SpawnSessionSurfaceRequest,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnsureSurfaceResult {
    FocusedExisting(SessionSurfaceState),
    SpawnScheduled,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SessionSpawnManager;

impl SessionSpawnManager {
    pub fn ensure_surface<A: EngineSurfaceAdapter>(
        &self,
        adapter: &A,
        session_id: &str,
        request: SpawnSessionSurfaceRequest,
        window: Option<Window>,
    ) -> Result<EnsureSurfaceResult, A::Error> {
        if let Ok(surface) = adapter.attach_surface(session_id) {
            let state = SessionFocusManager.focus_surface(adapter, surface.surface_id)?;
            return Ok(EnsureSurfaceResult::FocusedExisting(state));
        }

        adapter.spawn_surface(request, window)?;
        Ok(EnsureSurfaceResult::SpawnScheduled)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use engine_term::TerminalSize;
    use portable_pty::CommandBuilder;
    use window::Window;

    use crate::{
        EngineSurfaceAdapter, EngineSurfaceRef, EnsureSurfaceResult, LeafId, MoveLeafTarget,
        SessionSpawnManager, SessionSurfaceState, SpawnSessionSurfaceRequest, SurfaceId,
    };

    #[derive(Default)]
    struct TestAdapter {
        attach_result: Mutex<Option<EngineSurfaceRef>>,
        focused: Mutex<Vec<SurfaceId>>,
        spawned: Mutex<usize>,
    }

    impl EngineSurfaceAdapter for TestAdapter {
        type Error = &'static str;

        fn attach_surface(&self, _session_id: &str) -> Result<EngineSurfaceRef, Self::Error> {
            self.attach_result
                .lock()
                .expect("lock attach result")
                .clone()
                .ok_or("surface not found")
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
            _direction: config::keyassignment::PaneDirection,
        ) -> Result<Option<LeafId>, Self::Error> {
            Ok(None)
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
            *self.spawned.lock().expect("lock spawned") += 1;
            Ok(())
        }

        fn snapshot_surface(
            &self,
            surface_id: SurfaceId,
        ) -> Result<SessionSurfaceState, Self::Error> {
            Ok(SessionSurfaceState::detached("session-a", surface_id))
        }
    }

    fn request(session_id: &str) -> SpawnSessionSurfaceRequest {
        SpawnSessionSurfaceRequest {
            session_id: session_id.to_string(),
            terminal_size: TerminalSize::default(),
            current_host_leaf_id: None,
            workspace: "default".to_string(),
            domain: config::keyassignment::SpawnTabDomain::CurrentPaneDomain,
            command: CommandBuilder::new_default_prog(),
        }
    }

    #[test]
    fn ensure_surface_focuses_existing_surface() {
        let adapter = TestAdapter {
            attach_result: Mutex::new(Some(EngineSurfaceRef {
                surface_id: SurfaceId::new(5),
                session_id: "session-a".to_string(),
            })),
            ..TestAdapter::default()
        };

        let result = SessionSpawnManager
            .ensure_surface(&adapter, "session-a", request("session-a"), None)
            .expect("ensure existing surface");

        assert_eq!(
            result,
            EnsureSurfaceResult::FocusedExisting(SessionSurfaceState::detached(
                "session-a",
                SurfaceId::new(5),
            ))
        );
        assert_eq!(
            adapter.focused.lock().expect("lock focused").as_slice(),
            &[SurfaceId::new(5)]
        );
        assert_eq!(*adapter.spawned.lock().expect("lock spawned"), 0);
    }

    #[test]
    fn ensure_surface_schedules_spawn_when_missing() {
        let adapter = TestAdapter::default();

        let result = SessionSpawnManager
            .ensure_surface(&adapter, "session-b", request("session-b"), None)
            .expect("schedule spawn");

        assert_eq!(result, EnsureSurfaceResult::SpawnScheduled);
        assert_eq!(*adapter.spawned.lock().expect("lock spawned"), 1);
    }
}
