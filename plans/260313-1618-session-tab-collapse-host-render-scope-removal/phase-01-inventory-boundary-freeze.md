# Phase 01 — Model Freeze + Inventory + Boundary Freeze

**Status:** completed
**Priority:** P1
**Effort:** 0.5d

## Model Freeze Invariants (phải enforce trước khi làm bất kỳ phase nào)

Đây là các **bất biến kiến trúc** của toàn plan. Nếu bất kỳ phase nào vi phạm các điều này, phải dừng và review lại.

### 1. Session = single-terminal unit

- **1 session = 1 terminal instance** (không có multi-terminal-per-session nữa)
- Split màn hình = nhiều `session_view` của **nhiều session khác nhau** trong `WorkspaceLayout`
- Không có split nội bộ một session → không tạo thêm terminal child trong cùng session

### 2. Desktop là executor duy nhất

- `chatminal-runtime` chỉ giữ metadata/persistence/compatibility boundary — **không execute sessions**
- `chatminald` (daemon) chỉ giữ session store — **không execute sessions**
- Chỉ `apps/chatminal-desktop` (qua `desktop_host_runtime`) là executor
- Vì vậy: move execution core vào `desktop_host_runtime` là đúng, không phải "bước lùi"

### 3. Legacy flows phải được migrate hoặc xóa

Các flows sau là **di sản multi-terminal model**, phải xử lý trong Phase 02-06:

| Flow | File | Migration path |
|------|------|---------------|
| `split_terminal_instance` | `session_host.rs:275` | **Xóa** — split = tạo session mới trong WorkspaceLayout |
| `focus_direction` | `session_host.rs:226` | **Đổi nghĩa** → `focus_session_in_direction` (workspace op) |
| `swap_active_with_terminal_instance` | `session_host.rs:245` | **Đổi nghĩa** → `swap_session_positions` (workspace op) |
| `move_terminal_instance_*` | `engine_runtime_adapter.rs` | **Đổi nghĩa** → `move_session_to_*` (workspace/window op) |

Khi migrate, thay thế bằng workspace layout operations tương ứng trong `WorkspaceLayoutState`.

### 4. Layer boundary cho data types

- `chatminal-runtime` chỉ chứa pure data (primitive, Serialize/Deserialize)
- **KHÔNG được** thêm `Arc<ChatminalSessionPane>` hay bất kỳ desktop type nào vào `SessionEntry`
- Mapping `session_id → Arc<ChatminalSessionPane>` chỉ sống trong `DesktopSessionHost`

---

## Goal

Lập danh sách đầy đủ mọi call-site dùng `HostRenderScope` và `HostMux.get_window().iter()` ngoài `desktop_host_runtime/`. Đóng băng boundary trước khi refactor.

## Context links

- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` — type aliases, public helpers
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` — `sync_render_state_for_runtime`, `render_scope_for_runtime`
- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs` — `EngineRuntimeAdapter` impl dùng `HostRenderScope` làm trung gian
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` — `desktop_render_scope_id_for_session`, `desktop_render_state_for_session`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs` — consumer của hai hàm trên

## Key insights

Qua grep và đọc code, các call-site **ngoài** `desktop_host_runtime/`:

| Call-site | File | Phase migrate |
|-----------|------|---------------|
| bridge definition `desktop_render_scope_id_for_session` | `chatminal_runtime/mod.rs:830` | Phase 03 — xóa sau khi consumer migrate |
| bridge definition `desktop_render_state_for_session` | `chatminal_runtime/mod.rs:840` | Phase 03 — refactor đọc từ DesktopSessionHost |
| `layout_positioned_panes` / `layout_positioned_splits` | `desktop_termwindow_layout_render.rs:99,119,165,174` | Phase 03 |
| `render_capability_for_layout_split` | `desktop_termwindow_layout_render.rs:202` | Phase 03 — duyệt render_state.splits rồi fallback scope_id; migrate sang workspace splits |
| consumer `desktop_render_scope_id_for_session` | `chatminal_runtime/mod.rs:685` | Phase 03 |
| consumer `desktop_render_state_for_session` | `chatminal_runtime/mod.rs:252` | Phase 03 |
| `positioned_panes_for_session` (dùng scope_id + render_state.panes) | `desktop_termwindow_positioned_session_helpers.rs:4,33` | Phase 03 — migrate sang single pane lookup |
| `positioned_splits_for_session` (dùng scope_id + render_state.splits) | `desktop_termwindow_positioned_session_helpers.rs:4,64` | Phase 03 — migrate sang workspace layout splits |
| `resize_host_window_tabs` | `desktop_host_runtime/mod.rs:400` | Phase 03 (trong desktop_host_runtime, được phép) |
| `activate_host_runtime_entry` | `desktop_host_runtime/mod.rs:858` | Phase 03 (trong desktop_host_runtime, được phép) |
| `host_launcher_tabs` | `desktop_host_runtime/mod.rs:498` | Phase 03 (trong desktop_host_runtime, được phép) |

**⚠️ Ngoài danh sách trên, không được thêm call-site mới — freeze từ đây.**

Call-site **trong** `desktop_host_runtime/` (cần sửa nhưng được phép):

| Hàm | File |
|-----|------|
| `sync_render_state_for_runtime` | `session_host.rs:410` — tạo `Arc<HostRenderScope>` để build `ChatminalRenderState` |
| `render_scope_for_runtime` | `session_host.rs:348` — lookup Tab trong Mux window |
| Toàn bộ `EngineRuntimeAdapter` impl | `engine_runtime_adapter.rs` — mọi method đều dùng `render_scope_id_for_runtime` |

## Freeze decisions

1. `HostRenderScope` chỉ được tham chiếu bên trong `desktop_host_runtime/` — không thêm call-site mới bên ngoài.
2. `desktop_render_scope_id_for_session` sẽ bị xóa ở Phase 03 — ghi comment `// BOUNDARY: remove in Phase 03` thay vì `#[deprecated]` (grep gate đủ, warning không thêm safety thật với internal refactor).
3. `desktop_render_state_for_session` sẽ được refactor ở Phase 03 để đọc từ `DesktopSessionHost` thay vì qua `HostRenderScope`.
4. `pane_for_session` (map mới `session_id → Arc<ChatminalSessionPane>`) sẽ được thêm vào `DesktopSessionHost` ở Phase 02 — **1 session = 1 pane**, không dùng Vec.

## Implementation steps

1. Chạy grep audit để xác nhận danh sách call-site đầy đủ:
   ```
   grep -rn "HostRenderScope\|render_scope_for_runtime\|get_window.*iter\|render_scope_id_for_session" \
     apps/chatminal-desktop/src/ --include="*.rs" | grep -v "desktop_host_runtime/"
   ```
2. Với mỗi call-site ngoài `desktop_host_runtime/`, ghi chú cách thay thế ở Phase 03.
3. Thêm comment `// BOUNDARY: remove in Phase 03` vào `desktop_render_scope_id_for_session` — không dùng `#[deprecated]` (grep gate đủ với internal refactor).
4. Không thay đổi logic, chỉ comment + document.
5. `cargo check --workspace` → phải pass.

## Todo

- [x] Chạy grep audit, ghi kết quả vào comment trong `chatminal_runtime/mod.rs`
- [x] Thêm comment `// BOUNDARY: remove in Phase 03` vào `desktop_render_scope_id_for_session`
- [x] Confirm không có call-site ngoài danh sách trên
- [x] `cargo check --workspace` pass

## Success criteria

- Grep audit không có call-site mới nào bị bỏ sót
- `cargo check --workspace` pass
- Không có logic thay đổi

## Risk

- Nếu có call-site trong overlay hay scripting path chưa tìm thấy → sẽ phát hiện ở bước `cargo check` sau Phase 02/03.
