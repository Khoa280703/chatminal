use std::sync::atomic::{AtomicU64, Ordering};

use super::{LayoutNodeId, RuntimeId, TerminalInstanceId};

const CORE_RUNTIME_BASE: u64 = 1 << 60;
const CORE_LEAF_BASE: u64 = 1 << 61;
const CORE_LAYOUT_BASE: u64 = 1 << 62;

#[derive(Debug)]
pub struct SessionCoreIdAllocator {
    next_runtime_id: AtomicU64,
    next_terminal_instance_id: AtomicU64,
    next_layout_node_id: AtomicU64,
}

impl Default for SessionCoreIdAllocator {
    fn default() -> Self {
        Self {
            next_runtime_id: AtomicU64::new(CORE_RUNTIME_BASE),
            next_terminal_instance_id: AtomicU64::new(CORE_LEAF_BASE),
            next_layout_node_id: AtomicU64::new(CORE_LAYOUT_BASE),
        }
    }
}

impl SessionCoreIdAllocator {
    pub fn next_runtime_id(&self) -> RuntimeId {
        RuntimeId::new(self.next_runtime_id.fetch_add(1, Ordering::Relaxed))
    }

    pub fn next_terminal_instance_id(&self) -> TerminalInstanceId {
        TerminalInstanceId::new(
            self.next_terminal_instance_id
                .fetch_add(1, Ordering::Relaxed),
        )
    }

    pub fn next_layout_node_id(&self) -> LayoutNodeId {
        LayoutNodeId::new(self.next_layout_node_id.fetch_add(1, Ordering::Relaxed))
    }
}
