use std::sync::{Arc, Mutex};

use config::keyassignment::SessionDirection;
use window::Window;

use super::leaf_runtime::TerminalInstanceRuntimeEvent;
use super::session_focus_manager::SessionFocusManager;
use super::session_spawn_manager::SessionSpawnManager;
use super::{
    EngineRuntimeAdapter, EnsureRuntimeResult, MoveTerminalInstanceTarget, RuntimeId,
    SessionCoreState, SessionEngineShared, SessionRuntimeState, SpawnSessionRuntimeRequest,
    TerminalInstanceId,
};

pub trait SessionEngine {
    type Error;
    fn focus_session_state(&self, session_id: &str) -> Result<SessionRuntimeState, Self::Error>;
    fn focus_runtime_state(
        &self,
        runtime_id: RuntimeId,
    ) -> Result<SessionRuntimeState, Self::Error>;
    fn remove_session_runtime(&self, session_id: &str) -> Result<(), Self::Error>;
    fn runtime_id_for_session(&self, session_id: &str) -> Option<RuntimeId>;
    fn active_terminal_instance_id(&self, session_id: &str) -> Option<TerminalInstanceId>;
    fn focus_session_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
    ) -> Result<SessionRuntimeState, Self::Error>;
    fn swap_active_with_session_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
        keep_focus: bool,
    ) -> Result<SessionRuntimeState, Self::Error>;
    fn move_session_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
        target: MoveTerminalInstanceTarget,
    ) -> Result<(), Self::Error>;
    fn activate_session_direction(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> Result<Option<SessionRuntimeState>, Self::Error>;
    fn ensure_session_runtime(
        &self,
        session_id: &str,
        request: SpawnSessionRuntimeRequest,
        window: Option<Window>,
    ) -> Result<EnsureRuntimeResult, Self::Error>;
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

    pub(crate) fn leaf_runtime_registry(&self) -> Arc<super::leaf_runtime_registry::TerminalInstanceRuntimeRegistry> {
        self.shared.leaf_runtimes()
    }

    pub(crate) fn core_id_allocator(&self) -> Arc<super::session_core_ids::SessionCoreIdAllocator> {
        self.shared.core_ids()
    }

    pub fn shared(&self) -> Arc<SessionEngineShared> {
        Arc::clone(&self.shared)
    }

    pub(crate) fn event_hub(&self) -> Arc<super::session_event_bus::SessionEventHub> {
        self.shared.event_hub()
    }

    #[cfg(test)]
    pub(crate) fn subscribe(&self) -> super::SessionEventSubscription {
        self.shared.subscribe()
    }

    pub fn replay_leaf_output(&self, terminal_instance_id: TerminalInstanceId) -> Option<String> {
        self.leaf_runtime_registry().replay_output(terminal_instance_id)
    }

    pub(crate) fn leaf_runtime_events_tx(
        &self,
    ) -> std::sync::mpsc::SyncSender<TerminalInstanceRuntimeEvent> {
        self.shared.leaf_runtime_events_tx()
    }
}

impl<A: EngineRuntimeAdapter> StatefulSessionEngine<A> {
    fn record_runtime_state(&self, state: &SessionRuntimeState) {
        let core_handle = self.core_state_handle();
        let mut core = core_handle.lock().unwrap();
        let runtime =
            core.register_runtime(state.snapshot.session_id.clone(), state.snapshot.runtime_id);
        runtime.root_layout_node_id = state.snapshot.root_layout_node_id;
        runtime.active_terminal_instance_id = state.snapshot.active_terminal_instance_id;
        if let Some(layout) = &state.layout {
            runtime.sync_layout(layout);
        }
    }

    fn remove_runtime_from_state(&self, runtime_id: RuntimeId) {
        self.core_state_handle()
            .lock()
            .unwrap()
            .remove_runtime(runtime_id);
    }

    pub fn refresh_runtime_state_from_adapter(
        &self,
        runtime_id: RuntimeId,
    ) -> Result<SessionRuntimeState, A::Error> {
        let state = self.adapter.snapshot_runtime(runtime_id)?;
        self.record_runtime_state(&state);
        Ok(state)
    }
}

impl<A: EngineRuntimeAdapter> SessionEngine for StatefulSessionEngine<A> {
    type Error = A::Error;

    fn focus_session_state(&self, session_id: &str) -> Result<SessionRuntimeState, Self::Error> {
        let state = SessionFocusManager.focus_session(&self.adapter, session_id)?;
        self.record_runtime_state(&state);
        Ok(state)
    }

    fn focus_runtime_state(
        &self,
        runtime_id: RuntimeId,
    ) -> Result<SessionRuntimeState, Self::Error> {
        let state = SessionFocusManager.focus_runtime(&self.adapter, runtime_id)?;
        self.record_runtime_state(&state);
        Ok(state)
    }

    fn remove_session_runtime(&self, session_id: &str) -> Result<(), Self::Error> {
        let runtime_id = self.runtime_id_for_session(session_id).or_else(|| {
            self.adapter
                .attach_runtime(session_id)
                .map(|runtime| runtime.runtime_id)
                .ok()
        });
        if let Some(runtime_id) = runtime_id {
            self.adapter.close_runtime(runtime_id)?;
            self.remove_runtime_from_state(runtime_id);
        }
        Ok(())
    }

    fn runtime_id_for_session(&self, session_id: &str) -> Option<RuntimeId> {
        if let Some(runtime_id) = self
            .core_state_handle()
            .lock()
            .unwrap()
            .runtime_id_for_session(session_id)
        {
            return Some(runtime_id);
        }
        let runtime = self.adapter.attach_runtime(session_id).ok()?;
        self.core_state_handle()
            .lock()
            .unwrap()
            .register_runtime(runtime.session_id, runtime.runtime_id);
        Some(runtime.runtime_id)
    }

    fn active_terminal_instance_id(&self, session_id: &str) -> Option<TerminalInstanceId> {
        if let Some(runtime_id) = self
            .core_state_handle()
            .lock()
            .unwrap()
            .runtime_id_for_session(session_id)
        {
            let active_terminal_instance_id = self
                .core_state_handle()
                .lock()
                .unwrap()
                .runtime(runtime_id)
                .and_then(|runtime| runtime.active_terminal_instance_id);
            if active_terminal_instance_id.is_some() {
                return active_terminal_instance_id;
            }
        }
        let runtime_id = self.adapter.attach_runtime(session_id).ok()?.runtime_id;
        let state = self.adapter.snapshot_runtime(runtime_id).ok()?;
        let active_terminal_instance_id = state.snapshot.active_terminal_instance_id;
        self.record_runtime_state(&state);
        active_terminal_instance_id
    }

    fn focus_session_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
    ) -> Result<SessionRuntimeState, Self::Error> {
        let runtime = self.adapter.attach_runtime(session_id)?;
        let state = SessionFocusManager.focus_terminal_instance(&self.adapter, runtime.runtime_id, terminal_instance_id)?;
        self.record_runtime_state(&state);
        Ok(state)
    }

    fn swap_active_with_session_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
        keep_focus: bool,
    ) -> Result<SessionRuntimeState, Self::Error> {
        let runtime = self.adapter.attach_runtime(session_id)?;
        let state = SessionFocusManager.swap_active_terminal_instance(
            &self.adapter,
            runtime.runtime_id,
            terminal_instance_id,
            keep_focus,
        )?;
        self.record_runtime_state(&state);
        Ok(state)
    }

    fn move_session_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
        target: MoveTerminalInstanceTarget,
    ) -> Result<(), Self::Error> {
        let runtime = self.adapter.attach_runtime(session_id)?;
        self.adapter.move_terminal_instance(runtime.runtime_id, terminal_instance_id, target)
    }

    fn activate_session_direction(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> Result<Option<SessionRuntimeState>, Self::Error> {
        let runtime = self.adapter.attach_runtime(session_id)?;
        let state =
            SessionFocusManager.focus_direction(&self.adapter, runtime.runtime_id, direction)?;
        if let Some(state) = &state {
            self.record_runtime_state(&state);
        }
        Ok(state)
    }

    fn ensure_session_runtime(
        &self,
        session_id: &str,
        request: SpawnSessionRuntimeRequest,
        window: Option<Window>,
    ) -> Result<EnsureRuntimeResult, Self::Error> {
        let result =
            SessionSpawnManager.ensure_runtime(&self.adapter, session_id, request, window)?;
        if let EnsureRuntimeResult::FocusedExisting(state) = &result {
            self.record_runtime_state(state);
        }
        Ok(result)
    }
}


#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use engine_term::TerminalSize;
    use portable_pty::CommandBuilder;
    use window::Window;

    use super::{SessionEngine, StatefulSessionEngine};
    use super::super::{
        EngineRuntimeAdapter, EngineRuntimeRef, EnsureRuntimeResult, TerminalInstanceId, MoveTerminalInstanceTarget,
        SessionCoreState, SessionLayoutSnapshot, SessionRuntimeState, SpawnSessionRuntimeRequest,
        RuntimeId, LayoutNodeId,
    };

    struct TestAdapter {
        attach_missing: bool,
    }

    impl EngineRuntimeAdapter for TestAdapter {
        type Error = &'static str;
        fn attach_runtime(&self, _: &str) -> Result<EngineRuntimeRef, Self::Error> {
            if self.attach_missing {
                Err("runtime missing")
            } else {
                Ok(EngineRuntimeRef {
                    runtime_id: RuntimeId::new(7),
                    session_id: "session-a".into(),
                })
            }
        }
        fn focus_runtime(&self, _: RuntimeId) -> Result<(), Self::Error> {
            Ok(())
        }
        fn focus_terminal_instance(&self, _: RuntimeId, _: TerminalInstanceId) -> Result<(), Self::Error> {
            Ok(())
        }
        fn adjacent_active_terminal_instance(
            &self,
            _: RuntimeId,
            _: config::keyassignment::SessionDirection,
        ) -> Result<Option<TerminalInstanceId>, Self::Error> {
            Ok(None)
        }
        fn swap_active_terminal_instance(&self, _: RuntimeId, _: TerminalInstanceId, _: bool) -> Result<(), Self::Error> {
            Ok(())
        }
        fn move_terminal_instance(&self, _: RuntimeId, _: TerminalInstanceId, _: MoveTerminalInstanceTarget) -> Result<(), Self::Error> {
            Ok(())
        }
        fn close_runtime(&self, _: RuntimeId) -> Result<(), Self::Error> {
            Ok(())
        }
        fn spawn_runtime(
            &self,
            _: SpawnSessionRuntimeRequest,
            _: Option<Window>,
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

    fn request() -> SpawnSessionRuntimeRequest {
        SpawnSessionRuntimeRequest {
            session_id: "session-a".into(),
            terminal_size: TerminalSize::default(),
            current_host_handle: None,
            workspace: "default".into(),
            domain: config::keyassignment::SpawnSessionDomain::CurrentSessionDomain,
            command: CommandBuilder::new_default_prog(),
        }
    }

    #[test]
    fn facade_records_runtime_state_in_core_store() {
        let engine = StatefulSessionEngine::new(
            TestAdapter {
                attach_missing: false,
            },
            Arc::new(Mutex::new(SessionCoreState::default())),
        );
        let state = engine.focus_session_state("session-a").unwrap();
        let core = engine.core_state_handle();
        let core = core.lock().unwrap();
        assert_eq!(state.snapshot.runtime_id, RuntimeId::new(7));
        assert_eq!(
            core.runtime_id_for_session("session-a"),
            Some(RuntimeId::new(7))
        );
        assert_eq!(
            core.runtime(RuntimeId::new(7))
                .and_then(|runtime| runtime.active_terminal_instance_id),
            Some(TerminalInstanceId::new(2))
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
            engine.ensure_session_runtime("session-a", request(), None),
            Ok(EnsureRuntimeResult::SpawnScheduled)
        ));
    }
}
