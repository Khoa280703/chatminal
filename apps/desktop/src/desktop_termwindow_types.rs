use crate::runtime_module::SessionTerminalHandle;
use crate::desktop_session_host::overlay_shell::{
    OverlayPane, OverlayPaneLayout, OverlaySplitDirection, OverlaySplitLayout,
};
use crate::desktop_session_host::terminal_handle_for_pane as terminal_handle_for_overlay_pane;

pub type TerminalUiKey = u64;

pub fn terminal_ui_key_for_pane(pane: &dyn OverlayPane) -> TerminalUiKey {
    terminal_handle_for_overlay_pane(pane).as_u64()
}

pub fn terminal_handle_for_ui_key(key: TerminalUiKey) -> SessionTerminalHandle {
    SessionTerminalHandle::new(key)
}

#[derive(Clone)]
pub struct TerminalPaneLayout {
    pub index: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub pixel_width: usize,
    pub height: usize,
    pub pixel_height: usize,
    pub pane: std::sync::Arc<dyn OverlayPane>,
}

impl TerminalPaneLayout {
    pub fn from_overlay_layout(layout: OverlayPaneLayout) -> Self {
        Self {
            index: layout.index,
            is_active: layout.is_active,
            is_zoomed: layout.is_zoomed,
            left: layout.left,
            top: layout.top,
            width: layout.width,
            pixel_width: layout.pixel_width,
            height: layout.height,
            pixel_height: layout.pixel_height,
            pane: layout.pane,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalSplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalSplit {
    pub index: usize,
    pub direction: TerminalSplitDirection,
    pub left: usize,
    pub top: usize,
    pub size: usize,
}

impl TerminalSplit {
    pub fn from_overlay_split(split: OverlaySplitLayout) -> Self {
        Self {
            index: split.index,
            direction: TerminalSplitDirection::from_overlay_direction(split.direction),
            left: split.left,
            top: split.top,
            size: split.size,
        }
    }
}

impl TerminalSplitDirection {
    pub fn from_overlay_direction(direction: OverlaySplitDirection) -> Self {
        match direction {
            OverlaySplitDirection::Horizontal => Self::Horizontal,
            OverlaySplitDirection::Vertical => Self::Vertical,
        }
    }
}
