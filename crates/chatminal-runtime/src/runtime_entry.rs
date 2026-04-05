use engine_term::TerminalSize;

use crate::{RuntimeId, SessionTerminalHandle, SplitDirection};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusedPaneBinding {
    runtime_id: RuntimeId,
    terminal_handle: SessionTerminalHandle,
}

impl FocusedPaneBinding {
    pub fn new(runtime_id: RuntimeId, terminal_handle: SessionTerminalHandle) -> Self {
        Self {
            runtime_id,
            terminal_handle,
        }
    }

    pub fn runtime_id(self) -> RuntimeId {
        self.runtime_id
    }

    pub fn terminal_handle(self) -> SessionTerminalHandle {
        self.terminal_handle
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeEntryInfo {
    pub runtime_id: RuntimeId,
    pub title: String,
    pub session_id: Option<String>,
    pub active_terminal_handle: Option<SessionTerminalHandle>,
    pub active_terminal_instance_id: Option<u64>,
    pub size: TerminalSize,
}

#[derive(Clone, Debug)]
pub struct RuntimeEntryTerminalInfo {
    pub index: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub pixel_width: usize,
    pub height: usize,
    pub pixel_height: usize,
    pub terminal_handle: SessionTerminalHandle,
    pub session_id: Option<String>,
    pub terminal_instance_id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeEntrySplitInfo {
    pub index: usize,
    pub direction: SplitDirection,
    pub left: usize,
    pub top: usize,
    pub size: usize,
}
