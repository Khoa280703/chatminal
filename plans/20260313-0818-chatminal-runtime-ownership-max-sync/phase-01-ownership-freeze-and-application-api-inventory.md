# Phase 01 - Ownership Freeze And Application API Inventory

## Context Links
- `crates/chatminal-runtime/src/state.rs`
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `crates/chatminal-session-runtime/src/session_engine.rs`
- `apps/chatminal-desktop/src/chatminal_desktop_session.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: khóa ownership boundary, liệt kê toàn bộ desktop action/query nào còn đi tắt qua session-runtime hoặc host-runtime thay vì qua `chatminal-runtime`.

## Key Insights
- Muốn đồng bộ tối đa thì trước hết phải có bảng ownership duy nhất cho từng loại state.
- Không được implement mù; phải freeze inventory line-by-line trước khi di chuyển logic.

## Requirements
- Chỉ rõ source of truth cho: `active_profile_id`, `active_session_id`, `workspace_layout`, `session runtime`, `terminal_instance`, `render snapshot`.
- Liệt kê toàn bộ desktop callsite mutation/query đang bypass `chatminal-runtime`.
- Chốt public facade cần có ở `chatminal-runtime` để desktop không còn phải chạm trực tiếp lớp dưới.

## Architecture
- Ownership table đích:
  - `chatminal-runtime`: profile/session/workspace/layout/lifecycle/event facade
  - `chatminal-session-runtime`: runtime_id/terminal_instance/live execution
  - `desktop_host_runtime`: render/pty/target/window adapter
  - `desktop`: view + input dispatch only

## Related Code Files
- Refactor: `crates/chatminal-runtime/src/state.rs`
- Refactor: `crates/chatminal-runtime/src/state/native_api.rs`
- Refactor: `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_desktop_session.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`

## Implementation Steps
1. Freeze ownership matrix cho từng state target.
2. Inventory mọi desktop mutation/query bypass runtime facade.
3. Nhóm callsite thành `mutation`, `query`, `subscription`, `engine-only`.
4. Thiết kế facade API tối thiểu cho Phase 02.

## Todo List
- [x] Ownership matrix hoàn chỉnh
- [x] Desktop bypass inventory hoàn chỉnh
- [x] Facade API inventory hoàn chỉnh
- [x] Grep checkpoint được lưu trong phase notes

## Current Notes
- Ownership matrix đã freeze:
  - `chatminal-runtime`
    - source of truth cho `active_profile_id`, `active_session_id`, persisted `workspace_layout`, profile/session lifecycle, publish workspace/session updates
    - entrypoints thực tế: `state.rs` (`profile_create`, `profile_switch`, `session_create`, `session_activate`, `session_close`, `workspace_layout_load/save/clear`), `native_api.rs`, `runtime_bridge.rs`, `runtime_lifecycle.rs`
  - `chatminal-session-runtime`
    - source of truth cho `runtime_id`, `terminal_instance_id`, layout/runtime snapshots, execution graph
    - public engine surface hiện tại tập trung ở `session_engine.rs`
  - `desktop_host_runtime`
    - đúng ra phải chỉ là engine adapter/render host, nhưng inventory cho thấy hiện còn giữ derived ownership/business routing ở `session_host.rs`
  - `apps/chatminal-desktop`
    - nhiều path đã đi qua facade `chatminal_runtime/client.rs`, nhưng vẫn còn desktop-side orchestration ở `chatminal_desktop_session.rs`, `chatminal_layout/workspace_store.rs`, `desktop_host_runtime/session_host.rs`
- Desktop bypass inventory đã freeze:
  - `mutation`
    - `chatminal_sidebar/mod.rs`: gọi `runtime_client()?.session_activate/session_close/profile_switch`
    - `chatminal_layout/workspace_store.rs`: gọi `runtime.state.workspace_layout_load/save/clear`
    - `chatminal_desktop_session.rs`: create/split/focus/layout mutate qua `DesktopWorkspaceLayoutStore` + `DesktopSessionHost`
    - `desktop_host_runtime/session_host.rs`: trực tiếp `persist_layout/load_persisted_layout`, giữ `active_session_id`, `last_active_session_id`, `current_layout`
  - `query`
    - `chatminal_desktop_session.rs`: `current_active_session_id`, `current_layout`, `view_id_for_session`
    - `termwindow/mod.rs`: active session/view/runtime lookup ghép từ sidebar snapshot + desktop session host + host metadata
  - `subscription`
    - `chatminal_runtime/client.rs`: subscribe runtime events qua `RuntimeSubscription`
    - desktop vẫn còn join thêm lookup/host state thay vì dựa vào snapshot contract duy nhất
  - `engine-only` match hợp lệ giữ lại cho phase sau
    - `desktop_host_runtime/engine_runtime_adapter.rs`
    - `desktop_host_runtime/session_pane.rs`
    - `desktop_host_runtime/mod.rs`
    - `desktop_termwindow_layout_render.rs`
    - `chatminal_render/mod.rs`
    - `tabbar.rs`
- Grep checkpoints ghi nhận:
  - direct `chatminal_session_runtime` imports ở desktop app-facing path hiện còn tại `chatminal_desktop_session.rs`, `chatminal_layout/workspace_store.rs`, `chatminal_runtime/client.rs`, `termwindow/mod.rs`
  - local workspace persistence còn ở `chatminal_layout/workspace_store.rs`, `chatminal_desktop_session.rs`, `desktop_host_runtime/session_host.rs`
  - host-derived active ownership còn ở `desktop_host_runtime/session_host.rs`
- Facade API inventory tối thiểu cho Phase 02:
  - `workspace_snapshot_desktop()`
  - `workspace_layout_load/save/clear`
  - `workspace_split_view/focus_view/resize_split/close_view/attach_session`
  - `session_activate/close/create/clone`
  - `session_focus_terminal_instance/move_terminal_instance/close_terminal_instance`
  - `desktop_reconcile_lookup` / `desktop_runtime_events`

## Success Criteria
- Không còn mơ hồ về owner của từng state.
- Có danh sách đầy đủ callsite cần migrate trong các phase sau.

## Risk Assessment
- Risk: bỏ sót desktop bypass path nhỏ lẻ trong `termwindow` helper files.
- Mitigation: grep theo action verbs + runtime ids + session/layout mutations trước khi sang Phase 02.

## Security Considerations
- Không đổi behavior runtime ở phase này; chỉ freeze boundary và inventory.

## Next Steps
- Phase 02 tạo facade application-level và chuyển ownership thật vào `chatminal-runtime`.
