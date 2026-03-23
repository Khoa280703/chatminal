## Context Links
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/termwindow/box_model.rs`

## Overview
- Priority: P1
- Current status: pending
- Brief description: normalize shell spacing, chrome heights, sidebar width, footer offsets, and shared layout primitives without touching terminal content logic.
- Explicit boundary: không đụng `crates/chatminal-terminal-core/**`; không đổi terminal cell layout/state math beyond shell padding/chrome offsets.

## Objective
- Tạo một set primitive cho shell chrome spacing và bounds.
- Giảm duplicated math cho sidebar width, top chrome, footer height, available content rect.

## Scope
- Consolidate shell-only geometry constants and helper functions.
- Keep terminal viewport contract intact: same terminal draw pipeline, only adjusted shell padding inputs.

## Key Insights
- `padding_left_top`, `chatminal_terminal_footer_height`, sidebar width, and layout render bounds are spread across multiple files.
- Shell polish will stay brittle until these numbers are normalized.

## Requirements
- Functional: one consistent shell bounds model shared by sidebar, footer, overlays, and layout splits.
- Non-functional: no extra frame cost, no new render pass, no change to terminal text metrics.

## Architecture
- Centralize shell geometry helpers in existing desktop modules.
- Keep `DesktopWorkspaceLayoutStore` and render-state consumers read-only from shell perspective.
- Let layout primitives feed render modules, not runtime.

## Files Likely Touched
- Modify:
  - `apps/chatminal-desktop/src/termwindow/mod.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
  - `apps/chatminal-desktop/src/termwindow/box_model.rs`
- Create: none preferred
- Delete: none

## Implementation Steps
1. Inventory duplicated shell geometry calculations.
2. Introduce shared helpers for sidebar width, top chrome bounds, footer bounds, content viewport bounds.
3. Route sidebar/footer/layout render code through those helpers.
4. Verify hit-testing uses same bounds as rendering.

## Todo List
- [ ] Geometry duplication reduced
- [ ] Shared shell bounds helper in place
- [ ] Sidebar/footer/layout use same bounds source
- [ ] Hit-test/render parity verified

## Success Criteria
- Same shell dimensions apply across render and mouse handling.
- No visual drift between content rect and interactive rect.
- No terminal rendering regression outside changed padding/chrome area.

## Risk Assessment
- Risk: off-by-one or DPI drift causes click mismatch.
- Mitigation: validate bounds on multiple window widths and DPI values.

## Security Considerations
- No change to execution/runtime capabilities.
- No new IPC or command path.

## Next Steps
- Use shared primitives to rebuild sidebar/tree rendering.
- Unresolved questions: none.
