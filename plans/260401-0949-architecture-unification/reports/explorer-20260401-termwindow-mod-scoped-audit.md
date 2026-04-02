# Scoped Audit: termwindow/mod.rs

Context
- Work context: `/Users/khoa2807/development/2026/chatminal`
- File: [apps/chatminal-desktop/src/termwindow/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs)
- Goal: map `crate::chatminal_runtime::...` callsites by family and identify low-risk patch cuts.

Summary
- Found 30 `crate::chatminal_runtime::...` callsites in [termwindow/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs), plus 2 top-level imports from `crate::chatminal_runtime` that are still host-side bridge surface:
  - `overlay_compat` import at [termwindow/mod.rs:7](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L7)
  - `PrimaryHostWindowId`, `RuntimeNotification`, `RuntimeWindow` import at [termwindow/mod.rs:12](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L12)
- No direct `Mux::get()` / `HostMux::get()` in this file.
- Most callsites are already going through `chatminal_runtime` facade/native desktop runtime APIs, not raw host bridge leakage.
- The file has 2 clearly separable buckets:
  - small host/window/pane compat bucket
  - larger desktop session/workspace bucket

## 1. Window/Shell family
These are host-window style helpers, currently already available through facade functions in `chatminal_runtime/mod.rs`.

- [termwindow/mod.rs:274](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L274) `resolved_window_title()`
- [termwindow/mod.rs:1667](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1667) `active_host_runtime_entry_size()`
- [termwindow/mod.rs:1710](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1710) `resize_host_window_tabs(...)`
- [termwindow/mod.rs:1891](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1891) `host_window_initial_position()`
- [termwindow/mod.rs:2592](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L2592) `subscribe_runtime_notifications(...)`

Notes
- Đây là nhóm patch an toàn nhất: mechanical import cleanup, không cần đổi control flow.
- `RuntimeNotification`, `RuntimeWindow`, `PrimaryHostWindowId` hiện vẫn là host-side explicit re-export ở đầu file.

## 2. Pane/Render-Target compat family
Đây là các helper bắc cầu giữa terminal UI và session-native render target/pane binding.

- [termwindow/mod.rs:323](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L323) `resolve_public_pane(...)`
- [termwindow/mod.rs:1190](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1190) `desktop_render_state_for_session(...)`
- [termwindow/mod.rs:1193](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1193) `host_active_render_scope_id()`
- [termwindow/mod.rs:1262](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1262) `desktop_session_entry_binding_for_render_target(...)`
- [termwindow/mod.rs:1263](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1263) `SessionRenderTargetId::new(...)`
- [termwindow/mod.rs:1277](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1277) `desktop_session_entry_binding_for_render_target(...)`
- [termwindow/mod.rs:1278](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1278) `SessionRenderTargetId::new(...)`
- [termwindow/mod.rs:1282](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1282) `desktop_pane_for_session(...)`
- [termwindow/mod.rs:2616](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L2616) `desktop_session_terminal_binding(...)`
- [termwindow/mod.rs:2617](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L2617) `SessionTerminalHandle::new(...)`
- [termwindow/mod.rs:2646](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L2646) `desktop_session_entry_bindings()`

Notes
- Nhóm này vẫn an toàn nếu chỉ chuyển sang import rõ ở đầu file.
- Không nên đổi logic render/selection ở cùng patch.

## 3. Frontend/Workspace family
Không thấy callsite nào dùng trực tiếp các helper facade kiểu `active_frontend_client`, `active_workspace_for_client`, `workspace_names`, `workspace_is_empty`, `set_active_workspace_for_client` trong file này.

Thay vào đó, file này dùng desktop session/window snapshot APIs của runtime session model:
- [termwindow/mod.rs:1238](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1238) `desktop_session_window_snapshot()`
- [termwindow/mod.rs:1245](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1245) `reconcile_runtime_session_lookup(...)`
- [termwindow/mod.rs:1407](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1407) `desktop_can_close_view_only()`

Conclusion
- Theo family bạn yêu cầu, bucket `frontend/workspace` trong file này thực tế gần như trống ở lớp host facade; thay vào đó là bucket native desktop workspace/session API.

## 4. Native runtime API family
Đây là phần lớn callsites trong session-native path. Chúng không phải host-bridge leakage; chúng là API business/runtime thật của Chatminal.

- Layout prepare/resize:
  - [termwindow/mod.rs:695](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L695) `desktop_prepare_workspace_layout(...)`
  - [termwindow/mod.rs:696](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L696) `desktop_resize_visible_sessions(...)`
  - [termwindow/mod.rs:724](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L724) `desktop_prepare_workspace_layout(...)`
  - [termwindow/mod.rs:725](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L725) `desktop_resize_visible_sessions(...)`
  - [termwindow/mod.rs:992](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L992) `desktop_prepare_workspace_layout(...)`
  - [termwindow/mod.rs:1138](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1138) `desktop_prepare_workspace_layout(...)`
- Session/window state:
  - [termwindow/mod.rs:740](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L740) `desktop_render_state_for_session(...)`
  - [termwindow/mod.rs:1091](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1091) `desktop_last_active_session_id()`
  - [termwindow/mod.rs:1154](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1154) `DesktopSessionRuntimeSummary`
  - [termwindow/mod.rs:1159](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1159) `desktop_activate_session(...)`
  - [termwindow/mod.rs:1166](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1166) `notify_runtime_session_activated(...)`
  - [termwindow/mod.rs:1313](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1313) `desktop_detach_session_runtime_and_notify(...)`
  - [termwindow/mod.rs:1327](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1327) `desktop_detach_session_runtime_and_notify(...)`
  - [termwindow/mod.rs:1387](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1387) `desktop_focus_session_view_with_previous(...)`
  - [termwindow/mod.rs:1489](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1489) `run_runtime_session_startup_command(...)`
- Workspace model types/helpers:
  - [termwindow/mod.rs:933](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L933) `WorkspaceLayoutState::grouped_sessions(...)`
  - [termwindow/mod.rs:983](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L983) `WorkspaceLayoutState::grouped_sessions(...)`
  - [termwindow/mod.rs:1230](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1230) `WorkspaceLayoutState`

Notes
- Đây là business/runtime API thật. Không nên trộn patch cleanup family này với patch host-facade mechanical nếu mục tiêu là diff nhỏ và an toàn.

## Safe patch cuts đề xuất

### Cut 1: host-facade mechanical import cleanup
Scope
- Chỉ động [termwindow/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs)
- Chỉ với symbols đã là facade/compat rõ ràng:
  - `resolved_window_title`
  - `resolve_public_pane`
  - `host_active_render_scope_id`
  - `active_host_runtime_entry_size`
  - `resize_host_window_tabs`
  - `host_window_initial_position`
  - `subscribe_runtime_notifications`
  - `desktop_render_state_for_session`
  - `desktop_session_entry_binding_for_render_target`
  - `desktop_session_terminal_binding`
  - `desktop_session_entry_bindings`
  - `SessionRenderTargetId`
  - `SessionTerminalHandle`

Why safe
- Mostly import cleanup.
- Không đụng control flow hoặc layout logic.
- Dễ review, diff nhỏ.

### Cut 2: session-native runtime batch
Scope
- Cũng chỉ [termwindow/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs)
- Tập trung đoạn [termwindow/mod.rs:695](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L695) đến [termwindow/mod.rs:1489](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs#L1489)
- Import/localize desktop session/workspace APIs:
  - `desktop_prepare_workspace_layout`
  - `desktop_resize_visible_sessions`
  - `desktop_last_active_session_id`
  - `desktop_activate_session`
  - `notify_runtime_session_activated`
  - `desktop_session_window_snapshot`
  - `reconcile_runtime_session_lookup`
  - `desktop_detach_session_runtime_and_notify`
  - `desktop_focus_session_view_with_previous`
  - `desktop_can_close_view_only`
  - `run_runtime_session_startup_command`
  - `DesktopSessionRuntimeSummary`
  - `WorkspaceLayoutState`

Why safe
- Đây là một cluster chức năng khá tự nhiên: session switching / layout sync / close / startup command.
- Không cần đụng paint/input/render core.
- Giữ lead trong một vùng logic liên quan với nhau thay vì bắn lẻ từng callsite.

## Recommendation
- Làm Cut 1 trước để giảm noise và xác nhận import surface sạch.
- Sau đó làm Cut 2 nếu muốn tiếp tục hạ residual `crate::chatminal_runtime::...` density trong file này.
- Chưa nên đụng phần `overlay_compat` import và host-side types ở đầu file cùng lúc với Cut 2; tách riêng sẽ an toàn hơn.

Unresolved questions
- Có muốn tách riêng một Cut 0 cực nhỏ chỉ xử lý top-level imports `overlay_compat`, `RuntimeWindow`, `RuntimeNotification`, `PrimaryHostWindowId` không, hay để sau cùng khi residual explicit re-export trong `chatminal_runtime/mod.rs` giảm thêm?
- Lead có muốn giữ `crate::chatminal_runtime::...` ở các native runtime APIs như một tín hiệu boundary rõ ràng, hay cũng muốn import local hết để file ngắn hơn?
