//! Single source of truth for shell chrome geometry (sidebar, header, footer, content).
//!
//! Both render and hit-test paths consume `ShellBounds` so that click targets
//! always match visible regions.

/// Pixel-space bounds for every shell chrome region.
///
/// Computed once per frame (or per resize) from window dimensions, OS border,
/// sidebar width, session-bar height, and footer height.
// Some fields are reserved for Phase 02-04 consumers (overlay, content area).
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ShellBounds {
    /// Full window pixel dimensions.
    pub window_width: f32,
    pub window_height: f32,

    /// OS-level border insets (title bar, rounded corners).
    pub border_top: f32,
    pub border_bottom: f32,
    pub border_left: f32,
    pub border_right: f32,

    /// Sidebar occupies left edge, full height inside border.
    pub sidebar_x: f32,
    pub sidebar_y: f32,
    pub sidebar_width: f32,
    pub sidebar_height: f32,

    /// Session bar (tab bar) — either top or bottom of content area.
    pub session_bar_x: f32,
    pub session_bar_y: f32,
    pub session_bar_width: f32,
    pub session_bar_height: f32,

    /// Footer bar — below content, right of sidebar.
    pub footer_x: f32,
    pub footer_y: f32,
    pub footer_width: f32,
    pub footer_height: f32,

    /// Content area — terminal panes live here.
    pub content_x: f32,
    pub content_y: f32,
    pub content_width: f32,
    pub content_height: f32,

    /// Whether sidebar shell is enabled at current dimensions.
    pub shell_enabled: bool,
    /// Whether session bar is at bottom (vs top).
    pub session_bar_at_bottom: bool,
}

#[allow(dead_code)]
impl ShellBounds {
    /// Check if pixel coordinate is inside sidebar region.
    pub fn is_inside_sidebar(&self, x: f32, y: f32) -> bool {
        self.shell_enabled
            && x >= self.sidebar_x
            && x < self.sidebar_x + self.sidebar_width
            && y >= self.sidebar_y
            && y < self.sidebar_y + self.sidebar_height
    }

    /// Check if pixel coordinate is inside footer region.
    pub fn is_inside_footer(&self, x: f32, y: f32) -> bool {
        self.footer_height > 0.0
            && x >= self.footer_x
            && x < self.footer_x + self.footer_width
            && y >= self.footer_y
            && y < self.footer_y + self.footer_height
    }
}
