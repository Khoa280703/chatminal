use std::sync::{Arc, Mutex};

use config::keyassignment::SessionDirection;

use super::leaf_runtime::TerminalInstanceRuntimeEvent;
use super::session_focus_manager::SessionFocusManager;
use super::{EngineRuntimeAdapter, RuntimeId, SessionCoreState, SessionEngineShared, SessionRuntimeState, TerminalInstanceId};

#[derive(Clone, Debug)]
pub struct StatefulSessionEngine<A> {
    adapter: A,
    shared: Arc<SessionEngineShared>,
}

impl<A> StatefulSessionEngine<A> {
    #[cfg(test)]
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

    pub(crate) fn event_hub(&self) -> Arc<super::session_event_bus::SessionEventHub> {
        self.shared.event_hub()
    }

    #[cfg(test)]
    pub(crate) fn subscribe(&self) -> super::SessionEventSubscription {
        self.shared.subscribe()
    }

    #[cfg(test)]
    pub fn replay_leaf_output(&self, terminal_instance_id: TerminalInstanceId) -> Option<String> {
        self.leaf_runtime_registry().replay_output(terminal_instance_id)
    }

    pub(crate) fn leaf_runtime_events_tx(&self) -> std::sync::mpsc::SyncSender<TerminalInstanceRuntimeEvent> {
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

    #[cfg(test)]
    pub fn runtime_id_for_session(&self, session_id: &str) -> Option<RuntimeId> {
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

    pub fn swap_active_with_session_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
        keep_focus: bool,
    ) -> Result<SessionRuntimeState, A::Error> {
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

    pub fn activate_session_direction(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> Result<Option<SessionRuntimeState>, A::Error> {
        let runtime = self.adapter.attach_runtime(session_id)?;
        let state = SessionFocusManager.focus_direction(&self.adapter, runtime.runtime_id, direction)?;
        if let Some(state) = &state {
            self.record_runtime_state(state);
        }
        Ok(state)
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::StatefulSessionEngine;
    use super::super::{
        EngineRuntimeAdapter, EngineRuntimeRef, LayoutNodeId, RuntimeId, SessionCoreState,
        SessionLayoutSnapshot, SessionRuntimeState, TerminalInstanceId,
    };

    struct TestAdapter;

    impl EngineRuntimeAdapter for TestAdapter {
        type Error = &'static str;

        fn attach_runtime(&self, _: &str) -> Result<EngineRuntimeRef, Self::Error> {
            Ok(EngineRuntimeRef {
                runtime_id: RuntimeId::new(7),
                session_id: "session-a".into(),
            })
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

        fn swap_active_terminal_instance(
            &self,
            _: RuntimeId,
            _: TerminalInstanceId,
            _: bool,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn snapshot_runtime(&self, runtime_id: RuntimeId) -> Result<SessionRuntimeState, Self::Error> {
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
    fn runtime_lookup_records_runtime_in_core_store() {
        let engine = StatefulSessionEngine::new(
            TestAdapter,
            Arc::new(Mutex::new(SessionCoreState::default())),
        );

        let runtime_id = engine.runtime_id_for_session("session-a");
        let core = engine.core_state_handle();
        let core = core.lock().unwrap();
        assert_eq!(runtime_id, Some(RuntimeId::new(7)));
        assert_eq!(core.runtime_id_for_session("session-a"), Some(RuntimeId::new(7)));
    }
}
