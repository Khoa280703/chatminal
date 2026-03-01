## Code Review Summary

### Scope
- Files: `src/config.rs`, `src/ui/theme.rs`, `src/app.rs`, `src/session/pty_worker.rs`, `src/session/grid.rs`, `src/session/tests.rs`, `src/session/manager.rs`
- LOC reviewed: ~1,424 (focused files)
- Focus: post-fix readiness review for `cook --auto`
- Scout findings: no git history (`HEAD~1` unavailable), scoped by recent-touch files + dependency tracing

### Overall Assessment
- 3 finding cũ status:
  - Hardcoded metrics: fixed at runtime path (`metrics_for_font` wired in boot/resize).
  - ESC M reverse index: logic implemented correctly in parser dispatch.
  - Clamp config bounds: numeric clamps in place for scrollback/font/sidebar.
- Main blocker now is event-delivery reliability under load (dropped update/exited events).

### Critical Issues
- None.

### High Priority
- Event loss risk due non-blocking send and ignored send errors.
  - File: `src/session/pty_worker.rs:33`, `src/session/pty_worker.rs:45`, `src/session/pty_worker.rs:107`, `src/app.rs:40`
  - Problem: PTY reader uses `try_send` for both `SessionEvent::Update` and `SessionEvent::Exited` on bounded channel size 4; send failures are ignored.
  - Impact:
    - Can drop final terminal snapshot if queue is full when output stops.
    - Can drop `Exited` signal, leaving dead session entry in sidebar until manual close.
    - Can desync scroll anchor (`lines_added` reset before successful send).
  - Recommendation: use blocking/backpressure strategy for terminal-thread -> UI event handoff (`blocking_send`) or retry-on-full for `Exited` with guaranteed delivery semantics.

### Medium Priority
- ESC M path still missing parser-level regression test.
  - File: `src/session/tests.rs:73`
  - Problem: current test validates `scroll_down()` primitive only, not `vte::Parser` -> `esc_dispatch('M')` integration.
  - Impact: future parser refactor can break reverse-index behavior without test failure.
  - Recommendation: add integration test feeding `\x1bM` into parser performer path.

- Config parse failure drops all valid fields instead of partial fallback.
  - File: `src/config.rs:98`
  - Problem: `toml::from_str(...).unwrap_or_default()` replaces whole config on single parse/type error.
  - Impact: one malformed field silently resets unrelated valid settings.
  - Recommendation: log parse error; parse partial table with per-field fallback.

### Low Priority
- None.

### Edge Cases Found by Scout
- Affected dependents:
  - `load_config()` -> `AppState::boot()` -> `SessionManager::new()` data path.
  - `metrics_for_font()` consumed by runtime layout math (`handle_resize`).
  - `SessionEvent` flow crosses OS thread -> tokio channel -> Iced subscription.
- Boundary checks verified:
  - `scrollback_lines`, `font_size`, `sidebar_width` clamp bounds active.
  - `TerminalGrid::new` clamp for scrollback max active.
- Async/state risk:
  - Bounded channel + ignored `try_send` errors can lose state transitions.

### Positive Observations
- Numeric clamps are explicit and covered by unit tests.
- Runtime metrics no longer hardcoded literals in app sizing path.
- Shell path validation in session manager is strong (`/etc/shells` + canonicalize + executable bit).
- Full verification gate currently green: `cargo test`, `cargo clippy -- -D warnings`, `cargo build --release`.

### Recommended Actions
1. Make `SessionEvent::Exited` delivery reliable even under full queue (must-not-drop).
2. Avoid dropping latest `Update` snapshot silently (retry/coalesce strategy with guaranteed last-state delivery).
3. Add ESC M parser-level regression test.
4. Improve config parse fallback behavior + error logging.

### Metrics
- Type Coverage: N/A (Rust static typing; no separate metric job)
- Test Coverage: N/A (coverage tool not run)
- Linting Issues: 0 (`cargo clippy -- -D warnings`)

### Task Completeness
- Plan TODO check in `plans/260301-1521-chatminal-messenger-terminal/*` still has open boxes, including:
  - `phase-07`: `pty_worker.rs` EOF path still expects `blocking_send` (matches high finding above).
  - Multiple manual verification checkboxes remain open (`phase-03`, `phase-04`, `phase-05`, `phase-06`, `phase-07`).

### Gate Verdict
- Overall score: **9.2 / 10**
- Auto-approve gate (`>= 9.5` and `0 critical`): **FAIL**
- Reason: 0 critical, but 1 high-priority reliability issue remains.

### Unresolved Questions
- Có chấp nhận semantics lossy-update cho terminal stream trong MVP không, hay bắt buộc guaranteed-delivery cho `Exited` + latest snapshot?
