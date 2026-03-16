# Phase 02 — Direct Pane Ownership

**Status:** completed
**Priority:** P1
**Effort:** 1d
**Blocked by:** Phase 01

## Goal

`DesktopSessionHost` thêm lookup `session_id → Arc<ChatminalSessionPane>` trực tiếp (1 session = 1 pane — invariant đã frozen ở Phase 01). Spawn session tạo pane mà không cần tạo `HostRenderScope` trước. `HostRenderScope` vẫn được tạo sau (Phase 03 sẽ xóa luôn), nhưng logic spawn không còn phụ thuộc vào nó.

## Context links

- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` — target chính
- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs` — `spawn_runtime_inner` dùng `HostMux.spawn_tab_or_window`
- `crates/chatminal-session-runtime/src/session_core_state.rs` — `SessionCoreState`, `SessionRuntimeRecord`

## Key insights

- `DesktopSessionHost.panes` hiện là `HashMap<TerminalInstanceId, Arc<ChatminalSessionPane>>` — lookup theo terminal instance, không theo session.
- `sync_render_state_for_runtime` build `ChatminalRenderState` bằng cách tạo `Arc<HostRenderScope>` tạm rồi gọi `tab.iter_panes()` → cần tách phần build ChatminalRenderState ra khỏi HostRenderScope.
- `EngineRuntimeAdapter.spawn_runtime` vẫn gọi `HostMux.spawn_tab_or_window` → OK, giữ nguyên trong Phase này vì đó là engine bootstrap. Sẽ review lại ở Phase 05.

## Architecture changes

### Thêm vào `DesktopSessionHost`

```rust
// session_id → pane for that session (1 session = 1 pane, invariant frozen)
session_pane: Mutex<HashMap<String, Arc<ChatminalSessionPane>>>,
```

### Thêm public method

```rust
pub(crate) fn pane_for_session(
    &self,
    session_id: &str,
) -> Option<Arc<ChatminalSessionPane>>;
```

### Sửa `sync_render_state_for_runtime`

Tách hàm thành hai phần:
1. `sync_session_pane_index` — đồng bộ `session_pane` map từ `panes` map (terminal_instance → session)
2. Phần build `ChatminalRenderState` giữ nguyên dùng HostRenderScope (sẽ thay ở Phase 03)

## Implementation steps

1. Thêm field `session_pane: Mutex<HashMap<String, Arc<ChatminalSessionPane>>>` vào `DesktopSessionHost`.
2. Trong `sync_render_state_for_runtime`, sau khi pane được tạo, cập nhật `session_pane`:
   - Lấy `session_id` từ `pane.session_id()`.
   - **Invariant enforcement**: nếu `session_pane` đã có entry cho `session_id` → `debug_assert!(false, "invariant violated: 1 session = 1 pane")` + `log::error!` + skip (không overwrite âm thầm).
   - Nếu chưa có → insert pane mới.
3. Trong `remove_runtime_resources`, xóa session_id tương ứng khỏi `session_pane`.
4. Thêm `pane_for_session` method.
5. Expose lên `chatminal_runtime/mod.rs` qua bridge mới `desktop_pane_for_session(window_id, session_id) -> Option<Arc<ChatminalSessionPane>>`.
6. `cargo check --workspace` pass.

## Related code files

**Sửa:**
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`

**Thêm (vào mod.rs hiện có):**
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` — thêm `desktop_pane_for_session`

## Todo

- [x] Thêm `session_pane` field vào `DesktopSessionHost`
- [x] Cập nhật `sync_render_state_for_runtime` để populate `session_pane`
- [x] Cập nhật `remove_runtime_resources` để dọn `session_pane`
- [x] Thêm `pane_for_session`
- [x] Expose `desktop_pane_for_session` qua `chatminal_runtime/mod.rs`
- [x] `cargo check --workspace` pass

## Success criteria

- `desktop_pane_for_session(window_id, "my-session")` trả về `Some(Arc<ChatminalSessionPane>)` khi session đang chạy
- Không có regression trong spawn / close flows
- `cargo check --workspace` pass

## Risk

- Race condition trong `Mutex`? Giữ cùng lock pattern với `panes` field hiện tại.
- Session chưa started → `pane_for_session` trả `None` → consumer phải handle gracefully.
