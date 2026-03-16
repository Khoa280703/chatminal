use super::{SessionLayoutSnapshot, SessionRuntimeSnapshot, RuntimeId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRuntimeState {
    pub snapshot: SessionRuntimeSnapshot,
    pub layout: Option<SessionLayoutSnapshot>,
}

impl SessionRuntimeState {
    pub fn detached(session_id: impl Into<String>, runtime_id: RuntimeId) -> Self {
        Self {
            snapshot: SessionRuntimeSnapshot::new(session_id, runtime_id),
            layout: None,
        }
    }

    pub fn attach_layout(&mut self, layout: SessionLayoutSnapshot) {
        self.snapshot.active_terminal_instance_id = Some(layout.active_terminal_instance_id);
        self.snapshot.root_layout_node_id = Some(layout.root_layout_node_id);
        self.layout = Some(layout);
    }
}
