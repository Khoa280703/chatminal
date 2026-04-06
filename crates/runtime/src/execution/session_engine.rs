use std::sync::{Arc, Mutex};

#[cfg(test)]
use super::TerminalInstanceId;
use super::leaf_runtime::TerminalInstanceRuntimeEvent;
use super::{SessionCoreState, SessionEngineShared};

#[derive(Clone, Debug)]
pub struct StatefulSessionEngine {
    shared: Arc<SessionEngineShared>,
}

impl StatefulSessionEngine {
    #[cfg(test)]
    pub fn new(core_state: Arc<Mutex<SessionCoreState>>) -> Self {
        Self {
            shared: Arc::new(SessionEngineShared::new(core_state)),
        }
    }

    pub fn with_shared(shared: Arc<SessionEngineShared>) -> Self {
        Self { shared }
    }

    pub fn core_state_handle(&self) -> Arc<Mutex<SessionCoreState>> {
        self.shared.core_state()
    }

    pub(crate) fn leaf_runtime_registry(
        &self,
    ) -> Arc<super::leaf_runtime_registry::TerminalInstanceRuntimeRegistry> {
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
        self.leaf_runtime_registry()
            .replay_output(terminal_instance_id)
    }

    pub(crate) fn leaf_runtime_events_tx(
        &self,
    ) -> std::sync::mpsc::SyncSender<TerminalInstanceRuntimeEvent> {
        self.shared.leaf_runtime_events_tx()
    }
}
