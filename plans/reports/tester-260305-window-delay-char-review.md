# tester-260305-window-delay-char-review

Date: 2026-03-05
Scope: quick QA for `apps/chatminal-app` patch, focus input path + render behavior.

## Test Results Overview
- Command: `cargo check --manifest-path apps/chatminal-app/Cargo.toml`
- Result: PASS
- Time: real `0.28s` (user `0.19s`, sys `0.09s`)

- Command: `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
- Result: PASS
- Totals: `61 passed`, `0 failed`, `0 ignored`, `0 measured`, `0 filtered out`
- Test runtime: suite finished `0.02s`, command real `0.29s`

## Coverage Metrics
- Not generated in this quick pass (no `cargo llvm-cov`/coverage runner in scope).

## Failed Tests
- None.

## Performance Metrics
- Build/test command runtime stable (< 0.3s each).
- No slow test surfaced from this run.

## Build Status
- Success. No compile errors. No test failures.

## Focused Regression Review

### 1) Input path (`apps/chatminal-app/src/window/native_window_wezterm_actions.rs`)
- Input batching and send path compiles clean, tests pass.
- Existing protections good:
  - non-legacy path blocks shortcut duplicates via mapper/filter chain.
  - legacy path suppresses duplicate plain-char key payload when text-like event already present (`is_legacy_plain_text_key_payload`).
- Risk note (medium, behavior):
  - `refresh_cached_terminal_text()` runs before `handle_terminal_input_events()` in frame update flow.
  - `send_terminal_payload()` sets `render_dirty = true`, then next frame may clear dirty before PTY echo arrives.
  - If PTY echo lands after that active frame, UI can wait up to idle repaint interval (`UI_IDLE_REPAINT_MS = 120`) before visible char update.
  - References:
    - `apps/chatminal-app/src/window/native_window_wezterm.rs:135-137`
    - `apps/chatminal-app/src/window/native_window_wezterm.rs:166-170`
    - `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:345-389`

### 2) Render text behavior (`terminal_wezterm_core.rs` + `native_window_wezterm.rs`)
- Observed patch in `terminal_wezterm_core.rs`:
  - from trimming trailing spaces per line to preserving full line text.
  - change at `apps/chatminal-app/src/terminal_wezterm_core.rs:106-110`.
- Functional impact:
  - better fidelity for intentional trailing spaces.
- Risk note (low-medium, perf/memory):
  - if wezterm screen lines include right-padding spaces to terminal width, `visible_text` size increases significantly.
  - native window clones and feeds this string into `TextEdit::multiline` each refresh (`apps/chatminal-app/src/window/native_window_wezterm.rs:149-157`).
  - may increase render cost on large grid/high scroll churn.

## Critical Issues
- None blocking found from compile/test + static review.

## Recommendations
1. Add a targeted regression test for "input visible latency" in window flow (mock delayed PTY echo around repaint cadence).
2. Add a unit test for trailing-space fidelity in `terminal_wezterm_core_tests.rs` to lock expected behavior explicitly.
3. If delay reproduces manually, consider keeping active repaint for short grace window after successful input write (e.g. 100-200ms), or refresh text after input processing step.

## Next Steps
1. Run `make smoke-window` (or equivalent xvfb smoke) to validate real window typing responsiveness.
2. Optionally run a focused benchmark capturing frame/update latency under burst input.

## Unresolved Questions
- Does current eframe integration wake repaint immediately on daemon event availability in all target platforms, or only by scheduled repaint/input events?
- With current wezterm line API, does `line.as_str()` always include right-padding spaces up to `cols` for each row?
