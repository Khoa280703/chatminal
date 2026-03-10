# Phase 01 - Runtime Extraction

## Context Links
- [plan.md](./plan.md)
- [state.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminald/src/state.rs)
- [session.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminald/src/session.rs)
- [request_handler.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminald/src/state/request_handler.rs)
- [server.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminald/src/server.rs)

## Overview
- Priority: P0
- Status: completed
- Brief: bóc runtime lõi ra khỏi daemon process để GUI có thể nhúng trực tiếp.

## Key Insights
- Runtime thật đang nằm trong `chatminald/src/state.rs`, không nằm trong GUI.
- `server.rs` chỉ là transport wrapper; đây là boundary phải tách.
- `SessionRuntime` hiện dùng `String` cho output/input API; cần chuẩn bị đường chuyển sang `Vec<u8>` ở hot path.

## Requirements
- Public API mới không phụ thuộc transport server.
- Runtime vẫn giữ `chatminal-store` làm persistence layer.
- `chatminald` phải compile lại bằng cách bọc crate mới thay vì copy logic.

## Architecture
- Tạo crate mới: `crates/chatminal-runtime`
- Module đề xuất:
  - `src/lib.rs`: public exports
  - `src/config.rs`: runtime config dùng chung desktop/daemon host
  - `src/runtime.rs`: `ChatminalRuntime`, lifecycle boot/shutdown
  - `src/session.rs`: PTY runtime + input/output event production
  - `src/metrics.rs`: runtime metrics
  - `src/event_bus.rs`: subscription/broadcast nội bộ
  - `src/workspace.rs`: profile/session state API
  - `src/explorer.rs`: session explorer API
  - `src/persistence.rs`: adapter với `chatminal-store`
  - `src/compat_protocol.rs`: mapper request/response cho daemon wrapper tạm thời

## Related Code Files
- Modify:
  - `Cargo.toml`
  - `apps/chatminald/Cargo.toml`
  - `apps/chatminald/src/main.rs`
- Create:
  - `crates/chatminal-runtime/Cargo.toml`
  - `crates/chatminal-runtime/src/lib.rs`
  - `crates/chatminal-runtime/src/config.rs`
  - `crates/chatminal-runtime/src/runtime.rs`
  - `crates/chatminal-runtime/src/session.rs`
  - `crates/chatminal-runtime/src/metrics.rs`
  - `crates/chatminal-runtime/src/event_bus.rs`
  - `crates/chatminal-runtime/src/workspace.rs`
  - `crates/chatminal-runtime/src/explorer.rs`
  - `crates/chatminal-runtime/src/persistence.rs`
  - `crates/chatminal-runtime/src/compat_protocol.rs`
- Delete later:
  - chưa xoá ngay trong phase này; chỉ chuyển ownership logic.

## Implementation Steps
1. Tạo crate `chatminal-runtime` và move logic từ `chatminald` sang crate mới theo module boundary ở trên.
2. Tách `DaemonState` thành `ChatminalRuntime` + wrapper compatibility cho `Request/Response`.
3. Giữ toàn bộ tests hiện tại của daemon bằng cách chạy lại qua wrapper mới.
4. Refactor `apps/chatminald` thành host mỏng: parse env, mở store, khởi tạo runtime, rồi attach transport server nếu cần.

## Todo List
- [ ] Tạo crate `chatminal-runtime`
- [ ] Bóc `state/session/metrics/explorer` sang crate mới
- [ ] Giữ green tests cho `apps/chatminald`
- [ ] Chuẩn bị event subscription API cho GUI phase sau

## Success Criteria
- `apps/chatminald` không còn chứa business logic chính.
- `chatminal-runtime` có API in-process để load workspace, create/switch session/profile, subscribe events.
- Test daemon pass qua wrapper mới.

## Risk Assessment
- Risk: vòng phụ thuộc giữa runtime/store/protocol.
- Mitigation: protocol mapper để riêng trong `compat_protocol.rs`; runtime API native không phụ thuộc `chatminal-protocol`.

## Security Considerations
- Không đổi data format SQLite ở phase này.
- Giữ boundary validate input size và path explorer như hiện tại.

## Next Steps
- Sau phase này mới có thể nối GUI trực tiếp mà không cần IPC.
