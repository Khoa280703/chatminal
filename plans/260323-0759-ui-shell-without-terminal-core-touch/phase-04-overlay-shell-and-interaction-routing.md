# Phase 04 - Overlay Shell and Interaction Routing

## Context Links
- `apps/chatminal-desktop/src/overlay/mod.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `apps/chatminal-desktop/src/overlay/quickselect.rs`
- `apps/chatminal-desktop/src/overlay/confirm.rs`
- `apps/chatminal-desktop/src/overlay/prompt.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`

## Overview
- Priority: P2 | Status: done | Effort: 1d
- Unify overlay chrome (padding, close, focus) across all overlay types; clean cancel path

## No-Touch
- Overlay terminal internals, runtime overlay protocol, pane IO semantics
- `desktop_termwindow_actions_items.rs`, `desktop_termwindow_state_helpers.rs` — behavior routing files, keep unchanged

## Objective
- All overlay types share same padding/close affordance/focus visual treatment
- Cancel path fully cleans up (no stale overlay state)
- Overlay sizing respects Phase 01 content bounds

## Files Likely Touched
- Modify: `overlay/mod.rs`, `overlay/launcher.rs`, `overlay/quickselect.rs`
- Modify: `overlay/confirm.rs`, `overlay/prompt.rs`
- Modify: `desktop_termwindow_host_runtime_helpers.rs`

## Implementation Steps
1. Audit overlay family chrome differences (padding, close, focus, sizing)
2. Extract shared overlay shell contract (bounds from Phase 01, common chrome tokens)
3. Align resize/cancel/focus behavior across launcher/quickselect/confirm/prompt
4. Verify overlay-active path does not break mouse/key routing to terminal content

## Success Criteria
- All overlay types (launcher, quickselect, confirm, prompt) use same padding and close affordance
- Cancel fully cleans up: no stale overlay handles, no ghost render
- Overlay respects content bounds from Phase 01 geometry contract
- `cargo check -p chatminal-desktop` passes
- No files changed in `crates/chatminal-terminal-core/**`

## Risk Assessment
- Visual refactor may accidentally touch behavior path of launcher/quickselect
- Mitigation: keep behavior logic unchanged, wrap through shared shell helpers only

## Dependencies
- Phase 01 geometry contract (overlay scope bounds)
