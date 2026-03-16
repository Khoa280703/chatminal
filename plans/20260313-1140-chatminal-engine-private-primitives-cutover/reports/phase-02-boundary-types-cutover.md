## Phase 02 Report

Status: completed

### Scope shipped
- Thêm boundary ids/app DTO ở `crates/chatminal-runtime/src/api/mod.rs`:
  - `SessionRenderTargetId`
  - `SessionRenderTargetSnapshot`
  - `SessionTerminalHandle`
  - `SessionLayoutTarget`
  - `SessionGroupId`
  - `SessionGroupSnapshot`
  - `SessionViewBinding`
  - `SessionWindowBinding`
  - `SessionEngineCapability`
- Export toàn bộ từ `crates/chatminal-runtime/src/lib.rs`.
- Refactor `apps/chatminal-desktop/src/chatminal_render/mod.rs` để render DTO dùng:
  - `SessionRenderTargetSnapshot`
  - `SessionTerminalHandle`
- Refactor `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`:
  - `desktop_render_scope_id_for_session -> Option<SessionRenderTargetId>`
  - `desktop_session_id_for_render_scope(SessionRenderTargetId)`
  - `desktop_current_active_terminal_handle -> Option<SessionTerminalHandle>`
  - thêm `desktop_session_view_binding`
  - thêm `desktop_window_binding`
  - `DesktopSessionRuntimeSummary` trả thêm render/view/group/window/capability boundary data
- Conversion từ host primitive sang boundary types chỉ nằm ở desktop private adapter:
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`

### Compatibility note
- `SessionGroupSnapshot` hiện là compatibility shape: mỗi `SessionView` map vào singleton synthetic group cùng id.
- Đây là chủ ý cho Phase 02 để có contract ổn định trước; ownership/grouping thật sẽ dồn tiếp ở Phase 03-04.

### Gates
- `cargo check -p chatminal-runtime`: pass
- `cargo check -p chatminal-desktop`: pass
- `cargo test -p chatminal-runtime`: pass
- `rg -n "host_runtime::(Mux|tab::Tab|pane::Pane)|\\bMuxWindow\\b|OverlayRenderScope" apps/chatminal-desktop/src/chatminal_runtime apps/chatminal-desktop/src/chatminal_render`: zero matches
- `rg -n "SessionRenderTargetId|SessionGroupId|SessionTerminalHandle|SessionWindowBinding" crates/chatminal-runtime apps/chatminal-desktop/src`: pass

### Review notes
- Boundary layer mới bọc đúng product-facing ids nhưng chưa thay hết callsites lookup/focus/close trong `termwindow`; phần đó thuộc Phase 03.
- Không tăng dependency mới từ desktop product path sang host primitives.
