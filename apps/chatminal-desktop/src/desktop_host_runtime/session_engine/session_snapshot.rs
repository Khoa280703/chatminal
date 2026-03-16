use std::collections::HashMap;

use super::{LayoutNodeId, TerminalInstanceId, RuntimeId};

pub const SESSION_GRAPH_SNAPSHOT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionGraphSnapshotVersion(pub u16);

impl Default for SessionGraphSnapshotVersion {
    fn default() -> Self {
        Self(SESSION_GRAPH_SNAPSHOT_VERSION)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRuntimeSnapshot {
    pub version: SessionGraphSnapshotVersion,
    pub session_id: String,
    pub runtime_id: RuntimeId,
    pub active_terminal_instance_id: Option<TerminalInstanceId>,
    pub root_layout_node_id: Option<LayoutNodeId>,
}

impl SessionRuntimeSnapshot {
    pub fn new(session_id: impl Into<String>, runtime_id: RuntimeId) -> Self {
        Self {
            version: SessionGraphSnapshotVersion::default(),
            session_id: session_id.into(),
            runtime_id,
            active_terminal_instance_id: None,
            root_layout_node_id: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionRuntimeLookup {
    pub active_session_id: Option<String>,
    pub last_active_session_id: Option<String>,
    pub runtime_ids_by_session: HashMap<String, RuntimeId>,
}
