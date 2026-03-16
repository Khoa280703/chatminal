use window::Window;

use super::session_focus_manager::SessionFocusManager;
use super::{EngineRuntimeAdapter, SessionRuntimeState, SpawnSessionRuntimeRequest};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnsureRuntimeResult {
    FocusedExisting(SessionRuntimeState),
    SpawnScheduled,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SessionSpawnManager;

impl SessionSpawnManager {
    pub fn ensure_runtime<A: EngineRuntimeAdapter>(
        &self,
        adapter: &A,
        session_id: &str,
        request: SpawnSessionRuntimeRequest,
        window: Option<Window>,
    ) -> Result<EnsureRuntimeResult, A::Error> {
        if let Ok(runtime) = adapter.attach_runtime(session_id) {
            let state = SessionFocusManager.focus_runtime(adapter, runtime.runtime_id)?;
            return Ok(EnsureRuntimeResult::FocusedExisting(state));
        }

        adapter.spawn_runtime(request, window)?;
        Ok(EnsureRuntimeResult::SpawnScheduled)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use engine_term::TerminalSize;
    use portable_pty::CommandBuilder;
    use window::Window;

    use super::SessionSpawnManager;
    use super::super::{
        EngineRuntimeAdapter, EngineRuntimeRef, EnsureRuntimeResult, MoveTerminalInstanceTarget,
        RuntimeId, SessionRuntimeState, SpawnSessionRuntimeRequest, TerminalInstanceId,
    };

    #[derive(Default)]
    struct TestAdapter {
        attach_result: Mutex<Option<EngineRuntimeRef>>,
        focused: Mutex<Vec<RuntimeId>>,
        spawned: Mutex<usize>,
    }

    impl EngineRuntimeAdapter for TestAdapter {
        type Error = &'static str;

        fn attach_runtime(&self, _session_id: &str) -> Result<EngineRuntimeRef, Self::Error> {
            self.attach_result
                .lock()
                .expect("lock attach result")
                .clone()
                .ok_or("runtime not found")
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
            _direction: config::keyassignment::SessionDirection,
        ) -> Result<Option<TerminalInstanceId>, Self::Error> {
            Ok(None)
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
            *self.spawned.lock().expect("lock spawned") += 1;
            Ok(())
        }

        fn snapshot_runtime(
            &self,
            runtime_id: RuntimeId,
        ) -> Result<SessionRuntimeState, Self::Error> {
            Ok(SessionRuntimeState::detached("session-a", runtime_id))
        }
    }

    fn request(session_id: &str) -> SpawnSessionRuntimeRequest {
        SpawnSessionRuntimeRequest {
            session_id: session_id.to_string(),
            terminal_size: TerminalSize::default(),
            current_host_handle: None,
            workspace: "default".to_string(),
            domain: config::keyassignment::SpawnSessionDomain::CurrentSessionDomain,
            command: CommandBuilder::new_default_prog(),
        }
    }

    #[test]
    fn ensure_runtime_focuses_existing_runtime() {
        let adapter = TestAdapter {
            attach_result: Mutex::new(Some(EngineRuntimeRef {
                runtime_id: RuntimeId::new(5),
                session_id: "session-a".to_string(),
            })),
            ..TestAdapter::default()
        };

        let result = SessionSpawnManager
            .ensure_runtime(&adapter, "session-a", request("session-a"), None)
            .expect("ensure existing runtime");

        assert_eq!(
            result,
            EnsureRuntimeResult::FocusedExisting(SessionRuntimeState::detached(
                "session-a",
                RuntimeId::new(5),
            ))
        );
        assert_eq!(
            adapter.focused.lock().expect("lock focused").as_slice(),
            &[RuntimeId::new(5)]
        );
        assert_eq!(*adapter.spawned.lock().expect("lock spawned"), 0);
    }

    #[test]
    fn ensure_runtime_schedules_spawn_when_missing() {
        let adapter = TestAdapter::default();

        let result = SessionSpawnManager
            .ensure_runtime(&adapter, "session-b", request("session-b"), None)
            .expect("schedule spawn");

        assert_eq!(result, EnsureRuntimeResult::SpawnScheduled);
        assert_eq!(*adapter.spawned.lock().expect("lock spawned"), 1);
    }
}
