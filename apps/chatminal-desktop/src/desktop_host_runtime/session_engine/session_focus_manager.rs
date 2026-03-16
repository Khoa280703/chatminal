use config::keyassignment::SessionDirection;

use super::TerminalInstanceId;
use super::{EngineRuntimeAdapter, SessionRuntimeState, RuntimeId};

#[derive(Clone, Copy, Debug, Default)]
pub struct SessionFocusManager;

impl SessionFocusManager {
    pub fn focus_runtime<A: EngineRuntimeAdapter>(
        &self,
        adapter: &A,
        runtime_id: RuntimeId,
    ) -> Result<SessionRuntimeState, A::Error> {
        adapter.focus_runtime(runtime_id)?;
        adapter.snapshot_runtime(runtime_id)
    }

    pub fn focus_session<A: EngineRuntimeAdapter>(
        &self,
        adapter: &A,
        session_id: &str,
    ) -> Result<SessionRuntimeState, A::Error> {
        let runtime = adapter.attach_runtime(session_id)?;
        self.focus_runtime(adapter, runtime.runtime_id)
    }

    pub fn focus_terminal_instance<A: EngineRuntimeAdapter>(
        &self,
        adapter: &A,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Result<SessionRuntimeState, A::Error> {
        adapter.focus_terminal_instance(runtime_id, terminal_instance_id)?;
        adapter.snapshot_runtime(runtime_id)
    }

    pub fn focus_direction<A: EngineRuntimeAdapter>(
        &self,
        adapter: &A,
        runtime_id: RuntimeId,
        direction: SessionDirection,
    ) -> Result<Option<SessionRuntimeState>, A::Error> {
        let Some(target_terminal_instance_id) = adapter.adjacent_active_terminal_instance(runtime_id, direction)? else {
            return Ok(None);
        };
        self.focus_terminal_instance(adapter, runtime_id, target_terminal_instance_id)
            .map(Some)
    }

    pub fn swap_active_terminal_instance<A: EngineRuntimeAdapter>(
        &self,
        adapter: &A,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        keep_focus: bool,
    ) -> Result<SessionRuntimeState, A::Error> {
        adapter.swap_active_terminal_instance(runtime_id, terminal_instance_id, keep_focus)?;
        adapter.snapshot_runtime(runtime_id)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use config::keyassignment::SessionDirection;
    use window::Window;

    use super::SessionFocusManager;
    use super::super::{
        EngineRuntimeAdapter, EngineRuntimeRef, LayoutNodeId, MoveTerminalInstanceTarget, RuntimeId,
        SessionLayoutSnapshot, SessionRuntimeState, SpawnSessionRuntimeRequest,
        TerminalInstanceId,
    };

    #[derive(Default)]
    struct TestAdapter {
        focused: Mutex<Vec<RuntimeId>>,
    }

    impl EngineRuntimeAdapter for TestAdapter {
        type Error = &'static str;

        fn attach_runtime(&self, session_id: &str) -> Result<EngineRuntimeRef, Self::Error> {
            Ok(EngineRuntimeRef {
                runtime_id: RuntimeId::new(9),
                session_id: session_id.to_string(),
            })
        }

        fn focus_runtime(&self, runtime_id: RuntimeId) -> Result<(), Self::Error> {
            self.focused.lock().expect("lock focused").push(runtime_id);
            Ok(())
        }

        fn focus_terminal_instance(&self, runtime_id: RuntimeId, _terminal_instance_id: TerminalInstanceId) -> Result<(), Self::Error> {
            self.focused.lock().expect("lock focused").push(runtime_id);
            Ok(())
        }

        fn adjacent_active_terminal_instance(
            &self,
            _runtime_id: RuntimeId,
            _direction: SessionDirection,
        ) -> Result<Option<TerminalInstanceId>, Self::Error> {
            Ok(Some(TerminalInstanceId::new(2)))
        }

        fn swap_active_terminal_instance(
            &self,
            runtime_id: RuntimeId,
            _terminal_instance_id: TerminalInstanceId,
            _keep_focus: bool,
        ) -> Result<(), Self::Error> {
            self.focused.lock().expect("lock focused").push(runtime_id);
            Ok(())
        }

        fn move_terminal_instance(
            &self,
            _runtime_id: RuntimeId,
            _terminal_instance_id: TerminalInstanceId,
            _target: MoveTerminalInstanceTarget,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn close_runtime(&self, _runtime_id: RuntimeId) -> Result<(), Self::Error> {
            Ok(())
        }

        fn spawn_runtime(
            &self,
            _request: SpawnSessionRuntimeRequest,
            _window: Option<Window>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn snapshot_runtime(
            &self,
            runtime_id: RuntimeId,
        ) -> Result<SessionRuntimeState, Self::Error> {
            let mut state = SessionRuntimeState::detached("session-a", runtime_id);
            state.attach_layout(SessionLayoutSnapshot::single_terminal_instance(
                LayoutNodeId::new(1),
                TerminalInstanceId::new(2),
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
            &[RuntimeId::new(9)]
        );
        assert_eq!(state.snapshot.runtime_id, RuntimeId::new(9));
        assert_eq!(state.snapshot.active_terminal_instance_id, Some(super::TerminalInstanceId::new(2)));
    }

    #[test]
    fn focus_direction_uses_adapter_target_leaf() {
        let adapter = TestAdapter::default();
        let state = SessionFocusManager
            .focus_direction(&adapter, RuntimeId::new(9), SessionDirection::Right)
            .expect("focus direction")
            .expect("direction target");

        assert_eq!(state.snapshot.active_terminal_instance_id, Some(super::TerminalInstanceId::new(2)));
        assert_eq!(
            adapter.focused.lock().expect("lock focused").as_slice(),
            &[RuntimeId::new(9)]
        );
    }
}
