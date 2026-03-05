## Code Review Summary

### Scope
- Files: `apps/chatminald/src/transport/windows.rs`, `apps/chatminal-app/src/ipc/client.rs`, `apps/chatminal-app/src/ipc/transport/windows.rs`
- LOC: 644
- Focus: final final sign-off after split-brain single-owner endpoint fix
- Scout findings: accept/connect handoff và single-owner guard đã được đóng; còn lại chủ yếu là timeout contract + writer stall behavior

### Overall Assessment
- Critical: **0**
- High: **0**
- Medium: **2**
- Low: **2**
- Sign-off status: **Pass for Critical/High gate**

### Critical Issues
- None.

### High Priority
- None.

### Medium Priority
1. `request(timeout)` vẫn là best-effort timeout, không bảo đảm request đã bị hủy khi caller timeout.
- Evidence: `apps/chatminal-app/src/ipc/client.rs:197`, `apps/chatminal-app/src/ipc/client.rs:200`, `apps/chatminal-app/src/ipc/client.rs:238`, `apps/chatminal-app/src/ipc/client.rs:246`
- Impact: caller có thể nhận timeout nhưng request vẫn được daemon xử lý muộn.

2. Writer thread có thể kẹt ở blocking write/flush, gây queue pressure và timeout dây chuyền.
- Evidence: `apps/chatminal-app/src/ipc/client.rs:184`, `apps/chatminal-app/src/ipc/client.rs:187`, `apps/chatminal-app/src/ipc/client.rs:246`, `apps/chatminal-app/src/ipc/client.rs:249`
- Impact: khi IPC peer stall, các request sau dễ timeout dù logic request/response vẫn đúng.

### Low Priority
1. Timeout connect named pipe ở client Windows đang hard-code 3s.
- Evidence: `apps/chatminal-app/src/ipc/transport/windows.rs:19`, `apps/chatminal-app/src/ipc/transport/windows.rs:53`
- Impact: môi trường cold-start chậm có thể cần retry outer-layer.

2. Daemon listener degrade về `Ok(None)` khi create/pre-create pipe fail (log warn + retry), có thể che dấu lỗi hạ tầng kéo dài.
- Evidence: `apps/chatminald/src/transport/windows.rs:77`, `apps/chatminald/src/transport/windows.rs:81`, `apps/chatminald/src/transport/windows.rs:112`, `apps/chatminald/src/transport/windows.rs:115`
- Impact: không crash là tốt, nhưng cần quan sát log/metrics để phát hiện failure lặp.

### Edge Cases Found by Scout
- Multi-daemon split-brain qua cùng endpoint đã được chặn tại bind-first-instance path (`FILE_FLAG_FIRST_PIPE_INSTANCE`) và map `ERROR_ACCESS_DENIED` thành "endpoint already in use".
- Connect/disconnect race (`ERROR_NO_DATA`) đã có recovery path `DisconnectNamedPipe` + recreate.
- Handoff gap đã giảm: pre-create next pending instance trước khi handoff stream connected.

### Positive Observations
- Single-owner guard đã có tại `apps/chatminald/src/transport/windows.rs:160`, `apps/chatminald/src/transport/windows.rs:180`, `apps/chatminald/src/transport/windows.rs:181`.
- Endpoint continuity tốt hơn nhờ pre-create next pending trước khi trả stream (`apps/chatminald/src/transport/windows.rs:111`, `apps/chatminald/src/transport/windows.rs:112`).
- Client Windows transport đã fail-fast nếu `SetNamedPipeHandleState` thất bại (`apps/chatminal-app/src/ipc/transport/windows.rs:110`, `apps/chatminal-app/src/ipc/transport/windows.rs:113`).

### Recommended Actions
1. Document rõ timeout contract của `ChatminalClient::request` (caller-unblock vs strict cancellation).
2. Bổ sung strategy unstick writer (bounded write/reconnect/reset policy) để giảm timeout cascade.
3. Cân nhắc make connect timeout configurable thay vì hard-code 3s.

### Metrics
- Type Coverage: N/A (Rust)
- Test Coverage: not measured in this pass
- Linting Issues: not measured in this pass
- Verification commands run:
  - `cargo check --workspace` -> PASS
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml` -> PASS (42 passed)
  - `cargo test --manifest-path apps/chatminald/Cargo.toml` -> PASS (36 passed)
  - `cargo check --manifest-path apps/chatminal-app/Cargo.toml --target x86_64-pc-windows-gnu` -> PASS
  - `cargo check --manifest-path apps/chatminald/Cargo.toml --target x86_64-pc-windows-gnu` -> FAIL (missing `x86_64-w64-mingw32-gcc`)

### Unresolved Questions
1. Timeout của `request()` cần semantics nào ở product level: "caller không block" hay "request chắc chắn không execute"?
2. Khi writer stall lâu trên Windows named pipe, có policy reconnect/reset bắt buộc không?
