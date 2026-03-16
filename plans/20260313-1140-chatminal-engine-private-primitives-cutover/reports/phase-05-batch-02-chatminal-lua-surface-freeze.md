## Phase 05 Batch 02

Status: completed
Date: 2026-03-13

### Scope shipped
- Xóa public host-id surface khỏi Lua bridge/session/window/leaf userdata.
- Đổi public vocabulary từ `host_tab` / `host_leaf` sang `terminal` / `terminal_instance_id`.
- Đổi list/query API:
  - `all_host_tabs` -> `list_sessions`
  - `all_host_leaves` -> `all_terminals`
- Giữ alias tối thiểu `render_scope_id` chỉ như compatibility field, không còn là source of truth của product model.

### Files changed
- `crates/chatminal-lua-bridge/src/lib.rs`
- `crates/chatminal-lua-bridge/src/session.rs`
- `crates/chatminal-lua-bridge/src/window.rs`
- `crates/chatminal-lua-bridge/src/leaf.rs`

### Gates
- `cargo check -p chatminal-lua-bridge`: pass
- `cargo check -p chatminal-desktop`: pass
- `rg -n "get_host_tab|get_host_leaf|host_tab|host_leaf" crates/chatminal-lua-bridge apps/chatminal-desktop/src/main.rs apps/chatminal-desktop/src/scripting`: zero matches trong source active

### Outcome
- Lua/config surface giờ kể cùng một câu chuyện với desktop product model.
- Host ids không còn là public API contract.
