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
- Keep overlays terminal-based, but unify host-side sizing/cancel/focus routing so overlay lifecycle is consistent with the shell

## No-Touch
- Overlay terminal internals, runtime overlay protocol, pane IO semantics
- `desktop_termwindow_actions_items.rs`, `desktop_termwindow_state_helpers.rs` — behavior routing files, keep unchanged

## Objective
- All overlay types share the same host-side lifecycle contract while remaining terminal-rendered
- Cancel path fully cleans up (no stale overlay state)
- Overlay sizing/routing respects Phase 01 content bounds where host runtime chooses overlay scope size

## Files Likely Touched
- Modify: `overlay/mod.rs`, `overlay/launcher.rs`, `overlay/quickselect.rs`
- Modify: `overlay/confirm.rs`, `overlay/prompt.rs`
- Modify: `desktop_termwindow_host_runtime_helpers.rs`

## Implementation Steps
1. Audit overlay family lifecycle differences (spawn scope, cancel, focus, sizing)
2. Extract shared host-side overlay contract (scope sizing from Phase 01 + shared cancel/focus routing)
3. Keep overlay visuals terminal-native; only align host/runtime behavior across launcher/quickselect/confirm/prompt
4. Verify overlay-active path does not break mouse/key routing to terminal content

## Success Criteria
- Overlay types keep terminal-native visuals; no fake shared chrome layer added on top
- Cancel fully cleans up: no stale overlay handles, no ghost render
- Host-side overlay sizing/routing respects Phase 01 geometry contract where applicable
- `cargo check -p chatminal-desktop` passes
- No files changed in `crates/chatminal-terminal-core/**`

## Risk Assessment
- Host-side cleanup may accidentally touch behavior path of launcher/quickselect
- Mitigation: keep overlay rendering logic unchanged, limit changes to host-side helpers only

## Dependencies
- Phase 01 geometry contract (overlay scope bounds)
