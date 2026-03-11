use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::leaf_runtime::LeafRuntimeEvent;
use crate::session_core_ids::SessionCoreIdAllocator;
use crate::{
    LeafRuntimeRegistry, SessionCoreState, SessionEventBus, SessionEventHub, SessionRuntimeEvent,
};

#[derive(Debug)]
pub struct SessionEngineShared {
    core_state: Arc<Mutex<SessionCoreState>>,
    leaf_runtimes: Arc<LeafRuntimeRegistry>,
    core_ids: Arc<SessionCoreIdAllocator>,
    event_hub: Arc<SessionEventHub>,
    leaf_runtime_events_tx: std_mpsc::SyncSender<LeafRuntimeEvent>,
}

impl SessionEngineShared {
    pub fn new(core_state: Arc<Mutex<SessionCoreState>>) -> Self {
        let event_hub = Arc::new(SessionEventHub::default());
        let (leaf_runtime_events_tx, leaf_runtime_events_rx) =
            std_mpsc::sync_channel::<LeafRuntimeEvent>(1024);
        let event_hub_for_thread = Arc::clone(&event_hub);
        thread::spawn(move || {
            while let Ok(event) = leaf_runtime_events_rx.recv() {
                match event {
                    LeafRuntimeEvent::Output {
                        session_id,
                        generation,
                        surface_id,
                        leaf_id,
                        chunk,
                    } => event_hub_for_thread.publish(SessionRuntimeEvent::LeafOutput {
                        session_id,
                        generation,
                        surface_id,
                        leaf_id,
                        chunk,
                    }),
                    LeafRuntimeEvent::Exited {
                        session_id,
                        generation,
                        surface_id,
                        leaf_id,
                        exit_code,
                    } => event_hub_for_thread.publish(SessionRuntimeEvent::LeafExited {
                        session_id,
                        generation,
                        surface_id,
                        leaf_id,
                        exit_code,
                    }),
                    LeafRuntimeEvent::Error {
                        session_id,
                        generation,
                        surface_id,
                        leaf_id,
                        message,
                    } => event_hub_for_thread.publish(SessionRuntimeEvent::LeafError {
                        session_id,
                        generation,
                        surface_id,
                        leaf_id,
                        message,
                    }),
                }
            }
        });
        Self {
            core_state,
            leaf_runtimes: Arc::new(LeafRuntimeRegistry::default()),
            core_ids: Arc::new(SessionCoreIdAllocator::default()),
            event_hub,
            leaf_runtime_events_tx,
        }
    }

    pub fn core_state(&self) -> Arc<Mutex<SessionCoreState>> {
        Arc::clone(&self.core_state)
    }

    pub fn leaf_runtimes(&self) -> Arc<LeafRuntimeRegistry> {
        Arc::clone(&self.leaf_runtimes)
    }

    pub fn core_ids(&self) -> Arc<SessionCoreIdAllocator> {
        Arc::clone(&self.core_ids)
    }

    pub fn event_hub(&self) -> Arc<SessionEventHub> {
        Arc::clone(&self.event_hub)
    }

    pub fn leaf_runtime_events_tx(&self) -> std_mpsc::SyncSender<LeafRuntimeEvent> {
        self.leaf_runtime_events_tx.clone()
    }
}
