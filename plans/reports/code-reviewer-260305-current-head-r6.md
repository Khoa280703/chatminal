## Code Review Summary

### Scope
- Files:
  - apps/chatminal-app/src/ipc/client.rs
  - apps/chatminal-app/src/ipc/client_tests.rs
  - plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/phase-07-windows-input-parity-follow-up.md
  - plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/windows-input-parity-report.md
- Focus: current HEAD only (fresh re-review after latest fix)

### Overall Assessment
- Fix bỏ fixed `25ms` grace đã được áp đúng trong `client.rs` (`wait_for_matching_response_until_deadline`) và loại bỏ High trước đó.
- Docs Phase 07 + report parity hiện đồng bộ nhau về phạm vi CI Windows (compile + tests tối thiểu).

### Critical Issues
- None.

### High Priority
- None.

### Medium Priority
- `frames_rx` được khóa bằng `Mutex` và giữ trong suốt `recv_timeout`, tạo head-of-line blocking giữa `request()` và `recv_event()` khi chạy đồng thời.
  - Evidence: `apps/chatminal-app/src/ipc/client.rs:145`, `apps/chatminal-app/src/ipc/client.rs:150`, `apps/chatminal-app/src/ipc/client.rs:75`.
  - Impact: khi có contention, thread kia phải chờ tối đa chunk timeout để đọc frame, có thể làm tăng latency hoặc chạm deadline.
  - Recommendation: tách lane request/event hoặc dùng dispatcher đơn + fanout queue để tránh blocking theo mutex trên receiver.

### Low Priority
- `client_tests.rs` chưa có test chuyên biệt cho nhánh mới `wait_for_matching_response_until_deadline` (disconnect + backlog race), nên regression ở behavior mới này chưa được khóa trực tiếp.
  - Evidence: `apps/chatminal-app/src/ipc/client.rs:228`, `apps/chatminal-app/src/ipc/client_tests.rs:40`.
  - Recommendation: thêm 1 test deterministic mô phỏng disconnect xảy ra ngay sau khi response được phát nhưng trước khi thread request đọc được frame.

### Positive Observations
- Concurrency test hiện tại vẫn pass ổn định sau fix (`concurrent_requests_receive_correct_response_variant`).
- Hai tài liệu Phase 07 đã thống nhất wording requirement/evidence, không còn mismatch “smoke input” vs “compile/tests”.

### Validation Performed
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml ipc::client::client_tests`
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml concurrent_requests_receive_correct_response_variant`

### Unresolved Questions
- Team có muốn chấp nhận HOL blocking hiện tại như tradeoff đơn giản hóa, hay sẽ ưu tiên refactor dispatcher IPC trong phase kế tiếp?
