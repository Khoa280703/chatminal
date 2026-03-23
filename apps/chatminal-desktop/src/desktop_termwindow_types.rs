use std::convert::TryFrom;

use crate::chatminal_runtime::overlay_compat::{
    OverlayPane, OverlayPaneHandle, OverlayPaneLayout, OverlaySplitDirection, OverlaySplitLayout,
};

pub type TerminalUiKey = u64;

pub fn pane_id_from_terminal_ui_key(key: TerminalUiKey) -> Option<OverlayPaneHandle> {
    OverlayPaneHandle::try_from(key).ok()
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
    pub fn from_mux(layout: OverlayPaneLayout) -> Self {
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
    pub fn from_mux(split: OverlaySplitLayout) -> Self {
        Self {
            index: split.index,
            direction: TerminalSplitDirection::from_mux(split.direction),
            left: split.left,
            top: split.top,
            size: split.size,
        }
    }
}

impl TerminalSplitDirection {
    pub fn from_mux(direction: OverlaySplitDirection) -> Self {
        match direction {
            OverlaySplitDirection::Horizontal => Self::Horizontal,
            OverlaySplitDirection::Vertical => Self::Vertical,
        }
    }
}
