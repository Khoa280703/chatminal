## Code Review Summary

### Scope
- Files:
  - `apps/chatminal-app/src/ipc/client.rs`
  - `apps/chatminal-app/src/ipc/transport/windows.rs`
  - `apps/chatminal-app/src/config.rs`
  - `apps/chatminald/src/config.rs`
  - `scripts/bench/phase02-rtt-memory-gate.sh`
- LOC scanned: 1040
- Focus: final sign-off review sau batch fix high trước đó (no code edits)
- Scout findings: benchmark scout + late rust scout (edge-case semantics) + manual dependency review

### Overall Assessment
- Critical: none
- High: 3
- Medium: 4
- Low: 3

### Critical Issues
- None.

### High Priority
1. Request timeout không đảm bảo request chưa được ghi/áp dụng; caller retry có thể tạo duplicate side effects cho request mutating.
- Evidence:
  - `apps/chatminal-app/src/ipc/client.rs:220`
  - `apps/chatminal-app/src/ipc/client.rs:224`
  - `apps/chatminal-app/src/ipc/client.rs:289`
  - `apps/chatminal-app/src/ipc/client.rs:334`
- Why high: vi phạm at-most-once expectation ở command path mutating (create/input/resize) khi timeout xảy ra gần deadline.

2. Windows writer worker dừng hẳn sau lỗi write/flush đầu tiên; toàn bộ request sau đó trên cùng client degrade thành `daemon writer disconnected`.
- Evidence:
  - `apps/chatminal-app/src/ipc/client.rs:299`
  - `apps/chatminal-app/src/ipc/client.rs:301`
  - `apps/chatminal-app/src/ipc/client.rs:214`
  - `apps/chatminal-app/src/ipc/client.rs:227`
- Why high: single transient write failure có thể làm path IPC của UI/client instance chết cứng.

3. Bench RSS hard gate có false-pass risk vì chỉ đo parent PID (daemon + app), không tính child process tree (shell/pty child) và sampling theo polling interval.
- Evidence:
  - `scripts/bench/phase02-rtt-memory-gate.sh:112`
  - `scripts/bench/phase02-rtt-memory-gate.sh:114`
  - `scripts/bench/phase02-rtt-memory-gate.sh:111`
  - `scripts/bench/phase02-rtt-memory-gate.sh:119`
- Why high: quality gate memory có thể pass dù memory thực tế vượt ngưỡng ở child processes.

### Medium Priority
1. Bench hard gate dùng threshold local (`P95_HARD_FAIL_MS`) thay vì enforce `pass_fail_gate`; có drift risk + rounding gap nhỏ.
- Evidence:
  - `scripts/bench/phase02-rtt-memory-gate.sh:24`
  - `scripts/bench/phase02-rtt-memory-gate.sh:175`
  - `scripts/bench/phase02-rtt-memory-gate.sh:192`

2. Bench monitor loop thiếu wall-clock timeout; benchmark hang có thể làm CI/job treo vô hạn.
- Evidence:
  - `scripts/bench/phase02-rtt-memory-gate.sh:111`

3. Windows endpoint fallback vẫn có collision edge case khi sanitize identity rỗng/trùng (fallback host/default-user).
- Evidence:
  - `apps/chatminal-app/src/config.rs:57`
  - `apps/chatminal-app/src/config.rs:70`
  - `apps/chatminald/src/config.rs:91`
  - `apps/chatminald/src/config.rs:104`

4. `CHATMINAL_DATA_DIR` nhận relative path trực tiếp; app/daemon khác CWD có thể resolve endpoint path khác nhau.
- Evidence:
  - `apps/chatminal-app/src/config.rs:74`
  - `apps/chatminald/src/config.rs:108`

### Low Priority
1. Temp path dưới `/tmp` dùng `$$` predictable, collision/symlink risk trong shared env.
- Evidence:
  - `scripts/bench/phase02-rtt-memory-gate.sh:6`
  - `scripts/bench/phase02-rtt-memory-gate.sh:8`

2. Cleanup kill theo PID raw; PID reuse edge case có thể kill nhầm process khác (rare).
- Evidence:
  - `scripts/bench/phase02-rtt-memory-gate.sh:31`
  - `scripts/bench/phase02-rtt-memory-gate.sh:37`

3. Script phụ thuộc `seq`; portability giảm ở minimal env.
- Evidence:
  - `scripts/bench/phase02-rtt-memory-gate.sh:88`

### Edge Cases Found by Scout
- IPC timeout vs delivery semantics: timeout ở client không đồng nghĩa daemon chưa nhận request.
- Writer worker lifecycle: one-error-break gây cascading availability failure.
- Benchmark gate parsing phụ thuộc format `RTT_BENCH key=value ...`.

### Positive Observations
- Non-Windows path đã set per-request write timeout trước write/flush (`apps/chatminal-app/src/ipc/client.rs:239`-`257`).
- Windows transport có endpoint validation + retry connect theo deadline (`apps/chatminal-app/src/ipc/transport/windows.rs:50`-`99`).
- Summary parser đã fail-closed khi thiếu/invalid fields (`scripts/bench/phase02-rtt-memory-gate.sh:146`-`165`).

### Recommended Actions
1. Tách timeout semantics khỏi delivery semantics: thêm request idempotency/ack contract rõ hoặc dedupe server-side cho mutating requests.
2. Đổi writer worker strategy: không break cứng với mọi error; hoặc reconnect/reset stream tự động có backoff.
3. Đổi memory gate sang process-tree RSS (hoặc parse metric trực tiếp từ daemon/app telemetry) + thêm wall-time timeout.
4. Cho benchmark script gate trực tiếp theo `pass_fail_gate` hoặc JSON report thay vì duplicate constant.
5. Harden Windows endpoint suffix bằng stable per-user identifier (SID/hash), tránh fallback generic.

### Metrics
- Build check:
  - `cargo check --manifest-path apps/chatminal-app/Cargo.toml --quiet` (pass)
  - `cargo check --manifest-path apps/chatminald/Cargo.toml --quiet` (pass)
  - `cargo check --manifest-path apps/chatminal-app/Cargo.toml --target x86_64-pc-windows-gnu --quiet` (pass, warning dead_code)
  - `cargo check --manifest-path apps/chatminald/Cargo.toml --target x86_64-pc-windows-gnu --quiet` (blocked: missing `x86_64-w64-mingw32-gcc`)
- Tests:
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml --quiet` (42 passed)
  - `cargo test --manifest-path apps/chatminald/Cargo.toml --quiet` (36 passed)
- Linting issues: không chạy linter; 1 warning dead_code ở app Windows target check

### Sign-off Conclusion
- Critical còn lại: **không**.
- High còn lại: **có (3)**.
- Batch này **chưa clean final sign-off**; cần xử lý High trước khi chốt release gate.

### Unresolved Questions
1. Team có chấp nhận retry-induced duplicate risk cho mutating IPC requests ở release hiện tại không?
2. Team muốn benchmark gate đo memory parent-only hay full process tree mới đúng policy?
