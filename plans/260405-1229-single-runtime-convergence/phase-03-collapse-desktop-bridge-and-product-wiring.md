---
phase: 03
status: completed
priority: high
effort: medium
risk: medium
---

# Phase 03: Collapse Desktop Bridge And Product Wiring

## Context Links
- [plan.md](./plan.md)
- [execution_bridge.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs)
- [desktop_host_runtime/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/mod.rs)
- [chatminal_runtime/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/mod.rs)

## Overview
- Priority: P1
- Current status: in_progress
- Mục tiêu: biến desktop thành UI shell thuần bằng cách cắt execution bridge/adapters và rewiring product path sang canonical runtime.

## Key Insights
- Nếu phase 2 xong mà desktop vẫn gọi qua bridge cũ, kiến trúc vẫn còn song song về mental model.
- Desktop chỉ nên hold render/input/view-model state; không nên own execution state lần thứ hai.
- Phase này không chỉ xóa `DesktopRuntimeExecutionBridge`; còn phải gỡ luôn `RuntimeHost`-based desktop facade như `desktop_runtime_host()` và các alias `SessionEngineShared` / `DesktopSessionHost`.

## Requirements
- Desktop app không còn cần `DesktopRuntimeExecutionBridge` để spawn/activate session.
- Desktop app không còn cần `RuntimeHost` facade để focus/hydrate/resize/fetch terminal binding.
- `desktop_host_runtime` chỉ còn UI-host/render glue và pane presentation adapters thật sự cần cho termwindow.
- `chatminal_runtime/mod.rs` re-export/wrappers được thu gọn theo canonical runtime API mới.
- Không được thay đổi UI hierarchy, styling, overlay positions, sidebar behaviors, hay menu/context actions hiện có.
- Nếu sau cutover module name `desktop_host_runtime` không còn phản ánh ownership thật, phase này phải re-home hoặc rename namespace để không giữ lại vocabulary sai.

## Architecture
- Runtime client/surface ở app layer gọi thẳng canonical runtime APIs.
- Desktop subscribe runtime events + render snapshots; không own duplicate execution registry.
- Session pane adapters chỉ map UI terminal surface tới runtime-owned terminal instances.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- Modify: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Modify: `apps/chatminal-desktop/src/main.rs`
- Modify/Delete: `crates/chatminal-runtime/src/runtime_host.rs` callers trong desktop nếu còn
- Delete: bridge/helper wrappers không còn caller hoặc ownership thật

## Implementation Steps
1. Rewire desktop bootstrap tới canonical runtime owner.
2. Xóa `DesktopRuntimeExecutionBridge`, `desktop_runtime_host()`, và bridge traits/structs khỏi app layer.
3. Thu gọn `chatminal_runtime/mod.rs` để không leak `SessionEngineShared`, `DesktopSessionHost`, `RuntimeHost`.
4. Thay bootstrap/shutdown ở `main.rs` và desktop init path sang canonical runtime boot, không qua host runtime root.
5. Giữ lại pane presentation adapters tối thiểu cho render/input.
6. Gỡ duplicate runtime/attachment lookup state ở desktop nếu còn.
7. Re-home/rename module namespaces gây hiểu sai ownership nếu còn.
8. Verify startup, switch session, join layout, right-click actions, restore, shutdown.

## Todo List
- [x] Desktop bootstrap gọi canonical runtime path — main.rs → desktop_host_runtime module → chatminal-runtime; no chatminal-host-runtime crate in path
- [x] Xóa `DesktopRuntimeExecutionBridge`
- [x] Xóa `desktop_runtime_host()` / `RuntimeHost` desktop facade
- [x] Thu gọn `chatminal_runtime/mod.rs` wrappers
- [x] Xóa host-runtime bootstrap khỏi `main.rs` / desktop init path — chatminal-host-runtime crate removed from Cargo.toml; bootstrap functions now desktop-local
- [x] Cắt duplicate attachment/session-engine state ở desktop — session_engine/mod.rs is now 7-line re-export from chatminal_runtime::execution; old engine files deleted
- [x] Dọn namespace/module names misleading về ownership — `desktop_host_runtime` → `desktop_session_host`; `desktop_termwindow_host_runtime_helpers.rs` → `desktop_termwindow_session_host_helpers.rs`
- [x] Chạy smoke tests desktop product path

## Phase 03 Progress Notes
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` đã nội địa hóa public spawn/split vocabulary qua `HostSpawnTargetHandle`, `HostSpawnedRuntimeEntry`, và `HostSplitSource`; caller desktop không còn lộ trực tiếp `host_runtime::spawn_target::{SpawnTarget, SplitSource}`.
- `apps/chatminal-desktop/src/desktop_host_runtime/window.rs` đã chuyển từ giữ `Arc<host_runtime::tab::Tab>` sang local `WindowRuntimeEntry { runtime_id, title }`, giúp desktop-local window registry không còn own host `Tab` object.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` đã dọn nhiều wrapper/fallback thừa và chuyển sync window metadata sang local registry; `launcher_sessions()` và active window metadata hiện ưu tiên state local thay vì giữ `Tab` object sống trong desktop window model.
- `apps/chatminal-desktop/src/main.rs` startup path đã ưu tiên activate theo `chatminal_session_id` metadata của pane vừa spawn; chỉ fallback qua `activate_host_runtime_entry()` khi pane không phải runtime-owned session.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` đã cắt thêm một lớp control-plane fallback lệch trạng thái: resolve runtime-owned session path giờ ưu tiên desktop-local window/runtime registry; focus vẫn còn sync sang `host_runtime::focus_root_runtime_entry(...)` như seam compat cho split/resize legacy, nhưng source-of-truth cho activation của session render targets đã nghiêng về local registry.
- `apps/chatminal-desktop/src/desktop_host_runtime/lua_bridge_backend.rs` đã dùng chung helper parse session metadata với desktop facade, bỏ duplicate parse logic.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` đã cắt sync `workspace/title/client` metadata sang `host_runtime` cho phase-03 slice hiện tại:
  - `host_set_workspace_name(...)` và `host_active_identity()` giờ ưu tiên local `HOST_REGISTRY` state thay vì ghi/đọc shadow metadata từ `host_runtime`
  - `initialize_desktop_host_runtime(...)` sau bootstrap sẽ seed workspace vào local `DesktopSessionHost` window/client registry, không còn đẩy `root_window_workspace_name` sang host layer làm source of truth thứ hai
  - `set_active_workspace_for_client(...)` giữ invariant mới: active client đổi workspace thì local root window workspace đổi cùng lúc
  - test seam `build_host_runtime_for_test(...)` đã được chỉnh để khởi tạo local `DesktopSessionHost` như product path thật, tránh phụ thuộc fallback metadata từ host layer
  - `host_terminal_handle(...)` và runtime-entry title sync giờ đọc/ghi local pane/window registry trước; desktop không còn mirror title metadata xuống `host_runtime` chỉ để giữ shadow state
- `apps/chatminal-desktop/src/desktop_spawn.rs`, [`desktop_host_runtime/mod.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/mod.rs), và [`session_host.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs) đã cắt thêm một seam host-flavored ở API shape:
  - `spawn_host_runtime_entry(...)` đã được đổi thành `spawn_desktop_terminal(...)`
  - tham số legacy `position` bị bỏ qua đã bị xóa khỏi boundary này
  - current sibling terminal handle giờ đi qua typed `SessionTerminalHandle` thay vì raw `u64`
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`, [`session_host.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs), và [`spawn_target.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs) đã co seam spawn-boundary xuống desktop-local trait:
  - `HostSpawnTargetHandle` hiện wrap `DesktopSpawnTargetBackend` thay vì lộ trực tiếp `host_runtime::spawn_target::SpawnTarget`
  - `legacy_host_runtime_spawn_target(...)` là adapter một chiều duy nhất quay về host_runtime substrate
  - `DesktopSessionHost` giữ primary spawn target local làm source of truth tại desktop boundary
- `apps/chatminal-desktop/src/main.rs` đã cắt thêm host-flavored startup plumbing khỏi entrypoint:
  - `async_run_terminal_gui(...)` không còn nhận `Option<HostSpawnTargetHandle>`; thay bằng `force_startup_spawn: bool`
  - serial startup path giờ gọi `install_serial_spawn_target(...)` thay vì tự cầm `HostSpawnTargetHandle`
  - startup spawn path đi qua `spawn_in_primary_target(...)`, nên `main.rs` không còn tự gọi `.spawn()` trên host-flavored handle
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`, `session_host.rs`, và `spawn_target.rs` đã cắt thêm một lớp legacy vocabulary khỏi public desktop boundary:
  - `HostSpawnTargetHandle` giờ wrap desktop-local trait `DesktopSpawnTargetBackend` thay vì lộ trực tiếp `host_runtime::spawn_target::SpawnTarget`
  - `DesktopSpawnTarget` đã implement desktop-local backend contract; adapter sang `host_runtime::spawn_target::SpawnTarget` chỉ còn là seam mỏng tại boundary cần compat
  - `session_host.rs` không còn phải kéo host spawn-target trait trực tiếp vào public flow; legacy bridge chỉ còn nằm ở `initialize_host_runtime_with_config` / `set_primary_spawn_target`
  - kết quả: vocabulary spawn của desktop gọn hơn, bề mặt cắt phase-04 nhỏ hơn trước dù Cargo graph chưa đổi
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`, [`session_host.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs), và [`spawn_target.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs) đã khép thêm seam spawn backend:
  - `HostSpawnTargetHandle` hiện wrap desktop-local trait `DesktopSpawnTargetBackend` thay vì lộ trực tiếp trait object `host_runtime::spawn_target::SpawnTarget`
  - bridge sang `host_runtime` đã bị co lại thành adapter một chiều `legacy_host_runtime_spawn_target(...)`
  - `DesktopSessionHost` giữ `primary_spawn_target` local làm source of truth ở desktop boundary; `host_runtime` chỉ còn là execution substrate phía sau chứ không còn là vocabulary công khai của handle này
- `apps/chatminal-desktop/src/termwindow/mod.rs`, [`desktop_host_runtime/mod.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/mod.rs), và [`session_host.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs) đã cắt thêm dead fallback branch của product path hiện tại:
  - `TermWindow::active_render_scope_id()` giờ đi thẳng theo active session render target; không còn fallback sang `host_active_render_scope_id()`
  - bootstrap size/resize trong `new_primary_window()` không còn giữ nhánh `sidebar disabled`, vì product invariant hiện tại là sidebar luôn bật
  - wrapper `resize_host_window_tabs()` / `active_host_runtime_entry_size()` và local method tương ứng đã bị xóa, giảm thêm một lớp call-through không còn giá trị kiến trúc
- `apps/chatminal-desktop/src/termwindow/mod.rs`, `desktop_host_runtime/mod.rs`, và `session_host.rs` đã cắt thêm nhánh desktop fallback không còn dùng khi sidebar là invariant-on:
  - `TermWindow::active_render_scope_id()` giờ chỉ resolve qua active session render target, không fallback `host_active_render_scope_id()`
  - bootstrap `new_primary_window()` không còn lấy size/resize initial window qua `active_host_runtime_entry_size()` và `resize_host_window_tabs()`
  - các wrapper `host_active_render_scope_id`, `active_host_runtime_entry_size`, `resize_host_window_tabs`, cùng helper `host_resize_all_tabs` đã bị bỏ khỏi desktop facade/session host
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`, `frontend.rs`, `termwindow/mod.rs`, và `chatminal_layout/workspace_store.rs` đã thu gọn thêm wrapper layer ở app boundary:
  - `FrontendClientHandle`, `HostActivityGuard`, `PrimaryHostWindowId`, và `RuntimeNotification` không còn được re-export vòng qua `chatminal_runtime/mod.rs` cho caller UI; `frontend` và `termwindow` giờ import trực tiếp từ `desktop_host_runtime`
  - `SessionEngineShared` ở `DesktopWorkspaceLayoutStore` đã dùng trực tiếp từ `chatminal_runtime::execution`, giảm thêm một alias product-local không cần thiết
  - kết quả: `chatminal_runtime/mod.rs` bớt vai trò façade cho desktop-only types và gần hơn với scope runtime/workspace canonical
- alias-cleanup chunk mới nhất đã khép và được verify:
  - caller UI (`frontend`, `termwindow`) không còn import desktop-only types qua `chatminal_runtime/mod.rs`
  - `SessionEngineShared` trong `workspace_store` đã dùng trực tiếp từ `chatminal_runtime::execution`
  - verify pass:
    - `cargo check -p chatminal-desktop -p chatminal-runtime -p chatminal-host-runtime`
    - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml chatminal_layout::workspace_store -- --test-threads=1`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` đã cắt thêm nhóm fallback local-vs-host không còn justified:
  - resolve pane/runtime/focused-pane/public-pane cho desktop-owned sessions giờ đọc local `DesktopSessionHost` registry trước và không còn fallback xuống `host_runtime` ở các path chính
  - `runtime_entry_terminal_handle_in_direction_by_session_id(...)`, `set_runtime_entry_active_terminal(...)`, và `overlay_pane_layouts_by_id(...)` giờ đi theo render/pane state local của desktop session host
  - `sync_runtime_window_entry(...)` là path local hiện tại cho window/title/focus sync; tab-shim history không còn là source-of-truth cho desktop session render targets
- Verify cho chunk này:
  - `cargo check -p chatminal-desktop -p chatminal-runtime -p chatminal-host-runtime` pass
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml desktop_host_runtime::session_host -- --test-threads=1` pass (`8/8`)
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` vẫn còn fail môi trường linker (`ld: write() failed, errno=28`), chưa phải regression code từ chunk này
- follow-up fix sau review:
  - `DesktopSpawnTarget::spawn()` đã quay lại local non-session path `spawn_pane + resolve_runtime_id_for_terminal_handle`, tránh double-wrap `Tab` ở legacy host-runtime spawn flow
  - `sync_render_state_for_runtime(...)` không còn ép mọi runtime về single-pane; runtime nhiều leaf giờ mirror terminal/split infos từ `host_runtime` để tránh mismatch source-of-truth cho split/zoom/resize paths
  - `host_focus_root_window_tab(...)` chỉ báo success khi local window focus và legacy host focus cùng đồng thuận ở các runtime-entry còn tồn tại trong `host_runtime`, giảm rủi ro lệch active target
  - `host_remove_tab(...)` vẫn dọn cả local window registry lẫn `host_runtime` runtime-entry để tránh stale entry còn sống trong legacy substrate
- Verify incremental sau các lát cắt này:
  - `cargo check -p chatminal-desktop -p chatminal-runtime -p chatminal-host-runtime`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml desktop_host_runtime::session_host -- --test-threads=1`
- Verify mới nhất cho chunk local-only + spawn backend:
  - `cargo check -p chatminal-desktop -p chatminal-runtime -p chatminal-host-runtime` pass
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml desktop_host_runtime::session_host -- --test-threads=1` pass (`8/8`)
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` hiện fail ở linker môi trường (`ld: write() failed, errno=28`), chưa xuất hiện failing test/assertion gắn với chunk refactor này
- Review/tetser cho chunk này đều xác nhận không có finding critical; residual lớn nhất còn lại là startup/attach/focus path và spawn substrate vẫn delegate sang `host_runtime`.

## Success Criteria
- Product path không còn compile-time dependency vào execution bridge layer cũ.
- Product path không còn compile-time dependency vào `RuntimeHost` desktop facade cũ.
- Desktop không own execution registry song song.
- UI behavior giữ nguyên.
- Không có visual regression chủ ý; mọi thay đổi chỉ được chấp nhận nếu là bug fix bắt buộc do seam cutover.
- Desktop module tree không còn giữ naming khiến người đọc hiểu nhầm nó vẫn là execution owner.

## Risk Assessment
- Có thể gãy startup/session attach nếu pane presentation adapters vẫn đang kéo execution assumptions cũ.
- Dễ còn dead wrappers nếu không grep toàn repo sau cutover.

## Security Considerations
- Giữ nguyên input routing guard và session focus checks.
- Không để UI path có thể write vào wrong terminal instance sau cutover.

## Next Steps
- Phase 04 bắt đầu khi product desktop path đã đứng trên canonical runtime owner duy nhất.
