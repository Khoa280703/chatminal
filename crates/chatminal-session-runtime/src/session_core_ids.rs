use std::sync::atomic::{AtomicU64, Ordering};

use crate::{LayoutNodeId, LeafId, SurfaceId};

const CORE_SURFACE_BASE: u64 = 1 << 60;
const CORE_LEAF_BASE: u64 = 1 << 61;
const CORE_LAYOUT_BASE: u64 = 1 << 62;

#[derive(Debug)]
pub struct SessionCoreIdAllocator {
    next_surface_id: AtomicU64,
    next_leaf_id: AtomicU64,
    next_layout_node_id: AtomicU64,
}

impl Default for SessionCoreIdAllocator {
    fn default() -> Self {
        Self {
            next_surface_id: AtomicU64::new(CORE_SURFACE_BASE),
            next_leaf_id: AtomicU64::new(CORE_LEAF_BASE),
            next_layout_node_id: AtomicU64::new(CORE_LAYOUT_BASE),
        }
    }
}

impl SessionCoreIdAllocator {
    pub fn next_surface_id(&self) -> SurfaceId {
        SurfaceId::new(self.next_surface_id.fetch_add(1, Ordering::Relaxed))
    }

    pub fn next_leaf_id(&self) -> LeafId {
        LeafId::new(self.next_leaf_id.fetch_add(1, Ordering::Relaxed))
    }

    pub fn next_layout_node_id(&self) -> LayoutNodeId {
        LayoutNodeId::new(self.next_layout_node_id.fetch_add(1, Ordering::Relaxed))
    }
}
