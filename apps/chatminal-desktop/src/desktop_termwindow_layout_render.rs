use std::sync::Arc;

use crate::chatminal_render::{ChatminalRenderPane, ChatminalRenderSplit, ChatminalRenderSplitAxis};
use crate::chatminal_runtime::{
    SessionViewId, WorkspaceLayoutNodeKind, WorkspaceLayoutState, WorkspaceNodeId,
    WorkspaceSplitAxis,
};
use crate::desktop_termwindow_types::{TerminalPaneLayout, TerminalSplit, TerminalSplitDirection};
use crate::scripting::guiwin::DesktopWindowId;
const WORKSPACE_SPLIT_INDEX_BASE: usize = 1_000_000;

#[derive(Clone)]
pub(super) struct LayoutRenderTarget {
    pub session_id: String,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
    pub is_active: bool,
}

#[derive(Clone, Copy)]
struct LayoutBounds {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy)]
struct WorkspaceSplitTarget {
    node_id: WorkspaceNodeId,
    split: TerminalSplit,
    bounds: LayoutBounds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RenderPaneGeometry {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    pixel_width: usize,
    pixel_height: usize,
    is_active: bool,
    is_zoomed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RenderSplitGeometry {
    direction: TerminalSplitDirection,
    left: usize,
    top: usize,
    size: usize,
}

impl crate::TermWindow {
    pub(super) fn layout_render_targets(&self) -> Vec<LayoutRenderTarget> {
        if !self.chatminal_sidebar.is_enabled() {
            return Vec::new();
        }
        let Ok(snapshot) = crate::chatminal_runtime::desktop_session_window_snapshot(
            self.window_id as DesktopWindowId,
        ) else {
            return Vec::new();
        };
        let active_view_id = snapshot.active_view_id();
        let Some(layout) = snapshot.workspace_snapshot.layout else {
            return Vec::new();
        };
        if layout.views.len() <= 1 {
            return Vec::new();
        }
        let mut targets = Vec::new();
        collect_targets(
            self,
            &layout,
            layout.root_node_id,
            LayoutBounds {
                left: 0,
                top: 0,
                width: self.terminal_size.cols as usize,
                height: self.terminal_size.rows as usize,
            },
            active_view_id,
            &mut targets,
        );
        targets
    }

    pub(super) fn layout_positioned_panes(&self) -> Vec<TerminalPaneLayout> {
        let mut panes = Vec::new();
        let cell_width = self.render_metrics.cell_size.width.max(0) as usize;
        let cell_height = self.render_metrics.cell_size.height.max(0) as usize;
        let mut next_index = 0;
        for target in self.layout_render_targets() {
            let Some(render_state) = crate::chatminal_runtime::desktop_render_state_for_session(
                self.window_id as DesktopWindowId,
                &target.session_id,
            ) else {
                continue;
            };
            let source_size = render_state.terminal_size;
            for render_pane in render_state.panes {
                let pane_id = render_pane.terminal_handle.as_u64();
                // Check for pane-level overlay first (e.g. launcher/prompt on this pane)
                if let Some(overlay) = self.terminal_ui_state(pane_id).overlay.as_ref() {
                    panes.push(TerminalPaneLayout {
                        index: next_index,
                        is_active: target.is_active,
                        is_zoomed: false,
                        left: target.left,
                        top: target.top,
                        width: target.width,
                        pixel_width: target.width * cell_width,
                        height: target.height,
                        pixel_height: target.height * cell_height,
                        pane: Arc::clone(&overlay.pane),
                    });
                    next_index += 1;
                    continue;
                }
                let Some(pane) = crate::chatminal_runtime::terminal_handle_arc_by_public_id(pane_id)
                else {
                    continue;
                };
                let geometry = map_render_pane_geometry(
                    &target,
                    source_size,
                    &render_pane,
                    cell_width,
                    cell_height,
                );
                panes.push(TerminalPaneLayout {
                    index: next_index,
                    is_active: geometry.is_active,
                    is_zoomed: geometry.is_zoomed,
                    left: geometry.left,
                    top: geometry.top,
                    width: geometry.width,
                    pixel_width: geometry.pixel_width,
                    height: geometry.height,
                    pixel_height: geometry.pixel_height,
                    pane,
                });
                next_index += 1;
            }
        }
        panes
    }

    pub(super) fn layout_positioned_splits(&mut self) -> Vec<TerminalSplit> {
        // Workspace-level splits from WorkspaceLayoutState (session-level splits are always empty
        // after Phase 03: splits = [] invariant).
        self.layout_workspace_splits()
    }

    fn layout_workspace_splits(&self) -> Vec<TerminalSplit> {
        self.workspace_split_targets()
            .into_iter()
            .map(|target| target.split)
            .collect()
    }

    pub(super) fn render_capability_for_layout_split(
        &self,
        _target_split: TerminalSplit,
    ) -> Option<u64> {
        // After Phase 03: session-level splits are always empty (splits = [] invariant).
        // Workspace-level splits are handled by resize_workspace_layout_split directly.
        None
    }

    pub(super) fn resize_workspace_layout_split(
        &self,
        target_split: TerminalSplit,
        delta: isize,
    ) -> Option<TerminalSplit> {
        let target = self.find_workspace_split_target(target_split)?;
        let usable = match target.split.direction {
            TerminalSplitDirection::Horizontal => target.bounds.width.saturating_sub(1),
            TerminalSplitDirection::Vertical => target.bounds.height.saturating_sub(1),
        };
        if usable <= 1 {
            return None;
        }

        let current_first = match target.split.direction {
            TerminalSplitDirection::Horizontal => {
                target.split.left.saturating_sub(target.bounds.left)
            }
            TerminalSplitDirection::Vertical => {
                target.split.top.saturating_sub(target.bounds.top)
            }
        };
        let max_first = usable.saturating_sub(1);
        let next_first = current_first
            .saturating_add_signed(delta)
            .clamp(1, max_first);
        let ratio = ((next_first as u64) * 1000 / (usable as u64)) as u16;
        crate::chatminal_runtime::desktop_resize_layout_split(
            self.window_id as DesktopWindowId,
            target.node_id,
            ratio,
        )?;
        self.workspace_split_targets()
            .into_iter()
            .find(|candidate| candidate.node_id == target.node_id)
            .map(|candidate| candidate.split)
    }

    fn workspace_split_targets(&self) -> Vec<WorkspaceSplitTarget> {
        if !self.chatminal_sidebar.is_enabled() {
            return Vec::new();
        }
        let Ok(snapshot) = crate::chatminal_runtime::desktop_session_window_snapshot(
            self.window_id as DesktopWindowId,
        ) else {
            return Vec::new();
        };
        let Some(layout) = snapshot.workspace_snapshot.layout else {
            return Vec::new();
        };
        if layout.views.len() <= 1 {
            return Vec::new();
        }

        let mut splits = Vec::new();
        collect_workspace_splits(
            &layout,
            layout.root_node_id,
            LayoutBounds {
                left: 0,
                top: 0,
                width: self.terminal_size.cols as usize,
                height: self.terminal_size.rows as usize,
            },
            &mut splits,
        );
        splits
    }

    fn find_workspace_split_target(
        &self,
        target_split: TerminalSplit,
    ) -> Option<WorkspaceSplitTarget> {
        self.workspace_split_targets()
            .into_iter()
            .find(|candidate| {
                candidate.split.direction == target_split.direction
                    && candidate.split.left == target_split.left
                    && candidate.split.top == target_split.top
                    && candidate.split.size == target_split.size
                    && candidate.split.index == target_split.index
            })
    }
}

fn collect_targets(
    term_window: &crate::TermWindow,
    layout: &WorkspaceLayoutState,
    node_id: WorkspaceNodeId,
    bounds: LayoutBounds,
    active_view_id: Option<SessionViewId>,
    targets: &mut Vec<LayoutRenderTarget>,
) {
    if bounds.width == 0 || bounds.height == 0 {
        return;
    }
    let Some(node) = layout.nodes.iter().find(|node| node.node_id == node_id) else {
        return;
    };
    match node.kind {
        WorkspaceLayoutNodeKind::View { view_id } => {
            let Some(view) = layout.view(view_id) else {
                return;
            };
            targets.push(LayoutRenderTarget {
                session_id: view.session_id.clone(),
                left: bounds.left,
                top: bounds.top,
                width: bounds.width.max(1),
                height: bounds.height.max(1),
                is_active: active_view_id == Some(view_id),
            });
        }
        WorkspaceLayoutNodeKind::Split {
            axis,
            first,
            second,
            ratio,
        } => {
            let (first_bounds, second_bounds) = split_bounds(bounds, axis, ratio);
            collect_targets(
                term_window,
                layout,
                first,
                first_bounds,
                active_view_id,
                targets,
            );
            collect_targets(
                term_window,
                layout,
                second,
                second_bounds,
                active_view_id,
                targets,
            );
        }
    }
}

fn collect_workspace_splits(
    layout: &WorkspaceLayoutState,
    node_id: WorkspaceNodeId,
    bounds: LayoutBounds,
    splits: &mut Vec<WorkspaceSplitTarget>,
) {
    if bounds.width == 0 || bounds.height == 0 {
        return;
    }
    let Some(node) = layout.nodes.iter().find(|node| node.node_id == node_id) else {
        return;
    };
    let WorkspaceLayoutNodeKind::Split {
        axis,
        first,
        second,
        ratio,
    } = node.kind
    else {
        return;
    };

    let direction = match axis {
        WorkspaceSplitAxis::Horizontal => {
            TerminalSplitDirection::Horizontal
        }
        WorkspaceSplitAxis::Vertical => TerminalSplitDirection::Vertical,
    };
    let (first_bounds, second_bounds) = split_bounds(bounds, axis, ratio);
    let (left, top, size) = match direction {
        TerminalSplitDirection::Horizontal => (
            second_bounds.left.saturating_sub(1),
            bounds.top,
            bounds.height,
        ),
        TerminalSplitDirection::Vertical => (
            bounds.left,
            second_bounds.top.saturating_sub(1),
            bounds.width,
        ),
    };
    splits.push(WorkspaceSplitTarget {
        node_id,
        split: TerminalSplit {
            index: WORKSPACE_SPLIT_INDEX_BASE + splits.len(),
            direction,
            left,
            top,
            size,
        },
        bounds,
    });
    collect_workspace_splits(layout, first, first_bounds, splits);
    collect_workspace_splits(layout, second, second_bounds, splits);
}

fn split_bounds(
    bounds: LayoutBounds,
    axis: WorkspaceSplitAxis,
    ratio: u16,
) -> (LayoutBounds, LayoutBounds) {
    match axis {
        WorkspaceSplitAxis::Horizontal => {
            let divider = usize::from(bounds.width > 1);
            let usable = bounds.width.saturating_sub(divider);
            let first = split_length(usable, ratio);
            let second = usable.saturating_sub(first);
            (
                LayoutBounds {
                    width: first,
                    ..bounds
                },
                LayoutBounds {
                    left: bounds.left + first + divider,
                    width: second,
                    ..bounds
                },
            )
        }
        WorkspaceSplitAxis::Vertical => {
            let divider = usize::from(bounds.height > 1);
            let usable = bounds.height.saturating_sub(divider);
            let first = split_length(usable, ratio);
            let second = usable.saturating_sub(first);
            (
                LayoutBounds {
                    height: first,
                    ..bounds
                },
                LayoutBounds {
                    top: bounds.top + first + divider,
                    height: second,
                    ..bounds
                },
            )
        }
    }
}

fn split_length(usable: usize, ratio: u16) -> usize {
    if usable <= 1 {
        return usable;
    }
    let scaled = ((usable as u64) * (ratio as u64)) / 1000;
    (scaled as usize).clamp(1, usable.saturating_sub(1))
}

fn map_offset(offset: usize, source_total: usize, target_total: usize) -> usize {
    offset.saturating_mul(target_total.max(1)) / source_total.max(1)
}

fn map_span(offset: usize, span: usize, source_total: usize, target_total: usize) -> usize {
    let start = map_offset(offset, source_total, target_total);
    let end = map_offset(offset.saturating_add(span), source_total, target_total);
    end.saturating_sub(start).max(1)
}

fn map_render_pane_geometry(
    target: &LayoutRenderTarget,
    source_size: engine_term::TerminalSize,
    render_pane: &ChatminalRenderPane,
    cell_width: usize,
    cell_height: usize,
) -> RenderPaneGeometry {
    let source_cols = source_size.cols.max(1);
    let source_rows = source_size.rows.max(1);
    let width = map_span(render_pane.left, render_pane.width, source_cols, target.width);
    let height = map_span(render_pane.top, render_pane.height, source_rows, target.height);
    RenderPaneGeometry {
        left: target.left + map_offset(render_pane.left, source_cols, target.width),
        top: target.top + map_offset(render_pane.top, source_rows, target.height),
        width,
        height,
        pixel_width: width * cell_width,
        pixel_height: height * cell_height,
        is_active: target.is_active && render_pane.is_active,
        is_zoomed: render_pane.is_zoomed,
    }
}

fn map_render_split_geometry(
    target: &LayoutRenderTarget,
    source_size: engine_term::TerminalSize,
    split: &ChatminalRenderSplit,
) -> RenderSplitGeometry {
    let source_cols = source_size.cols.max(1);
    let source_rows = source_size.rows.max(1);
    let direction = match split.axis {
        ChatminalRenderSplitAxis::Horizontal => TerminalSplitDirection::Horizontal,
        ChatminalRenderSplitAxis::Vertical => TerminalSplitDirection::Vertical,
    };
    let (left, top, size) = match direction {
        TerminalSplitDirection::Horizontal => (
            target.left + map_offset(split.left, source_cols, target.width),
            target.top + map_offset(split.top, source_rows, target.height),
            map_span(split.top, split.size, source_rows, target.height),
        ),
        TerminalSplitDirection::Vertical => (
            target.left + map_offset(split.left, source_cols, target.width),
            target.top + map_offset(split.top, source_rows, target.height),
            map_span(split.left, split.size, source_cols, target.width),
        ),
    };
    RenderSplitGeometry {
        direction,
        left,
        top,
        size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chatminal_runtime::{SessionTerminalHandle, TerminalInstanceId};
    use engine_term::TerminalSize;

    fn sample_target() -> LayoutRenderTarget {
        LayoutRenderTarget {
            session_id: "session-a".to_string(),
            left: 10,
            top: 4,
            width: 100,
            height: 50,
            is_active: true,
        }
    }

    fn terminal_size(cols: usize, rows: usize) -> TerminalSize {
        TerminalSize {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 96,
        }
    }

    #[test]
    fn maps_render_pane_geometry_into_layout_target() {
        let geometry = map_render_pane_geometry(
            &sample_target(),
            terminal_size(200, 100),
            &ChatminalRenderPane {
                terminal_handle: SessionTerminalHandle::new(7),
                terminal_instance_id: TerminalInstanceId::new(9),
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 50,
                top: 25,
                width: 100,
                pixel_width: 0,
                height: 50,
                pixel_height: 0,
            },
            8,
            16,
        );

        assert_eq!(
            geometry,
            RenderPaneGeometry {
                left: 35,
                top: 16,
                width: 50,
                height: 25,
                pixel_width: 400,
                pixel_height: 400,
                is_active: true,
                is_zoomed: false,
            }
        );
    }

    #[test]
    fn maps_render_split_geometry_for_scaled_layout() {
        let geometry = map_render_split_geometry(
            &sample_target(),
            terminal_size(200, 100),
            &ChatminalRenderSplit {
                index: 3,
                axis: ChatminalRenderSplitAxis::Vertical,
                left: 100,
                top: 0,
                size: 100,
            },
        );

        assert_eq!(
            geometry,
            RenderSplitGeometry {
                direction: TerminalSplitDirection::Vertical,
                left: 60,
                top: 4,
                size: 50,
            }
        );
    }

    #[test]
    fn inactive_layout_target_keeps_render_pane_inactive() {
        let mut target = sample_target();
        target.is_active = false;
        let geometry = map_render_pane_geometry(
            &target,
            terminal_size(80, 24),
            &ChatminalRenderPane {
                terminal_handle: SessionTerminalHandle::new(3),
                terminal_instance_id: TerminalInstanceId::new(4),
                index: 0,
                is_active: true,
                is_zoomed: true,
                left: 0,
                top: 0,
                width: 80,
                pixel_width: 0,
                height: 24,
                pixel_height: 0,
            },
            10,
            20,
        );

        assert!(!geometry.is_active);
        assert!(geometry.is_zoomed);
    }

    #[test]
    fn split_length_clamps_for_small_targets() {
        assert_eq!(split_length(0, 500), 0);
        assert_eq!(split_length(1, 500), 1);
        assert_eq!(split_length(2, 0), 1);
        assert_eq!(split_length(2, 1000), 1);
        assert_eq!(split_length(10, 900), 9);
    }
}
