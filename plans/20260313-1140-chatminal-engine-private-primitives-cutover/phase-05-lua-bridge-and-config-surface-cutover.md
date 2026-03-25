# Phase 05 - Lua Bridge And Config Surface Cutover

## Context Links
- `appendices/forbidden-symbols-contract.md`
- `appendices/commit-and-cutover-strategy.md`
- `crates/chatminal-lua-bridge/src/lib.rs`
- `crates/chatminal-lua-bridge/src/spawn_target.rs`
- `crates/chatminal-lua-bridge/src/window.rs`
- `crates/chatminal-lua-bridge/src/session.rs`
- `crates/chatminal-lua-bridge/src/leaf.rs`
- `apps/chatminal-desktop/src/main.rs`
- `apps/chatminal-desktop/src/scripting/*`

## Overview
- Priority: P1
- Status: completed
- Brief: đồng bộ config/script layer với vocabulary của Chatminal, giảm hoặc loại bỏ public Lua APIs mang semantics `host_tab/host_leaf`.

## Key Insights
- Nếu Lua/config vẫn expose host model cũ, boundary app vẫn chưa sạch dù desktop code đã refactor.
- Đây là compatibility surface, nên cần migration path rõ thay vì xóa đột ngột tất cả.

## Requirements
- Xác định API Lua nào còn cần thật:
  - window lifecycle events
  - launch/spawn hooks
  - query session/workspace hiện tại
  - hotkey/config customization
- Thiết kế Chatminal-facing APIs mới, ví dụ:
  - `session.get_current_session()`
  - `session.get_session_view()`
  - `window.get_active_session_window()`
  - `session.list_sessions()`
- Đánh dấu deprecate hoặc xóa:
  - `get_host_tab`
  - `get_host_leaf`
  - các API leak host ids nếu không cần nữa

## Compatibility Policy
- `keep`:
  - GUI startup/attached events
  - window/query hooks phục vụ config thực sự
  - session/workspace queries theo model mới
- `deprecate for one migration window`:
  - `get_host_tab`
  - `get_host_leaf`
  - API nào trả host id nhưng còn callsite thực tế
- `delete immediately if no real consumer`:
  - helper Lua chỉ là thin wrapper của host model cũ mà không còn callsite/config path
- Rule:
  - deprecate phải có warning runtime + doc note + tracked removal point

## API Matrix
- `session.get_current_session()` -> keep/add
- `session.list_sessions()` -> keep/add
- `session.get_session_view()` -> add
- `session.get_session_group()` -> add if needed by future layout/group features
- `window.get_active_session_window()` -> add if needed
- `session.get_host_tab()` -> deprecate/delete
- `session.get_host_leaf()` -> deprecate/delete
- `session.get_target()` -> keep nếu còn đúng config scope

## Architecture
- `chatminal-lua-bridge` trở thành adapter từ Lua/config sang Chatminal app boundary.
- Nếu cần compatibility shim, shim phải gọi ngược về Chatminal boundary thay vì chạm host primitives trực tiếp ở nhiều nơi.

## Related Code Files
- Refactor: `crates/chatminal-lua-bridge/src/lib.rs`
- Refactor: `crates/chatminal-lua-bridge/src/session.rs`
- Refactor: `crates/chatminal-lua-bridge/src/window.rs`
- Refactor: `crates/chatminal-lua-bridge/src/leaf.rs`
- Refactor: `apps/chatminal-desktop/src/main.rs`
- Refactor: `apps/chatminal-desktop/src/scripting/guiwin.rs`

## Implementation Steps
1. Inventory Lua APIs hiện có và phân loại `keep / deprecate / delete`.
2. Thêm Chatminal-facing Lua APIs mới nếu thiếu.
3. Chuyển internal implementation sang runtime facade/boundary types.
4. Đánh dấu deprecate cho API cũ với log/docs rõ ràng, hoặc xóa nếu chắc chắn không dùng.
5. Verify startup/config/hotkey path vẫn chạy.

## Phase Gates
- `rg -n "get_host_tab|get_host_leaf|host_tab|host_leaf" crates/chatminal-lua-bridge apps/chatminal-desktop/src/main.rs apps/chatminal-desktop/src/scripting`
  - expected after phase: zero public API residual ngoài shim deprecate đã annotate rõ
- `rg -n "tab_id\\(|pane_id\\(" crates/chatminal-lua-bridge`
  - expected: no product-facing Lua API depends on raw host ids unless inside deprecated shim internals
- `rg -n "SessionRef\\(|LeafRef\\(" crates/chatminal-lua-bridge`
  - expected: no new product-facing reliance on old host wrappers after cutover

## Todo List
- [x] Lua API inventory hoàn tất
- [x] API mới theo vocabulary Chatminal được thêm vào nơi cần
- [x] Host id APIs cũ bị deprecate hoặc xóa
- [x] Compatibility policy được áp dụng rõ từng API
- [x] Main/config path không còn chạm host model trực tiếp ngoài adapter hợp lệ
- [x] Smoke startup/config pass

## Success Criteria
- User-config/script layer không còn ép app layer suy nghĩ bằng `host_tab/host_leaf`.
- Lua bridge trở thành compatibility adapter gọn, không phải nguồn leak semantics cũ.

## Risk Assessment
- Risk: phá config power-user scripts hiện có.
- Mitigation: deprecate có kiểm soát, giữ shim ngắn hạn nếu cần, log rõ tại runtime.

## Security Considerations
- Lua bridge không được mở thêm quyền runtime hay access path mới ngoài hiện trạng.

## Next Steps
- Phase 06 xóa nốt dead paths sau khi desktop/UI/Lua boundary đều đã sạch.
- Batch logs:
  - `reports/phase-05-batch-01-remove-host-lua-entrypoints.md`
  - `reports/phase-05-batch-02-chatminal-lua-surface-freeze.md`
