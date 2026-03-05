## Code Review Summary

### Scope
- Files:
  - apps/chatminal-app/src/ipc/client.rs
  - apps/chatminal-app/src/ipc/client_tests.rs
  - scripts/migration/phase06-killswitch-verify.sh
- Focus: current HEAD only (final re-review after `read_next_incoming` lock-slice patch)

### Overall Assessment
- Patch lock-slice trong `read_next_incoming` đã xử lý vấn đề lock-held dài trên receiver; không còn thấy rủi ro Critical/High trong IPC client ở phạm vi này.
- IPC tests hiện pass ổn định trong run thường và loop stress.

### Critical Issues
- None.

### High Priority
- None.

### Medium Priority
- Default `ATTACH_TIMEOUT_SECONDS=2` trong kill-switch verify vẫn dễ tạo false-fail trên host/CI chậm.
  - Evidence: `scripts/migration/phase06-killswitch-verify.sh:8`, `scripts/migration/phase06-killswitch-verify.sh:49`, `scripts/migration/phase06-killswitch-verify.sh:61`.
  - Impact: gate có thể fail dù attach path thực tế vẫn hoạt động.
  - Recommendation: tăng default (ví dụ 5-8s) hoặc adaptive timeout theo môi trường.

### Low Priority
- Chưa có test riêng cho nhánh behavior mới của `read_next_incoming` lock-slice + disconnect wait path.
  - Evidence: `apps/chatminal-app/src/ipc/client.rs:147`, `apps/chatminal-app/src/ipc/client.rs:241`, `apps/chatminal-app/src/ipc/client_tests.rs:40`.
  - Impact: regression tinh vi ở fairness/timeout path có thể không bị bắt sớm.
  - Recommendation: thêm test deterministic cho contention giữa `request()` và `recv_event()` + disconnect race.

### Positive Observations
- `read_next_incoming` đã chuyển sang lock-slice (`READ_NEXT_INCOMING_LOCK_SLICE_MS=20`) để giảm head-of-line blocking.
- `wait_for_matching_response_until_deadline` thay thế grace cố định, tránh false disconnect của review trước.

### Validation Performed
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml ipc::client::client_tests`
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml concurrent_requests_receive_correct_response_variant` (loop 50 lần: pass)
- `bash -n scripts/migration/phase06-killswitch-verify.sh`

### Unresolved Questions
- Team có muốn giữ timeout 2s làm “aggressive fail-fast” cho kill-switch script, hay ưu tiên giảm flaky bằng default timeout cao hơn?
