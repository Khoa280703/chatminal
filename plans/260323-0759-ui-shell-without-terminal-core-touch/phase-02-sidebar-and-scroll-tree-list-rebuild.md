# Phase 02 - Sidebar and Scroll Tree List Rebuild

## Context Links
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_event_helpers.rs`
- `apps/chatminal-desktop/src/termwindow/box_model.rs`

## Overview
- Priority: P1 | Status: done | Effort: 2.5d
- Rebuild sidebar as proper scroll-tree-list with stable hit-test, smooth scroll, clear visual hierarchy

## No-Touch
- Session data source contract, runtime sidebar subscription protocol, terminal pane semantics

## Objective
- Sidebar profile/session tree with pixel-scroll clipping, clipped hit-test, expand/collapse
- Separate projection, scroll math, and visual rendering into distinct concerns
- **WARNING: Do NOT use row virtualization — use pixel scroll + clip**

## Files Likely Touched
- Modify: `chatminal_sidebar/mod.rs`, `termwindow/render/chatminal_sidebar.rs`
- Modify: `desktop_termwindow_mouseevent.rs`, `desktop_termwindow_event_helpers.rs`
- Modify: `termwindow/box_model.rs`

## Scroll Model (pixel-based)
- `scroll_offset_px: f32` — pixel-based offset, NOT row index
- Full tree rendered, translated by `-scroll_offset_px`, then clip rect cuts
- Hit-test: `mouse_y + scroll_offset_px` → tree coordinate space
- NO "visible row range" concept — all rows exist, only clipped

## Implementation Steps
1. Extract projection layer: `SidebarSnapshot -> SidebarTreeRowView[]`
2. Implement pixel-scroll model: `scroll_offset_px: f32`, clamp to `[0, total_height - viewport_height]`, clip rect
3. Rework click/wheel: `mouse_y + scroll_offset_px` → tree coordinate space (same bounds as render)
4. Add scrollbar thumb rendering based on viewport ratio
5. Polish visual hierarchy: indentation, active/hover/error states, density

## Success Criteria
- Scroll 50+ sessions without ghost hitbox or clipping artifacts
- Click targets match visible rows after scroll (no off-by-one)
- Expand/collapse preserves scroll position within 1 row tolerance
- Snapshot refresh does not cause scroll jump
- `cargo check -p chatminal-desktop` passes
- No files changed in `crates/chatminal-terminal-core/**`

## Risk Assessment
- Deep coupling between render and hit-test in WezTerm pipeline may require careful refactoring
- Mitigation: keep `SidebarSnapshot` read-only, only persist scroll/expansion state in existing shell state
- Estimate padded to 2.5d to account for render pipeline complexity

## Dependencies
- Phase 01 geometry contract (shared bounds for sidebar region)
