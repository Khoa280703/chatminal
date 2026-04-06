use terminal_emulator::TerminalSize;

use crate::runtime_module::{
    SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle, TerminalInstanceId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ChatminalRenderSplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatminalRenderPane {
    pub terminal_handle: SessionTerminalHandle,
    pub terminal_instance_id: TerminalInstanceId,
    pub index: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub pixel_width: usize,
    pub height: usize,
    pub pixel_height: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatminalRenderSplit {
    pub index: usize,
    pub axis: ChatminalRenderSplitAxis,
    pub left: usize,
    pub top: usize,
    pub size: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatminalRenderState {
    pub render_target: SessionRenderTargetSnapshot,
    pub terminal_size: TerminalSize,
    pub active_terminal_instance_id: Option<TerminalInstanceId>,
    pub panes: Vec<ChatminalRenderPane>,
    pub splits: Vec<ChatminalRenderSplit>,
}

impl ChatminalRenderState {
    pub fn render_target_id(&self) -> SessionRenderTargetId {
        self.render_target.render_target_id
    }
}
