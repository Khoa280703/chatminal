# Phase 04 - Cross-Platform Transport

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminald/src/server.rs](/home/khoa2807/working-sources/chatminal/apps/chatminald/src/server.rs)

## Overview
- Priority: P1
- Status: Completed
- Mục tiêu: transport trait production cho Linux/macOS (UDS) + Windows Named Pipe qua abstraction chung.

## Key Insights
- TCP local path đã loại; cần giữ local-only IPC cứng.
- Phần khó nhất là Named Pipe async + quyền truy cập user scope.
- Đã tách transport module phía client theo platform (`ipc/transport/{unix,windows,unsupported}`).
- Đã tách transport module phía daemon (`transport/{mod,unix,windows}`), `server.rs` chạy qua abstraction listener/cleanup.
- Daemon đã bỏ hard-gate `unix-only`; loop request/response dùng chung cho UDS và Named Pipe.
- Daemon Windows bind đã claim first pipe instance để tránh multi-owner endpoint (split-brain).
- Daemon UDS path đã harden thêm:
  - stale-socket probe chỉ cleanup khi `ConnectionRefused` (không xóa bừa mọi lỗi connect)
  - `accept` loop xử lý lỗi recoverable (`Interrupted`, `ConnectionAborted`) theo nhánh retry
  - set permissions endpoint lỗi sẽ fail sớm thay vì nuốt lỗi
- Cross-target compile snapshot:
  - `chatminal-app` check target `x86_64-pc-windows-gnu` pass.
  - `chatminald` local Linux vẫn block khi cross-check target `x86_64-pc-windows-gnu` do thiếu `x86_64-w64-mingw32-gcc` cho `libsqlite3-sys` (đã có CI `windows-latest` để kiểm tra native toolchain).

## Requirements
- Functional:
1. Daemon/client dùng chung transport abstraction.
2. Linux/macOS chạy UDS ổn với stale socket cleanup.
3. Windows chạy Named Pipe ổn.
- Non-functional:
1. Không cần quyền firewall/network.
2. Recovery rõ ràng khi endpoint stale/busy.

## Architecture
- `TransportServer`/`TransportClient` traits.
- Backend implementations: `uds`, `named_pipe`.
- Unified frame codec + timeout/retry policy.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/server.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/ipc/transport/mod.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/ipc/client.rs`
- Create:
1. `apps/chatminald/src/transport/*`
2. `apps/chatminal-app/src/ipc/transport/*`
- Delete:
1. Platform-specific code lẫn trong logic parser sau khi tách

## Implementation Steps
1. Tách trait transport và codec boundary.
2. Implement UDS backend chuẩn hóa stale cleanup + chmod policy.
3. Implement Named Pipe backend + auth scope current user.
4. Thêm integration tests theo platform.
5. Cập nhật config endpoint resolver theo OS.

## Todo List
- [x] Hoàn tất trait transport cho daemon + client. (client `ReadWriteStream`; daemon `TransportBackend` + `TransportListener`)
- [x] Tách platform module độc lập. (đã tách phía client + daemon; behavior linux/macOS giữ nguyên)
- [x] Thêm test stale socket/pipe reconnect. (UDS đã có: stale socket cleanup, active socket reject, reconnect after disconnect, boot-from-stale socket; Named Pipe matrix để phase Windows)
- [x] Thêm docs vận hành endpoint theo OS. (đã cập nhật `docs/deployment-guide.md` cho Linux/macOS + note rollout Windows)
- [x] Cập nhật endpoint resolver theo OS:
  - Linux/macOS giữ UDS path theo data dir.
  - Windows mặc định Named Pipe endpoint `\\.\pipe\chatminald[-<username>]`.
- [x] Bổ sung CI lane chạy check/test trên `windows-latest` để bắt regression cross-platform sớm. (`.github/workflows/rewrite-quality-gates.yml`)
- [x] Implement backend Named Pipe thật cho app + daemon:
  - daemon: `apps/chatminald/src/transport/windows.rs`
  - app: `apps/chatminal-app/src/ipc/transport/windows.rs`
  - endpoint validation + connect/wait policy local-only

## Success Criteria
- Linux/macOS pass UDS integration tests.
- Windows pass Named Pipe smoke + reconnect (sau khi hoàn tất rollout Linux/macOS).
- Không còn đường chạy TCP trong production code.

## Risk Assessment
- Risk: bug platform-specific khó tái hiện trên máy dev hiện tại.
- Mitigation: CI matrix OS + smoke scripts riêng từng platform.

## Security Considerations
- UDS permissions user-only.
- Named Pipe local-only bằng `PIPE_REJECT_REMOTE_CLIENTS` + first-instance ownership.
- Endpoint path validation chống path traversal.

## Next Steps
- Theo dõi quality gate `windows-latest` để bắt regression theo platform.

## Unresolved questions
- None.
