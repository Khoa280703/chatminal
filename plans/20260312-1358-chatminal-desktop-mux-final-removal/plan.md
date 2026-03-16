# Chatminal Desktop Mux Final Removal

Status: completed
Goal: cắt bỏ hoàn toàn `mux/tab/pane/leaf` khỏi desktop active path, session-runtime active path, và workspace compile graph liên quan, để Chatminal chỉ còn product/runtime/render model first-party: `session + session_view + workspace_layout + terminal_instance`.

## Why New Plan
- Các plan trước hoàn tất `product cutover` và `execution cutover`, nhưng cố ý giữ `mux` compatibility ở render/overlay/bootstrap.
- Vì vậy hiện trạng là `Chatminal product model` chạy trên `mux compatibility render layer`, chưa phải clean cut.
- Plan này chỉ thành công khi desktop compile/runtime path không còn phụ thuộc `mux`.

## Phases
- Phase 01 - Freeze Final Boundary And Rename Target Vocabulary
- Phase 02 - Session Runtime Detach From Mux Types
- Phase 03 - Desktop Render Model Bootstrap
- Phase 04 - TermWindow And Action Routing Cutover
- Phase 05 - Overlay Search Copy Launcher Cutover
- Phase 06 - Desktop Bootstrap Frontend And Event Loop Cutover
- Phase 07 - Dependency Prune Delete And Final Verification

## Progress
- Phase 01: completed
- Phase 02: completed
- Phase 03: completed
- Phase 04: completed
- Phase 05: completed
- Phase 06: completed
- Phase 07: completed

## Target Architecture
- `chatminal-runtime`: profile/session/history/store/workspace persistence.
- `chatminal-session-runtime`: live session graph first-party only; không expose `mux::Tab`, `mux::Pane`, `TabId`, `PaneId`.
- `apps/chatminal-desktop/src/chatminal_render/*`: render tree, terminal instance handle, overlay host, window event bridge.
- `termwindow`: chỉ consume `ChatminalRenderTree` + `DesktopSessionHost`; không gọi `Mux::get()`.
- `overlay/*`: chỉ nhận `TerminalInstanceId` hoặc `SessionViewId`; không nhận `Tab`/`Pane`.

## Product Rules
- `session` là một runtime/process độc lập.
- Một `session` không được split nội bộ.
- Chia ngang/dọc là hành vi của `workspace_layout`, không phải hành vi bên trong một session.
- Một ô hiển thị chỉ attach vào đúng một session tại một thời điểm.
- Có thể tạo session mới, clone session hiện tại thành session mới, rồi gộp nhiều session cùng hiển thị trong một layout.
- Product model mục tiêu là `session + session_view + workspace_layout + terminal_instance`, không quay lại `tab -> pane -> leaf`.

## Hard Invariants
- Không đụng `third_party/`.
- Không xoá code UI user đang sửa; chỉ thay runtime/render plumbing.
- Không giữ lại `mux` trong desktop compile graph sau Phase 07.
- Không giữ public/app-facing vocabulary `tab`, `pane`, `leaf`; internal runtime còn cần thì phải rename sang khái niệm first-party.
- Không dùng type alias/wrapper chỉ để đổi tên nhưng vẫn lộ `mux` type ở app path.
- Mỗi phase phải có grep gate + build/test gate trước khi sang phase sau.

## Completion Gates
- `rg -n --glob '!third_party/**' --glob '!vendor/**' --glob '!frontend/node_modules/**' "use mux::|mux::|\\bTabId\\b|\\bPaneId\\b|Arc<Tab>|Arc<dyn Pane>|impl Pane for|PositionedPane|PositionedSplit|host_tab|host_pane|get_tab\\(|get_pane\\(" apps/chatminal-desktop/src crates/chatminal-session-runtime/src`
  - expected: zero dòng active code dùng type/API mux cũ; không match key name `Tab` của bàn phím hay text UI chung chung.
- `rg -n --glob '!third_party/**' --glob '!vendor/**' --glob '!frontend/node_modules/**' "LeafId|leaf_id|leaf-|session_surface|surface_id|SurfaceInfo|SpawnSurface|surface_" apps/chatminal-desktop/src crates/chatminal-session-runtime/src crates/chatminal-runtime/src`
  - expected: zero residual product/runtime naming cũ; không match các graphics/backend surface như `wgpu::Surface` hay `termwiz::surface`.
- `rg -n "mux = |chatminal-mux" Cargo.toml apps/chatminal-desktop/Cargo.toml crates/chatminal-session-runtime/Cargo.toml`
  - expected: desktop và session-runtime không còn phụ thuộc `mux`; workspace không còn member `crates/chatminal-mux`.
- `cargo check --workspace`
- `cargo test -p chatminal-session-runtime -- --test-threads=1`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`

## Final Verification
- `cargo check -p chatminal-desktop`: pass
- `cargo check --workspace`: pass
- `cargo test -p chatminal-session-runtime -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`: pass
- Desktop/session-runtime grep gate for `mux::|TabId|PaneId|PositionedPane|PositionedSplit|get_tab(|get_pane(`: zero match
- Residual naming grep gate for `leaf-|leaf_id|session_surface|surface_id|SurfaceInfo|SpawnSurface`: zero match in scoped runtime files
- Manifest grep gate for `mux = |chatminal-mux`: zero match in active manifests

## Done When
- Desktop active path không còn `Mux::get()` hoặc `mux::*`.
- Session runtime active path không còn bridge kiểu `host tab/pane`.
- `termwindow`, `overlay`, `frontend`, `main` chỉ nói bằng model first-party.
- `chatminal-mux` không còn là workspace member và không còn downstream trong build graph của repo active.
