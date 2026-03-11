# Chatminal Session Runtime Direction B

Status: Completed
Goal: Thay runtime graph tab-centric bằng session-centric architecture, cô lập rồi loại bỏ mux public surface khỏi app layer.

## Phases
- Phase 01 - Target Architecture And Boundary Freeze: define target modules, ownership, compatibility boundary
- Phase 02 - Desktop Session Surface Facade: cô lập desktop shell khỏi mux tab trực tiếp
- Phase 03 - Chatminal Session Runtime Crate Bootstrap: tạo live graph crate mới và adapter tạm
- Phase 04 - Runtime Bridge And Session Event Bus: nối business runtime với live session graph
- Phase 05 - Session Layout Focus And Spawn Cutover: chuyển split/focus/spawn/close sang session graph mới
- Phase 06 - Desktop Renderer Input And Overlay Cutover: desktop chỉ bind session surface/layout, bỏ tab-centric UI path
- Phase 07 - Mux Removal And Hard Cleanup: xóa dependency/callsite mux public còn sót

## Progress
- Phase 01: completed
- Phase 02: completed
- Phase 03: completed
- Phase 04: completed
- Phase 05: completed
- Phase 06: completed
- Phase 07: completed

## Phase 07 Checkpoint
- compatibility unix-domain default server không còn spawn `chatminal-mux`; đã chuyển sang `chatminald`
- desktop package không còn ship `chatminal-mux` wrapper binary
- public scripting/window identity giữ `DesktopWindowId`
- session-surface helper path và spawn helper path đã neutralize `MuxWindowId` ở app-facing boundary
- overlay/Lua callback path không còn expose `MuxPane`; callback công khai đã chuyển sang `pane_id: u64`
- `chatminal-lua-bridge` public naming đã được dọn thêm theo `session/surface/leaf`:
  - helper `tab_*` -> `surface_*`
  - `SpawnTab` -> `SpawnSurface`
  - `TabInfo` -> `SurfaceInfo`
  - `PaneInfo` -> `LeafInfo`
  - `ToString`/error wording đã chuyển sang `surface/leaf-first`
- desktop notification path cho selection/action/overlay cancel đã đổi từ `...PaneId` sang `...LeafId`
- desktop session-surface helper naming đã đổi từ `...tab...` sang `...surface...` ở app-facing path
- residual mux exposure hiện còn chủ yếu ở private `termwindow` engine state, engine adapter, và runtime internals

## Key Dependencies
- `crates/chatminal-runtime` tiếp tục giữ profile/session/store ownership
- `apps/chatminal-desktop` là consumer đầu tiên của session graph mới
- `mux` chỉ được phép sống trong `apps/chatminal-desktop/src/chatminal_session_surface.rs` ở Phase 02; từ Phase 03 trở đi chỉ được phép sống trong `EngineSurfaceAdapter`
- Không đụng terminal parser/render core trong migration này
- `chatminal-runtime` là source of truth cho `session_id` đang active ở mức business/workspace
- `chatminal-session-runtime` là source of truth cho `surface_id/leaf_id/layout_node_id` và focus nội bộ trong một session surface

## Non-Goals
- Không redesign visual UI trong plan này
- Không đổi persistence model profile/session nếu chưa cần
- Không xóa OS desktop window; chỉ hạ `window` khỏi business model

## Done When
- App layer không còn API tab-centric public path
- Session bar/sidebar dùng `session_id` làm public identity; render/input/overlay route bằng `session_id + surface_id + leaf_id`
- Mux không còn là kiến trúc public của Chatminal
- Mỗi phase đều có grep/build/test gate đo được trước khi sang phase kế tiếp
