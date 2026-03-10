# Phase 04 - Desktop Hot Path Cutover

## Context Links
- [plan.md](./plan.md)
- [main.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-app/src/main.rs)
- [ipc/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-app/src/ipc/mod.rs)
- [lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-protocol/src/lib.rs)

## Overview
- Priority: P1
- Status: completed
- Brief: cắt hoàn toàn desktop hot path khỏi protocol/IPC/daemon boundaries.

## Key Insights
- Sau phase 03, `apps/chatminal-app` không còn nằm trên đường chạy terminal desktop.
- `chatminal-protocol` và `apps/chatminal-app/src/ipc/*` chỉ còn giá trị cho compatibility CLI/automation/remote control.

## Requirements
- `make window` và GUI startup không phụ thuộc `chatminal-app`.
- Build graph desktop không còn cần `chatminal-protocol` cho runtime nội bộ.
- Compatibility path nếu còn phải được đánh dấu rõ là non-hot-path.

## Architecture
- `apps/chatminal-chatminal-desktop` tự bootstrap runtime và persistence.
- `apps/chatminal-app` thu hẹp scope thành:
  - launcher CLI compatibility
  - optional remote control client
  - benchmark/smoke helpers chưa di trú
- `chatminal-protocol` đổi vai trò từ shared-core sang compatibility boundary.

## Related Code Files
- Modify:
  - `apps/chatminal-app/src/main.rs`
  - `apps/chatminal-app/Cargo.toml`
  - `apps/chatminal-chatminal-desktop/Cargo.toml`
  - root `Cargo.toml`
- Delete later or shrink heavily:
  - `apps/chatminal-app/src/ipc/*`
  - `crates/chatminal-protocol` khỏi desktop default path
- Create:
  - có thể thêm `apps/chatminal-chatminal-desktop/src/cli_bridge.rs` nếu cần command reuse tại desktop app

## Implementation Steps
1. Đổi launcher path để `make window` gọi GUI app trực tiếp hoặc qua launcher mỏng không phụ thuộc protocol.
2. Bỏ dependency GUI -> app binary path/env `CHATMINAL_APP_BIN`.
3. Co lại `apps/chatminal-app` về compatibility commands không nằm trên desktop runtime path.
4. Điều chỉnh smoke/test scripts sang desktop single-runtime path.

## Todo List
- [ ] Cắt `CHATMINAL_APP_BIN` khỏi GUI runtime path
- [ ] Gỡ protocol khỏi desktop hot path
- [ ] Co lại `apps/chatminal-app`
- [ ] Cập nhật smoke/verify scripts

## Success Criteria
- GUI có thể khởi động và quản lý session mà không cần `chatminal-app` binary.
- `chatminal-protocol` không còn nằm trên hot path desktop.
- Scripts desktop verify path mới pass.

## Risk Assessment
- Risk: tool/script ecosystem hiện phụ thuộc CLI cũ.
- Mitigation: giữ compatibility shell đến phase 05 rồi mới quyết định remove hay preserve.

## Security Considerations
- Nếu giữ remote control/daemon compatibility thì boundary đó phải tách rõ khỏi desktop runtime nội bộ.

## Next Steps
- Sau phase này mới dọn mạnh docs/code compatibility được.
