## Phase 03 Batch 01

Status: in_progress

### Scope shipped
- Thêm `DesktopSessionTerminalBinding` vào desktop runtime facade.
- Thêm facade lookups/actions:
  - `desktop_session_terminal_binding`
  - `desktop_focus_session_terminal_handle`
  - `desktop_swap_active_with_terminal_handle`
  - `desktop_close_session_terminal_handle_or_session`
- Routing `focus/swap/close` ở session mode không còn đọc trực tiếp `pane metadata` từ `termwindow` helper.
- `positioned_pane_to_terminal_instance_info` ưu tiên resolve terminal-instance qua runtime facade thay vì metadata host.
- Thêm `DesktopSessionWindowSnapshot` để gom `window_binding + workspace_snapshot + lookup` vào một facade object.
- Thêm `DesktopSessionEntryBinding` và `desktop_session_entry_bindings` để facade own:
  - session ordering
  - active/last-active flags
  - view binding
  - render target binding
- Thêm facade helpers:
  - `desktop_ordered_session_ids`
  - `desktop_last_active_session_id`
  - `desktop_can_close_view_only`
- `termwindow` đã chuyển các query sau sang facade snapshot:
  - `active_session_id`
  - `active_view_id`
  - `active_render_scope_id`
  - phần lookup trong `sync_active_chatminal_session_from_mux`
  - phần lookup/workspace state trong `get_session_entry_information`
- `termwindow` không còn gọi trực tiếp `load_desktop_workspace_snapshot` hay `desktop_collect_session_lookup`.
- Thu hẹp `desktop_host_runtime/session_host.rs` xuống `pub(crate)`/private cho phần lifecycle + lookup host adapter.

### Files changed
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_session_close_helpers.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`

### Gates
- `cargo check -p chatminal-desktop`: pass

### What remains in Phase 03
- `termwindow` vẫn còn own nhiều `render_scope_id` routing cho overlay/close path.
- `active_render_scope_id` và `desktop_collect_session_lookup` vẫn còn được consume trực tiếp ở UI shell.
- `desktop_host_runtime/session_host.rs` vẫn còn public surface rộng hơn mức private adapter tối thiểu.
