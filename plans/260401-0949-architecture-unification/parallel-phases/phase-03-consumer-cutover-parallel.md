# Phase 03: Consumer Cutover Parallel

## Goal
Sau khi host-runtime contract đã freeze ở Phase 02, migrate các consumer lớn theo ownership tách biệt.

## Lanes
### Lane 03A: Lua Bridge Cutover
- Ownership:
  - `crates/chatminal-lua-bridge/src/lib.rs`
  - `crates/chatminal-lua-bridge/src/window.rs`
  - `crates/chatminal-lua-bridge/src/leaf.rs`
  - `crates/chatminal-lua-bridge/src/session.rs`
- Scope:
  - loại tối đa `Arc<Tab>` / raw `pane_id()` khỏi Lua read/capability boundary
  - chỉ giữ concrete tab/pane ở mutate/tree-traversal paths thật sự cần

### Lane 03B: Desktop Host Adapter Cutover
- Ownership:
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs`
- Scope:
  - migrate adapter desktop sang host-runtime helper/DTO mới
  - giảm `Arc<Tab>` / raw handle usage ở host adapter boundary

### Lane 03C: Config Dead Sweep
- Ownership:
  - `crates/chatminal-config/src/*`
- Scope:
  - hoàn tất phần Step 2 có thể làm độc lập: dọn dead fields/module không ảnh hưởng runtime contract
  - không đổi cấu trúc config lớn ở phase này

## Parallel Safety
- 03A, 03B, 03C không đụng cùng file.
- Cả 3 chỉ phụ thuộc contract đã chốt ở Phase 02.

## Gate
- `cargo check -p chatminal-lua-bridge -p chatminal-desktop -p chatminal-config`
- desktop tests xanh
