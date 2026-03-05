## Code Review Summary

### Scope
- Files: `apps/chatminald/src/transport/windows.rs`, `apps/chatminal-app/src/ipc/client.rs`, `apps/chatminal-app/src/ipc/transport/windows.rs`
- LOC: 634
- Focus: final sign-off after full gates pass
- Scout findings: verified, filtered to current-code-confirmed risks only

### Overall Assessment
- Critical: **0**
- High: **1**
- Medium: **2**
- Low: **1**
- Sign-off status: **Not ready for full sign-off** (still 1 High).

### Critical Issues
- None.

### High Priority
1. Windows daemon endpoint chưa có single-owner guard, có thể gây split-brain khi chạy nhiều daemon cùng pipe name.
- Evidence: `CreateNamedPipeW(..., PIPE_UNLIMITED_INSTANCES, ...)` tại `apps/chatminald/src/transport/windows.rs:160`-`164`; `bind()` chỉ validate prefix, không probe endpoint đang active tại `apps/chatminald/src/transport/windows.rs:135`-`140`.
- Impact: client có thể attach ngẫu nhiên vào daemon instance khác nhau, vi phạm nguyên tắc daemon single source of truth.
- Suggested fix: enforce first-instance ownership (vd `FILE_FLAG_FIRST_PIPE_INSTANCE`) hoặc cơ chế lock/probe tương đương Unix path semantics.

### Medium Priority
1. `request(timeout)` không đảm bảo “timeout => request không được thực thi”.
- Evidence: timeout chờ ACK ở `apps/chatminal-app/src/ipc/client.rs:197`-`203`, nhưng writer chỉ check deadline trước write tại `apps/chatminal-app/src/ipc/client.rs:238` và có thể vẫn write/flush xong tại `apps/chatminal-app/src/ipc/client.rs:246`-`249`.

2. Writer có thể block dài ở `write_all/flush`, kéo theo queue pressure và timeout dây chuyền.
- Evidence: blocking write path tại `apps/chatminal-app/src/ipc/client.rs:246`-`249`; transport trait không có timeout contract tại `apps/chatminal-app/src/ipc/transport/mod.rs:10`-`11`.

### Low Priority
1. Windows client connect timeout hard-coded 3s có thể fail khi daemon cold-start chậm.
- Evidence: `PIPE_CONNECT_TIMEOUT_MS = 3_000` tại `apps/chatminal-app/src/ipc/transport/windows.rs:19`.

### Edge Cases Found by Scout
- Multi-daemon same pipe name on Windows (split-brain routing).
- Timed-out caller nhưng request có thể execute muộn.
- Writer stall gây saturation ở queue 2048 và tăng timeout rate.

### Positive Observations
- `ERROR_NO_DATA` trên daemon transport đã có recovery path (`DisconnectNamedPipe` + recreate) tại `apps/chatminald/src/transport/windows.rs:191`-`194`.
- Client transport đã xử lý lỗi `SetNamedPipeHandleState` rõ ràng tại `apps/chatminal-app/src/ipc/transport/windows.rs:110`-`116`.
- Full tests hiện tại pass:
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml` (42 passed)
  - `cargo test --manifest-path apps/chatminald/Cargo.toml` (36 passed)

### Recommended Actions
1. Chặn multi-owner trên Windows named pipe endpoint (ưu tiên cao nhất).
2. Chốt rõ timeout contract của `ChatminalClient::request` và cập nhật call-sites theo contract đó.
3. Bổ sung strategy unstick writer (bounded write / reconnect policy) để giảm timeout cascade.

### Metrics
- Type Coverage: N/A (Rust)
- Test Coverage: not measured in this pass
- Linting Issues: not measured in this pass

### Unresolved Questions
1. Timeout của `request()` được định nghĩa là “caller unblocked” hay “guaranteed not executed”?
2. Có yêu cầu behavior parity với Unix về single-instance daemon ownership trên endpoint không?
