# Phase 01 - Layout Primitives and Chrome Geometry

## Context Links
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
- `apps/chatminal-desktop/src/termwindow/resize.rs`

## Overview
- Priority: P1 | Status: done | Effort: 1.5d
- Unify geometry contract for sidebar/footer/content/overlay bounds. Absorbs boundary freeze scope.

## Shell Zone Definitions (from boundary freeze)
- **Safe zone**: render tree, mouse hit-test, chrome metrics, overlay visuals, session bar visuals
- **Adapter seam**: helpers resolving render target/session/overlay (read-only)
- **Black box**: terminal core, parser/state, PTY/store ownership

## No-Touch List
- `crates/chatminal-terminal-core/**`
- `WorkspaceLayoutState` ownership, runtime persistence format
- `workspace_store.rs` = READ-ONLY geometry consumer — do NOT mutate WorkspaceLayoutState ownership
- Terminal size semantics inside core
- `desktop_termwindow_render_pane.rs` may only consume shared shell geometry/clip bounds; do NOT change pane paint semantics, terminal content ordering, or terminal viewport behavior

## Objective
- Single source of truth for chrome bounds (sidebar, header, footer, content, overlay)
- Eliminate duplicate padding/bounds calculations scattered across render + mouse paths

## Files Likely Touched
- Modify: `desktop_termwindow_render_mod.rs`, `desktop_termwindow_layout_render.rs`
- Modify: `desktop_termwindow_render_pane.rs` (read-only geometry consumer only), `termwindow/resize.rs`
- Modify: `termwindow/mod.rs`, `desktop_termwindow_types.rs`
- Create: geometry helper module under `apps/chatminal-desktop/src/` if needed

## Implementation Steps
1. Inventory all functions computing chrome/padding/bounds across render + mouse files
2. Define shared struct: `ShellBounds { sidebar, header, footer, content, overlay_scope }`
3. Route render path through shared bounds
4. Route hit-test/mouse path through same bounds
5. Verify: session bar top/bottom, sidebar on/off, multi-split layout all render correctly

## Success Criteria
- One struct/fn as single source for all shell region bounds
- No magic numbers for chrome/footer/sidebar offsets in main render path
- Any `desktop_termwindow_render_pane.rs` diff is limited to consuming `ShellBounds.content`/clip geometry only
- `cargo check -p chatminal-desktop` passes
- Manual smoke: session bar top/bottom + sidebar on/off render correctly
- No files changed in `crates/chatminal-terminal-core/**`

## Risk Assessment
- Geometry change may regress click/hit-test or split drag bounds
- Mitigation: same helper for mouse and render guarantees consistency
