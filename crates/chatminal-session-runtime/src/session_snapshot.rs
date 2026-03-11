use std::collections::HashMap;

use crate::{LayoutNodeId, LeafId, SurfaceId};

pub const SESSION_GRAPH_SNAPSHOT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionGraphSnapshotVersion(pub u16);

impl Default for SessionGraphSnapshotVersion {
    fn default() -> Self {
        Self(SESSION_GRAPH_SNAPSHOT_VERSION)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSurfaceSnapshot {
    pub version: SessionGraphSnapshotVersion,
    pub session_id: String,
    pub surface_id: SurfaceId,
    pub active_leaf_id: Option<LeafId>,
    pub root_layout_node_id: Option<LayoutNodeId>,
}

impl SessionSurfaceSnapshot {
    pub fn new(session_id: impl Into<String>, surface_id: SurfaceId) -> Self {
        Self {
            version: SessionGraphSnapshotVersion::default(),
            session_id: session_id.into(),
            surface_id,
            active_leaf_id: None,
            root_layout_node_id: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionSurfaceLookup {
    pub active_session_id: Option<String>,
    pub last_active_session_id: Option<String>,
    pub surface_ids_by_session: HashMap<String, SurfaceId>,
}
