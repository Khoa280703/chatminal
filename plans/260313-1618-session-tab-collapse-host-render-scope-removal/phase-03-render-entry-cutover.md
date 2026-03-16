# Phase 03 — Render Entry Cutover

**Status:** completed
**Priority:** P1
**Effort:** 1d
**Blocked by:** Phase 02

## Goal

Thay cầu nối `session_id → HostRenderScope → pane` trong render pipeline bằng `session_id → DesktopSessionHost.pane_for_session`. `HostRenderScope` không còn được tạo trong `sync_render_state_for_runtime`. Sau phase này GPU draw path không đổi — chỉ thay nguồn pane.

## Context links

- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs:93-160` — `layout_positioned_panes` consumer chính
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs:162-` — `layout_positioned_splits`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs:830,840` — `desktop_render_scope_id_for_session`, `desktop_render_state_for_session`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:410-538` — `sync_render_state_for_runtime`
- `apps/chatminal-desktop/src/chatminal_render/` — `ChatminalRenderState`, `ChatminalRenderPane`

## Render pipeline — before vs after

**Before** (`layout_positioned_panes`):
```
target.session_id
  → desktop_render_scope_id_for_session   ← lookup HostRenderScope
  → desktop_render_state_for_session      ← ChatminalRenderState built từ HostRenderScope.iter_panes()
  → render_pane.terminal_handle → terminal_handle_arc_by_public_id → Arc<dyn HostTerminal>
  → TerminalPaneLayout { pane, geometry }
```

**After**:
```
target.session_id
  → desktop_pane_for_session              ← lookup trực tiếp từ DesktopSessionHost.session_pane
  → ChatminalRenderState built từ single pane (1 session = 1 pane)
  → TerminalPaneLayout { pane: Arc<ChatminalSessionPane>, geometry (từ LayoutRenderTarget) }
```

GPU draw (`paint_pane`) nhận `TerminalPaneLayout` — **không thay đổi**.

## Key changes

### 1. `sync_render_state_for_runtime` — xóa `Arc<HostRenderScope>`

Hiện tại (session_host.rs:470):
```rust
let tab = Arc::new(HostRenderScope::new(&mux_size));
// ... sync_with_pane_tree ...
let render_state = ChatminalRenderState {
    panes: tab.iter_panes().into_iter().map(|pos| ChatminalRenderPane { ... }).collect(),
    splits: tab.iter_splits()...
};
```

Sau khi sửa — build `ChatminalRenderState` trực tiếp từ `session_pane` map (không qua HostRenderScope):
```rust
// Real struct: ChatminalRenderState { render_target, terminal_size,
//   active_terminal_instance_id, panes: Vec<ChatminalRenderPane>, splits: Vec<ChatminalRenderSplit> }
// 1 session = 1 pane → panes có đúng 1 phần tử; splits là [] (splits ở workspace layout level, không phải session level)
// Geometry KHÔNG build ở đây — đến từ LayoutRenderTarget (path không thay đổi)
let pane = session_pane_guard[&session_id].clone();
let render_state = ChatminalRenderState {
    render_target: runtime_snapshot.render_target.clone(),
    terminal_size: pane.terminal_size(),
    active_terminal_instance_id: Some(pane.terminal_instance_id()),
    panes: vec![ChatminalRenderPane {
        terminal_handle: SessionTerminalHandle::new(pane.pane_id_value() as u64),
        terminal_instance_id: pane.terminal_instance_id(),
        ..
    }],
    splits: vec![],
};
```
**Geometry contract**: `TerminalPaneLayout.geometry` tiếp tục đến từ `LayoutRenderTarget` (path không thay đổi). Phase 03 chỉ thay nguồn pane lookup — không động đến geometry path.

### 2. `desktop_render_state_for_session` — đọc từ `DesktopSessionHost`

Hàm này hiện gọi `render_state_for_runtime` (tra `runtime_render_state` map). Sau Phase 02 map này vẫn tồn tại nhưng được populate bằng cách mới (không qua HostRenderScope). Không cần thay interface hàm này.

### 3. `desktop_render_scope_id_for_session` — xóa sau khi tất cả call-site chuyển xong

| Consumer | File | Migration |
|----------|------|-----------|
| `layout_positioned_panes` overlay check | `layout_render.rs:99,103` | pane_id → overlay lookup |
| `render_capability_for_layout_split` (splits + fallback scope_id) | `layout_render.rs:202` | Dùng `WorkspaceLayoutState.splits` trực tiếp — không cần scope_id |
| `desktop_render_scope_id_for_session` bridge call | `chatminal_runtime/mod.rs:685` | Xóa sau khi consumer migrate |
| `desktop_render_state_for_session` bridge call | `chatminal_runtime/mod.rs:252` | Xóa sau khi consumer migrate |
| `positioned_panes_for_session` (scope_id + render_state.panes) | `positioned_session_helpers.rs:4,33` | Migrate sang single pane lookup (1 session = 1 pane) |
| `positioned_splits_for_session` (scope_id + render_state.splits) | `positioned_session_helpers.rs:4,64` | Migrate sang `WorkspaceLayoutState` workspace splits |

**Splits model sau Phase 03:**
- `ChatminalRenderState.splits = []` — session không còn own splits
- Splits ở workspace layout level (`WorkspaceLayoutState`) — `render_capability_for_layout_split` và `positioned_splits_for_session` phải đọc từ workspace, không từ session render state

**Overlay contract:** Migrate từ `render_scope_id` (Tab id) sang `pane_id` lookup:
```rust
// Thay: render_scope_id → render_target_overlay(id)
// Thành: pane_id → terminal_ui_state(pane_id).overlay
```

## Implementation steps

1. **Sửa `sync_render_state_for_runtime`** — build `ChatminalRenderState` trực tiếp từ `session_pane[session_id]` (single pane); `splits = []`.
2. **Sửa `layout_positioned_panes`** — xóa `desktop_render_scope_id_for_session` overlay check; thay bằng lookup pane_id → overlay.
3. **Sửa `render_capability_for_layout_split`** — đọc splits từ `WorkspaceLayoutState` (workspace level), bỏ fallback `desktop_render_scope_id_for_session`.
4. **Sửa `positioned_panes_for_session`** — thay scope_id overlay check → pane_id overlay check; bỏ iteration `render_state.panes` (lấy single pane từ `session_pane[session_id]` thay thế).
5. **Sửa `positioned_splits_for_session`** — đọc splits từ `WorkspaceLayoutState`, không dùng `render_state.splits` hay `render_scope_id`.
5. **Sửa `desktop_render_state_for_session` consumers** tại `chatminal_runtime/mod.rs:252,685` — chuyển sang đọc trực tiếp từ `DesktopSessionHost`.
6. **Giữ `HostMux.add_pane`** — pane vẫn phải đăng ký với Mux để `terminal_handle_arc_by_public_id` hoạt động.
7. **Xóa `render_scope_for_runtime`** khỏi `DesktopSessionHost` (hoặc `#[allow(dead_code)]` đến Phase 06).
8. **`EngineRuntimeAdapter` methods** vẫn dùng `HostRenderScope` cho focus — giữ nguyên đến Phase 05.
9. `cargo check --workspace` pass.
10. Smoke test: render session trong split layout.

## Related code files

**Sửa:**
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_positioned_session_helpers.rs` — `positioned_panes_for_session` + `positioned_splits_for_session`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` — overlay lookup, consumers mod.rs:252,685

## Todo

- [x] Sửa `sync_render_state_for_runtime` — build từ `session_pane[session_id]`, `splits = []`
- [x] Sửa `layout_positioned_panes` — thay overlay check → pane_id
- [x] Sửa `render_capability_for_layout_split` → WorkspaceLayoutState splits
- [x] Sửa `positioned_panes_for_session` → single pane lookup + pane_id overlay check
- [x] Sửa `positioned_splits_for_session` → WorkspaceLayoutState splits
- [x] Sửa consumers `chatminal_runtime/mod.rs:252,685`
- [x] `cargo check --workspace` pass
- [x] Visual smoke test: render session trong split layout

## Success criteria

- `layout_positioned_panes` trả về đúng panes với geometry đúng
- Overlay vẫn hoạt động
- `ChatminalRenderState` không còn dùng `HostRenderScope` để build pane list
- `cargo check --workspace` pass

## Risk

- Overlay lookup refactor nhỏ có thể ảnh hưởng launcher overlay — cần test riêng.

## Resolved

**`SessionTerminalInstanceSnapshot` KHÔNG có geometry fields** (`left/top/width/height/pixel_*`):
```rust
pub struct SessionTerminalInstanceSnapshot {
    pub terminal_instance_id: TerminalInstanceId,
    pub title: Option<String>,
    // không có geometry
}
```
→ **Không cần thêm.** Geometry đã đến từ `LayoutRenderTarget` (tính từ `WorkspaceLayoutState` trong `layout_render_targets()`). Phase 03 chỉ cần fix bridge `session_id → pane` — geometry path đã đúng rồi. Xóa todo item "kiểm tra geometry" ở trên.
