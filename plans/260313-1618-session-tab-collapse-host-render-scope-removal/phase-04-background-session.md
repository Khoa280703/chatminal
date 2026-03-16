# Phase 04 — Background Session Support

**Status:** completed
**Priority:** P1
**Effort:** 0.5d
**Blocked by:** Phase 03

## Goal

Session có thể tồn tại mà không có layout slot (background). Close session trong split view → layout co lại, không để placeholder trống. Đây là product behavior đã confirm, không phải optional.

## Behavior mapping (chốt dứt khoát)

| Action | Behavior | Call path |
|--------|----------|-----------|
| `CloseCurrentSession` | **Hard delete**: stop PTY + xóa layout slot + **xóa SessionEntry khỏi DaemonState** | `close_runtime_session` → `session_close` → `remove_session_and_publish_workspace` |
| `DetachSession` (future keybind) | **Stop runtime + giữ metadata**: stop PTY + gỡ layout slot, **SessionEntry còn lại** trong DaemonState — có thể re-open session sau | `desktop_detach_session_runtime_and_notify` |

**Tại sao DetachSession = stop runtime + giữ metadata (không phải background PTY):**
- Background PTY (PTY chạy ẩn) cần quản lý thread lifecycle, reconnect → quá phức tạp, không cần thiết với model "session + profile"
- Model này: session name/config được giữ lại, re-open = tạo PTY mới (fresh start)
- `desktop_detach_session_runtime_and_notify` hiện tại đã implement đúng behavior này: `host.close_runtime` (kill PTY) + gỡ layout slot, SessionEntry vẫn còn

**Hiện trạng cần sửa:**
- `desktop_close_session_terminal_handle_or_session` (mod.rs:1049) hiện gọi `desktop_detach_session_runtime_and_notify` khi `panes.len() <= 1` — đây là detach behavior, không phải hard delete
- Phase 04 phải wire `CloseCurrentSession` qua `close_runtime_session` (xóa SessionEntry persisted)
- `desktop_detach_session_runtime_and_notify` giữ nguyên, dùng cho `DetachSession` keybind (future)

**Note:** `close_runtime` trong `session_host.rs` chỉ dọn host/runtime resources (PTY thread, pane maps) — không đụng DaemonState. `session_close` mới là API xóa SessionEntry.

## Context links

- `crates/chatminal-session-runtime/src/workspace_layout.rs` — `WorkspaceLayoutState`, layout close logic
- `crates/chatminal-session-runtime/src/session_engine_core.rs` — `close_leaf_native`, `close_runtime_native`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` — `close_runtime`, `close_leaf`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs` — `layout_render_targets` (skip sessions không có layout slot)

## Key insights

### Background session

Hiện tại: session luôn gắn vào layout khi được attach. Không có concept "session tồn tại nhưng không visible".

Mô hình mới:
- `WorkspaceLayoutState` track danh sách session IDs đang có layout slot.
- Session không có slot → `layout_render_targets()` bỏ qua → không render.
- **Background session = SessionEntry trong DaemonState nhưng không có layout slot và không có runtime đang chạy.** PTY không còn sống — session name/config được giữ để re-open sau (DetachSession behavior).

Đây chủ yếu là **behavior clarification**, không phải structural change lớn vì `layout_render_targets()` đã chỉ render sessions có trong `WorkspaceLayoutState`.

### Close in split view → layout collapse

Hiện tại khi close một view trong split: `session_engine_core` gọi close → workspace layout update. Cần kiểm tra `WorkspaceLayoutState` có tự co lại hay để node trống.

## Implementation steps

1. **Audit `WorkspaceLayoutState` close behavior**: đọc `workspace_layout_rebuild.rs` để xem close node có collapse sibling lên không.
2. Nếu chưa collapse: sửa `WorkspaceLayoutState::remove_view` hoặc equivalent để merge sibling vào parent khi một node bị xóa (giống tmux pane close).
3. **Wire `CloseCurrentSession` → hard delete path**: sửa `desktop_close_session_terminal_handle_or_session` để gọi `close_runtime_session(session_id)` thay vì `desktop_detach_session_runtime_and_notify` khi `panes.len() <= 1`. `close_runtime_session` gọi sang runtime server → `session_close` → `remove_session_and_publish_workspace` (xóa SessionEntry).
4. **Giữ `desktop_detach_session_runtime_and_notify`** như implementation cho `DetachSession` (future keybind) — stop PTY + remove layout slot, SessionEntry vẫn còn.
5. Nếu cần helper nội bộ cho workspace store, chỉ thêm helper phục vụ `desktop_detach_session_runtime_and_notify`/close flow; **không** định nghĩa public behavior mới kiểu "layout-only detach, PTY vẫn sống".
6. `cargo test -p chatminal-session-runtime -- --test-threads=1` pass.

## Related code files

**Đọc để audit:**
- `crates/chatminal-session-runtime/src/workspace_layout_rebuild.rs`
- `crates/chatminal-session-runtime/src/session_engine_core.rs`

**Có thể sửa:**
- `crates/chatminal-session-runtime/src/workspace_layout.rs`
- `crates/chatminal-session-runtime/src/workspace_layout_rebuild.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`

## Todo

- [x] Audit `WorkspaceLayoutState` — xác nhận close/collapse behavior
- [x] Fix collapse nếu cần (sibling node merge lên parent)
- [x] Wire `CloseCurrentSession` → `close_runtime_session` (hard delete: PTY + SessionEntry xóa)
- [x] Giữ `DetachSession` semantics = stop runtime + giữ metadata; không tạo thêm path "layout-only detach"
- [x] `cargo test -p chatminal-session-runtime -- --test-threads=1` pass

## Success criteria

- Close một session trong 2-view split → window còn lại chiếm toàn bộ không gian
- `CloseCurrentSession` → gọi `close_runtime_session` → SessionEntry bị xóa khỏi DaemonState
- `DetachSession` (future) → `desktop_detach_session_runtime_and_notify` → SessionEntry còn lại trong DaemonState
- Hai paths không bị nhầm lẫn
- Không có empty placeholder trong render
- Tests pass

## Risk

- Nếu `WorkspaceLayoutState` chưa có collapse logic, đây là thay đổi nhỏ nhưng cần test kỹ các layout edge cases (3-way split, nested split).
