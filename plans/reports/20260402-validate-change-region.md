# QA Report - 2026-04-02 - validate-change-region

## Scope
- `apps/chatminal-desktop/src/frontend.rs`
- `apps/chatminal-desktop/src/main.rs`
- `apps/chatminal-desktop/src/termwindow/palette.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_event_helpers.rs`
- `crates/chatminal-host-runtime/src/lib.rs`
- `crates/chatminal-host-runtime/src/window.rs`
- `crates/chatminal-host-runtime/src/spawn_target.rs`
- `crates/chatminal-host-runtime/src/tab.rs`
- `crates/chatminal-host-runtime/src/localpane.rs`
- `crates/chatminal-host-runtime/src/activity.rs`
- `crates/chatminal-host-runtime/src/termwiztermtab.rs`
- `crates/chatminal-lua-bridge/src/lib.rs`
- `crates/chatminal-lua-bridge/src/leaf.rs`
- `crates/chatminal-lua-bridge/src/session.rs`
- `crates/chatminal-lua-bridge/src/window.rs`

## Test Results Overview
- `cargo check -p chatminal-desktop -p chatminal-host-runtime -p chatminal-lua-bridge`
- `cargo test -p chatminal-host-runtime`
- `cargo test -p chatminal-lua-bridge`
- `cargo test -p chatminal-desktop`

Totals:
- Tests run: 91
- Passed: 91
- Failed: 0
- Skipped: 0

## Coverage / Evidence
- Direct unit tests found in changed scope:
  - `apps/chatminal-desktop/src/termwindow/palette.rs`
  - `crates/chatminal-host-runtime/src/tab.rs`
- No direct tests in the changed files for:
  - `apps/chatminal-desktop/src/frontend.rs`
  - `apps/chatminal-desktop/src/main.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_event_helpers.rs`
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `crates/chatminal-host-runtime/src/window.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
  - `crates/chatminal-host-runtime/src/localpane.rs`
  - `crates/chatminal-host-runtime/src/activity.rs`
  - `crates/chatminal-host-runtime/src/termwiztermtab.rs`
  - `crates/chatminal-lua-bridge/src/lib.rs`
  - `crates/chatminal-lua-bridge/src/leaf.rs`
  - `crates/chatminal-lua-bridge/src/session.rs`
  - `crates/chatminal-lua-bridge/src/window.rs`

Coverage metrics:
- Line coverage: not measured
- Branch coverage: not measured
- Function coverage: not measured

## Build Status
- `cargo check` status: success
- `cargo test` status: success
- Build warnings: none observed in command output

## Failed Tests
- None

## Performance Metrics
- `cargo check`: 11.32s
- `cargo test -p chatminal-host-runtime`: 2.88s
- `cargo test -p chatminal-lua-bridge`: 0.16s profile reuse, no tests
- `cargo test -p chatminal-desktop`: 12.43s

## Critical Issues
- None blocking from the current evidence set.

## Regression Risk
- Medium:
  - large glue-layer changes across desktop, host-runtime, and lua bridge
  - `chatminal-lua-bridge` has no unit tests, so validation there is compile-only
  - event-helper and app entrypoint changes are covered only indirectly by package tests, not by targeted assertions

## Recommendations
- Add direct tests for `desktop_termwindow_event_helpers.rs`
- Add direct tests for `frontend.rs` and `main.rs` behavior that changed
- Add unit/integration tests for `chatminal-lua-bridge` public bindings and edge cases
- If behavior changed in runtime/window/session glue, add focused regression tests around spawn/focus/close paths

## Next Steps
1. Add targeted tests for the changed glue layers
2. Run the package tests again after those tests exist
3. If GUI-specific behavior changed, add a small smoke/UI check on desktop startup and session switching

## Unresolved Questions
- None
