## Code Review Summary

### Scope
- Files: `src/session/pty_worker.rs`, `src/app.rs`, `src/config.rs`, `src/ui/theme.rs`, `src/session/grid.rs`, `src/session/manager.rs`, `src/session/tests.rs`
- LOC reviewed: ~1.5k (focused review)
- Focus: re-review sau fix High + integration tests cho `cook --auto`
- Scout findings: repo chưa có commit (`HEAD~1` unavailable), scout theo dependency graph + targeted grep các vùng liên quan findings

### Overall Assessment
- 3 finding cũ đã được xử lý đúng hướng và có evidence test/usage rõ ràng.
- Finding High queue-full trước đó đã được giảm từ High xuống non-blocking (đã đảm bảo delivery cho `Exited`, và retry behavior cho `Update`).
- Còn 1 edge-case Medium liên quan flush cuối khi EOF ngay sau lần queue full.

### Critical Issues
- None.

### High Priority
- None.

### Medium Priority
- Có thể mất snapshot cuối khi EOF xảy ra ngay sau lần `Update` bị `Full`.
  - File: `src/session/pty_worker.rs:33`, `src/session/pty_worker.rs:103`
  - Problem: khi `flush_update()` gặp `TrySendError::Full`, `dirty` được giữ để retry (đúng); nhưng nếu ngay sau đó reader nhận EOF (`Ok(0)`), code gửi `Exited` rồi break, không có bước “flush pending dirty update trước exit”.
  - Impact: trong edge case output burst + process exit nhanh, frame cuối có thể không render.
  - Recommendation: trước khi gửi `Exited`, thử một lần flush bắt buộc cho snapshot pending (hoặc coalesce latest snapshot + Exited theo thứ tự guaranteed).

- Parse config lỗi 1 field vẫn fallback toàn bộ default.
  - File: `src/config.rs:98`
  - Problem: `toml::from_str::<Config>(&raw).map(Config::normalized).unwrap_or_default()` làm mất các field hợp lệ khi có 1 field hỏng type/syntax.
  - Impact: degrade UX cấu hình; không phải security/blocker.
  - Recommendation: log parse error + parse theo table/field-level fallback.

### Low Priority
- Plan TODO docs chưa sync hoàn toàn với trạng thái code hiện tại.
  - File: `plans/260301-1521-chatminal-messenger-terminal/phase-07-integration-polish.md:140`
  - Note: TODO vẫn unchecked cho `blocking_send`, nhưng code đã dùng `blocking_send` ở EOF path.

### Edge Cases Found by Scout
- `SessionEvent` path: OS thread -> bounded tokio mpsc -> Iced subscription; behavior under full queue đã cải thiện.
- Boundary clamps xác nhận hoạt động cho scrollback/font/sidebar.
- ESC sequence path xác nhận `ESC M` qua parser integration test mới.

### Confirmation of Requested Findings
1. Hardcoded metrics: **Resolved**
- Evidence: `metrics_for_font()` + wiring runtime ở `AppState::boot`/resize.
  - `src/ui/theme.rs:10`
  - `src/app.rs:39`
  - `src/app.rs:272`

2. ESC M reverse index: **Resolved**
- Evidence code:
  - `src/session/pty_worker.rs:392`
- Evidence integration test:
  - `src/session/pty_worker.rs:412`

3. Clamp config bounds: **Resolved**
- Evidence:
  - `src/config.rs:34`
  - `src/config.rs:36`
  - `src/config.rs:42`
  - `src/config.rs:115`
  - `src/config.rs:130`

4. High finding queue-full: **Resolved at high-severity level**
- `Exited` now uses `blocking_send` (reliable delivery):
  - `src/session/pty_worker.rs:35`
  - `src/session/pty_worker.rs:46`
- `Update` now retries by keeping dirty state on full:
  - `src/session/pty_worker.rs:103`
  - `src/session/pty_worker.rs:112`
- Integration test added:
  - `src/session/pty_worker.rs:429`

### Positive Observations
- Full verification green:
  - `cargo test`: PASS (13/13)
  - `cargo clippy -- -D warnings`: PASS
  - `cargo build --release`: PASS
  - `cargo fmt -- --check`: PASS
- Security baseline ổn: shell path validation (`/etc/shells` + canonicalize + executable bit) + input size guard.

### Recommended Actions
1. Harden EOF path để flush snapshot pending trước `Exited` (medium fix).
2. Nâng config parsing sang partial fallback để không mất toàn bộ config khi lỗi cục bộ.
3. Sync checklist docs/plan với trạng thái thực thi thực tế.

### Metrics
- Type Coverage: N/A (Rust static typing; no explicit type coverage metric)
- Test Coverage: N/A (coverage tool not run)
- Linting Issues: 0

### Gate Verdict
- Overall score: **9.6 / 10**
- Auto-approve gate (`>=9.5` and `0 critical`): **PASS**
- Rationale: không có critical/high; còn medium non-blocking.

### Unresolved Questions
- Có muốn enforce strict guarantee “last update always delivered before Exited” cho mọi session exit path không?
