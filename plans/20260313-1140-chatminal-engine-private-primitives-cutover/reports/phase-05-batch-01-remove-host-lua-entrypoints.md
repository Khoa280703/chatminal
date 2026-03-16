## Phase 05 Batch 01

Status: in_progress

### Scope shipped
- Xóa public Lua API `session.get_host_tab()`.
- Xóa public Lua API `session.get_host_leaf()`.
- Xóa helper chết `resolve_host_leaf_by_id()` sau khi bỏ API cũ.

### Files changed
- `crates/chatminal-lua-bridge/src/lib.rs`

### Gates
- `cargo check -p chatminal-lua-bridge`: pass
- `cargo check -p chatminal-desktop`: pass
- `rg -n "get_host_tab|get_host_leaf" crates/chatminal-lua-bridge/src -g '!third_party/**'`: zero matches

### Remaining in Phase 05
- Lua userdata vẫn còn expose vocabulary/fields kiểu `host_tab_id`, `host_leaf_id`, `active_host_leaf_id`.
- `LeafRef` / `SessionRef` public method names vẫn còn leak host primitive semantics.
- Cần quyết định batch kế tiếp: xóa thẳng các host id public fields hay đổi sang object refs/product-named accessors.
