## Code Review Summary

### Scope
- Files:
  - apps/chatminal-app/src/window/native_window_wezterm_actions.rs
  - scripts/migration/phase06-killswitch-verify.sh
  - apps/chatminal-app/src/ipc/client.rs
  - apps/chatminal-app/src/ipc/client_tests.rs
  - docs/terminal-fidelity-matrix.md
  - plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/windows-input-parity-report.md
  - plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/phase-07-windows-input-parity-follow-up.md
- LOC reviewed: 1249
- Focus: current HEAD only (fresh rereview)
- Scout findings: async race/timeout risks around IPC disconnect window and UI-thread blocking IPC path

### Overall Assessment
- HEAD hiện tại đã ổn định hơn (test IPC concurrency pass, app tests pass), không thấy lỗi Critical.
- Vẫn còn 1 High risk về độ bền logic disconnect grace trong IPC client; ngoài ra có vài Medium về timeout/UX blocking và tính nhất quán tài liệu.

### Critical Issues
- None.

### High Priority
- Fixed grace window 25ms vẫn có thể trả lỗi disconnect giả trong cạnh tranh thread.
  - Evidence: `apps/chatminal-app/src/ipc/client.rs:25`, `apps/chatminal-app/src/ipc/client.rs:84`, `apps/chatminal-app/src/ipc/client.rs:230`.
  - Impact: request có thể fail sai khi response đã/đang được thread khác đưa vào backlog nhưng đến sau cửa sổ 25ms.
  - Recommendation: thay grace cố định bằng chờ đến `deadline` (hoặc ít nhất dynamic grace theo `remaining timeout`).

### Medium Priority
- Input path gọi IPC đồng bộ ngay trong UI event loop, timeout đến 1s/event.
  - Evidence: `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:21`, `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:375`.
  - Impact: burst input (IME/paste/repeat key) có thể gây khựng UI và cảm giác drop input.
  - Recommendation: queue async/non-blocking send hoặc coalesce payload trong frame.

- Khi session disconnected, mỗi keypress có thể kích hoạt lại flow activate/snapshot đồng bộ.
  - Evidence: `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:361`.
  - Impact: trong trạng thái reconnect chập chờn, UI có thể liên tục block và mutate state.
  - Recommendation: thêm backoff/circuit-breaker cho auto-reactivate, tránh gọi lại liên tiếp mỗi event.

- Default timeout 2s cho kill-switch verify attach dễ false fail trên máy/CI chậm.
  - Evidence: `scripts/migration/phase06-killswitch-verify.sh:8`, `scripts/migration/phase06-killswitch-verify.sh:49`, `scripts/migration/phase06-killswitch-verify.sh:61`.
  - Impact: flaky gate dù attach path thực tế vẫn đúng.
  - Recommendation: tăng default (ví dụ 5-8s) hoặc adaptive timeout theo trạng thái build host.

- Tài liệu Phase 07 ghi CI cần smoke input tối thiểu nhưng evidence hiện mô tả compile/tests.
  - Evidence: `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/phase-07-windows-input-parity-follow-up.md:28`, `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/windows-input-parity-report.md:14`.
  - Impact: trạng thái “Completed” có thể đang overstate so với yêu cầu non-functional đã ghi.
  - Recommendation: hoặc bổ sung smoke step Windows, hoặc chỉnh requirement/evidence cho đồng nhất.

### Low Priority
- Fallback nhánh không có `timeout/gtimeout` kill PID `script` theo deadline, vẫn có khả năng để lại descendant process ngắn hạn.
  - Evidence: `scripts/migration/phase06-killswitch-verify.sh:65`, `scripts/migration/phase06-killswitch-verify.sh:26`.
  - Recommendation: kill theo process group để chắc chắn cleanup.

- Test hiện cover reorder response, chưa cover race disconnect-grace/backlog saturation.
  - Evidence: `apps/chatminal-app/src/ipc/client_tests.rs:40`.
  - Recommendation: thêm tests stress có inject scheduling delay quanh nhánh disconnect.

### Edge Cases Found by Scout
- Response đến sát thời điểm stream disconnect + cạnh tranh giữa request threads.
- UI freeze khi daemon chậm nhưng input event tiếp tục đổ vào.
- Kill-switch script chạy trên host lạnh/chậm gây false negative do timeout default thấp.

### Positive Observations
- IPC request path đã thêm deadline-aware write/flush loop, hạn chế block vô hạn.
- Test `concurrent_requests_receive_correct_response_variant` đang pass ổn định (đã chạy lặp nhiều lần tại HEAD).
- Docs matrix Windows đã ghi rõ trạng thái follow-up và manual gate context.

### Recommended Actions
1. Sửa disconnect grace theo remaining deadline thay vì fixed 25ms.
2. Tách input send khỏi UI hot path (queue/background worker hoặc bounded batching).
3. Tăng/adapt attach timeout trong `phase06-killswitch-verify.sh`.
4. Đồng bộ lại requirement vs evidence cho Phase 07 Windows smoke.
5. Bổ sung test cho race disconnect/backlog edge-case.

### Metrics
- Type Coverage: N/A (Rust project, không có metric type coverage riêng trong run này)
- Test Coverage: N/A (không có report coverage trong run này)
- Linting Issues: N/A (không chạy lint trong run này)
- Validation run in this review:
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml concurrent_requests_receive_correct_response_variant` (loop stress 80 runs: pass)
  - `bash -n scripts/migration/phase06-killswitch-verify.sh`

### Unresolved Questions
- Team có chấp nhận semantics “compile+unit tests = smoke tối thiểu” cho Windows Phase 07 không, hay bắt buộc smoke input step riêng trong CI?
