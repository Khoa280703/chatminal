# Phase 04 - TermWindow Render Cutover To Session View Layout

## Overview
- Priority: P0
- Status: completed
- Brief: termwindow render theo `layout_tree` mới; mỗi ô render một session view thay vì coi một session là một surface split-tree.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/termwindow/mod.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/*`
- Modify: `apps/chatminal-desktop/src/overlay/*`

## Success Criteria
- Multi-view render không cần public `leaf` semantics.
- Pane selection/action layer không còn giả định `pos.index` là global trong window.
- Divider của `layout_tree` được render ở window-level và resize được qua workspace layout ratio.
