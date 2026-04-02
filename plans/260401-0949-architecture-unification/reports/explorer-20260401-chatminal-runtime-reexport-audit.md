# Re-export Audit: `chatminal_runtime/mod.rs` -> `desktop_host_runtime::*`

Scope:
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- callers under `apps/chatminal-desktop/src/**`

Key point:
- Không phải mọi `chatminal_runtime::...` callsite đều là vấn đề.
- Subset đang leak qua `pub(crate) use crate::desktop_host_runtime::*;` là 31 symbols.
- Phần còn lại là API native thật sự của `chatminal_runtime/mod.rs` hoặc `pub use chatminal_runtime::{...}` từ crate runtime.

## Symbols caller đang dùng nhưng thực chất đi từ wildcard re-export

### 1. Entry/bootstrap/runtime-host lifecycle
- `HostSpawnTargetHandle`
  - callers: `main.rs:183,192,272`
- `set_host_spawn_target`
  - callers: `main.rs:153`
- `primary_host_spawn_target`
  - callers: `main.rs:195,288`
- `host_has_panes_in_workspace`
  - callers: `main.rs:187`
- `activate_host_runtime_entry`
  - callers: `main.rs:302`
- `show_host_configuration_error_message`
  - callers: `main.rs:410,423`
- `start_host_activity`
  - callers: `frontend.rs:56`, `desktop_spawn.rs:96`, `main.rs:341`
- `host_activity_count`
  - callers: `frontend.rs:156`
- `HostActivityGuard`
  - callers: `frontend.rs:31`

### 2. Window/shell host facade
- `host_workspace_name`
  - callers: `frontend.rs:65`, `desktop_spawn.rs:97`, `desktop_termwindow_host_runtime_helpers.rs:78`
- `host_window_exists`
  - callers: `frontend.rs:469`, `desktop_termwindow_host_runtime_helpers.rs:70`
- `host_window_contains_render_scope`
  - callers: `desktop_termwindow_host_runtime_helpers.rs:74`
- `host_window_initial_position`
  - callers: `termwindow/mod.rs:1891`
- `resolved_window_title`
  - callers: `termwindow/mod.rs:274`
- `active_host_runtime_entry_size`
  - callers: `termwindow/mod.rs:1667`
- `resize_host_window_tabs`
  - callers: `termwindow/mod.rs:1710`, `termwindow/resize.rs:341`
- `with_host_window`
  - callers: `desktop_termwindow_host_runtime_helpers.rs:55`
- `with_host_window_mut`
  - callers: `desktop_termwindow_host_runtime_helpers.rs:62`

### 3. Terminal/pane/render-target compat facade
- `terminal_handle_arc`
  - callers: `desktop_termwindow_host_runtime_helpers.rs:39`
- `terminal_handle_arc_by_public_id`
  - callers: `desktop_termwindow_layout_render.rs:140`
- `remove_terminal_handle`
  - callers: `desktop_termwindow_host_runtime_helpers.rs:47`
- `remove_runtime_entry_scope`
  - callers: `desktop_termwindow_host_runtime_helpers.rs:66`, `overlay/confirm_close_pane.rs:10`
- `resolve_public_pane`
  - callers: `termwindow/mod.rs:323`
- `focus_terminal_handle`
  - callers: `desktop_termwindow_host_runtime_helpers.rs:159`
- `record_host_focus_for_current_identity`
  - callers: `termwindow/render/paint.rs:254`
- `host_active_render_scope_id`
  - callers: `termwindow/mod.rs:1193`
- `frontend_resolve_pane`
  - callers: `overlay/copy.rs:123`
- `subscribe_runtime_notifications`
  - callers: `termwindow/mod.rs:2592`
- `RuntimeNotification`
  - callers: `desktop_host_runtime/session_pane.rs:47`

### 4. Overlay/launcher compat surface
- `launcher_sessions`
  - callers: `overlay/launcher.rs:57`
- `LauncherSessionEntry`
  - callers: `overlay/launcher.rs:8`

## Wrapper explicit nên thêm vào `chatminal_runtime/mod.rs`

### P0. Entry/bootstrap facade
Mục tiêu: cắt `main.rs`, `frontend.rs`, `desktop_spawn.rs` khỏi wildcard sớm nhất.

Nên có explicit surface riêng, cùng namespace, không cần đổi behavior:
- `pub(crate) use crate::desktop_host_runtime::HostSpawnTargetHandle;`
- `pub(crate) use crate::desktop_host_runtime::HostActivityGuard;`
- `pub(crate) fn desktop_set_host_spawn_target(...)`
- `pub(crate) fn desktop_primary_host_spawn_target(...)`
- `pub(crate) fn desktop_host_has_panes_in_workspace(...)`
- `pub(crate) fn desktop_activate_host_runtime_entry(...)`
- `pub(crate) fn desktop_start_host_activity(...)`
- `pub(crate) fn desktop_host_activity_count(...)`
- `pub(crate) fn desktop_show_host_configuration_error_message(...)`

Lý do:
- caller ít
- gần entrypoint nhất
- tách được phần bootstrap/lifecycle khỏi wildcard trước
- không đụng terminal shell sâu

### P1. Window/shell facade
Mục tiêu: gom API host-window còn bị termwindow/frontend gọi trực tiếp.

Nên có:
- `pub(crate) fn desktop_host_workspace_name()`
- `pub(crate) fn desktop_host_window_exists()`
- `pub(crate) fn desktop_host_window_contains_render_scope()`
- `pub(crate) fn desktop_host_window_initial_position()`
- `pub(crate) fn desktop_resolved_window_title()`
- `pub(crate) fn desktop_active_host_runtime_entry_size()`
- `pub(crate) fn desktop_resize_host_window_tabs()`
- `pub(crate) fn desktop_with_host_window()`
- `pub(crate) fn desktop_with_host_window_mut()`

Lý do:
- đây là cluster caller lớn thứ hai
- tập trung ở `termwindow/*`, `frontend.rs`, `desktop_termwindow_host_runtime_helpers.rs`
- sau bước này wildcard gần như không còn cần cho shell window path

### P2. Pane/render-target compat facade
Mục tiêu: cô lập overlay/compat shell khỏi `desktop_host_runtime` names trực tiếp.

Nên có:
- `pub(crate) fn desktop_terminal_handle_arc()`
- `pub(crate) fn desktop_terminal_handle_arc_by_public_id()`
- `pub(crate) fn desktop_remove_terminal_handle()`
- `pub(crate) fn desktop_remove_runtime_entry_scope()`
- `pub(crate) fn desktop_resolve_public_pane()`
- `pub(crate) fn desktop_focus_terminal_handle()`
- `pub(crate) fn desktop_record_host_focus_for_current_identity()`
- `pub(crate) fn desktop_host_active_render_scope_id()`
- `pub(crate) fn desktop_frontend_resolve_pane()`
- `pub(crate) fn desktop_subscribe_runtime_notifications()`
- `pub(crate) use crate::desktop_host_runtime::RuntimeNotification;`

Lý do:
- nhiều caller ở shell/overlay sâu hơn
- tốt để làm sau P0/P1 vì dễ rename mechanical
- đây vẫn là compat layer, chưa phải Phase 03C

### P3. Launcher/overlay type facade
Nên có explicit re-export hoặc wrapper rõ ràng:
- `pub(crate) use crate::desktop_host_runtime::LauncherSessionEntry;`
- `pub(crate) fn desktop_launcher_sessions()`

## Ưu tiên caller gần entrypoint/UI shell nhất

### Ưu tiên 1
- `main.rs`
- `frontend.rs`
- `desktop_spawn.rs`

Đây là nhóm ít file, ít callsites, lợi nhuận cao. Cắt xong sẽ loại phần lớn bootstrap/lifecycle dependence lên wildcard.

### Ưu tiên 2
- `desktop_termwindow_host_runtime_helpers.rs`
- `termwindow/mod.rs`
- `termwindow/resize.rs`
- `termwindow/render/paint.rs`

Đây là nhóm shell window/runtime helper. Nên cắt theo cluster window facade trước, rồi pane/render-target facade sau.

### Ưu tiên 3
- `overlay/launcher.rs`
- `overlay/copy.rs`
- `overlay/confirm_close_pane.rs`
- `desktop_termwindow_layout_render.rs`

Nhóm này phụ thuộc ít, phù hợp làm sau khi P1/P2 đã có explicit API.

## Nhận định kiến trúc
- Wildcard hiện tại không làm logic sai, nhưng làm biên public trong `chatminal_runtime/mod.rs` mờ.
- Nhiều symbol leak ra ngoài thực chất chỉ là desktop host compat API, không phải runtime domain API.
- Hướng đúng: giữ `chatminal_runtime/mod.rs` là desktop facade có chủ đích; thêm wrapper/re-export tường minh theo cluster, rồi xóa wildcard cuối cùng.
- Không cần refactor lớn ngay. Chỉ cần cắt theo cụm nhỏ, compile/test mỗi cụm.

## Fastest safe cut order
1. P0 entry/bootstrap facade
2. P1 window/shell facade
3. P2 pane/render-target compat facade
4. P3 launcher/overlay types
5. Sau đó mới xóa `pub(crate) use crate::desktop_host_runtime::*;`

## Unresolved questions
- Có muốn giữ naming hiện tại để giảm churn (`host_*`) hay đổi đồng loạt sang `desktop_*` cho rõ boundary?
- `with_host_window` / `with_host_window_mut` có được xem là API chấp nhận được ở facade dài hạn, hay chỉ là bridge tạm cần xóa sau 03C?
