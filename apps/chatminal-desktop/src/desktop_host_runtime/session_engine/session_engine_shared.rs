use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use chatminal_terminal_core::TerminalSize;

use super::leaf_runtime::TerminalInstanceRuntimeEvent;
use super::leaf_runtime_registry::TerminalInstanceRuntimeRegistry;
use super::session_core_ids::SessionCoreIdAllocator;
use super::session_event_bus::SessionEventHub;
use chatminal_runtime::WorkspaceLayoutRegistry;
use super::{
    SessionCoreState, SessionEventBus, SessionEventSubscription, SessionRuntimeEvent,
    TerminalInstanceId,
};

#[derive(Debug)]
pub struct SessionEngineShared {
    core_state: Arc<Mutex<SessionCoreState>>,
    workspace_layouts: Arc<Mutex<WorkspaceLayoutRegistry>>,
    leaf_runtimes: Arc<TerminalInstanceRuntimeRegistry>,
    core_ids: Arc<SessionCoreIdAllocator>,
    event_hub: Arc<SessionEventHub>,
    leaf_runtime_events_tx: std_mpsc::SyncSender<TerminalInstanceRuntimeEvent>,
}

impl SessionEngineShared {
    pub fn new(core_state: Arc<Mutex<SessionCoreState>>) -> Self {
        let event_hub = Arc::new(SessionEventHub::default());
        let (leaf_runtime_events_tx, leaf_runtime_events_rx) =
            std_mpsc::sync_channel::<TerminalInstanceRuntimeEvent>(1024);
        let event_hub_for_thread: Arc<SessionEventHub> = Arc::clone(&event_hub);
        thread::spawn(move || {
            while let Ok(event) = leaf_runtime_events_rx.recv() {
                match event {
                    TerminalInstanceRuntimeEvent::Output {
                        session_id,
                        generation,
                        runtime_id,
                        terminal_instance_id,
                        chunk,
                    } => event_hub_for_thread.publish(SessionRuntimeEvent::TerminalInstanceOutput {
                        session_id,
                        generation,
                        runtime_id,
                        terminal_instance_id,
                        chunk,
                    }),
                    TerminalInstanceRuntimeEvent::Exited {
                        session_id,
                        generation,
                        runtime_id,
                        terminal_instance_id,
                        exit_code,
                    } => event_hub_for_thread.publish(SessionRuntimeEvent::TerminalInstanceExited {
                        session_id,
                        generation,
                        runtime_id,
                        terminal_instance_id,
                        exit_code,
                    }),
                    TerminalInstanceRuntimeEvent::Error {
                        session_id,
                        generation,
                        runtime_id,
                        terminal_instance_id,
                        message,
                    } => event_hub_for_thread.publish(SessionRuntimeEvent::TerminalInstanceError {
                        session_id,
                        generation,
                        runtime_id,
                        terminal_instance_id,
                        message,
                    }),
                }
            }
        });
        Self {
            core_state,
            workspace_layouts: Arc::new(Mutex::new(WorkspaceLayoutRegistry::default())),
            leaf_runtimes: Arc::new(TerminalInstanceRuntimeRegistry::default()),
            core_ids: Arc::new(SessionCoreIdAllocator::default()),
            event_hub,
            leaf_runtime_events_tx,
        }
    }

    pub fn core_state(&self) -> Arc<Mutex<SessionCoreState>> {
        Arc::clone(&self.core_state)
    }

    pub(crate) fn leaf_runtimes(&self) -> Arc<TerminalInstanceRuntimeRegistry> {
        Arc::clone(&self.leaf_runtimes)
    }

    pub fn workspace_layouts(&self) -> Arc<Mutex<WorkspaceLayoutRegistry>> {
        Arc::clone(&self.workspace_layouts)
    }

    pub fn core_ids(&self) -> Arc<SessionCoreIdAllocator> {
        Arc::clone(&self.core_ids)
    }

    pub(crate) fn event_hub(&self) -> Arc<SessionEventHub> {
        Arc::clone(&self.event_hub)
    }

    pub(crate) fn leaf_runtime_events_tx(&self) -> std_mpsc::SyncSender<TerminalInstanceRuntimeEvent> {
        self.leaf_runtime_events_tx.clone()
    }

    pub fn subscribe(&self) -> SessionEventSubscription {
        self.event_hub.subscribe()
    }

    pub fn replay_output(&self, terminal_instance_id: TerminalInstanceId) -> Option<String> {
        self.leaf_runtimes.replay_output(terminal_instance_id)
    }

    pub fn write_terminal_input(
        &self,
        terminal_instance_id: TerminalInstanceId,
        data: impl AsRef<[u8]>,
    ) -> Result<(), String> {
        self.leaf_runtimes
            .runtime(terminal_instance_id)
            .ok_or_else(|| format!("terminal instance runtime {terminal_instance_id} missing"))?
            .write_input(data)
    }

    pub fn resize_terminal_instance(
        &self,
        terminal_instance_id: TerminalInstanceId,
        size: TerminalSize,
    ) -> Result<(), String> {
        self.leaf_runtimes
            .runtime(terminal_instance_id)
            .ok_or_else(|| format!("terminal instance runtime {terminal_instance_id} missing"))?
            .resize(size)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::SessionCoreState;

    use super::SessionEngineShared;

    #[test]
    fn shared_exposes_workspace_layout_registry() {
        let shared = SessionEngineShared::new(Arc::new(Mutex::new(SessionCoreState::default())));
        let layouts = shared.workspace_layouts();
        let ensured = layouts
            .lock()
            .expect("lock layouts")
            .ensure_layout("desktop-main", "session-a");

        assert_eq!(ensured.views.len(), 1);
        assert_eq!(ensured.views[0].session_id, "session-a");
    }
}
