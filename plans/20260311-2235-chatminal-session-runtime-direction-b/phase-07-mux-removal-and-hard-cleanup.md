# Phase 07 - Mux Removal And Hard Cleanup

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/Cargo.toml
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/bin/chatminal-mux/main.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/frontend.rs

## Overview
- Priority: P1
- Status: completed
- Brief: xóa public mux surface còn sót, dọn dependency graph và naming cũ

## Key Insights
- Chỉ được làm phase này khi desktop shell và runtime đã nói hoàn toàn bằng session graph mới
- Nếu còn callsite tab/pane public mà xóa sớm sẽ tạo hệ split-brain mới
- Lát cắt an toàn đầu tiên của phase này là đổi public scripting/window API sang naming trung tính trước, rồi mới đụng binary/dependency graph

## Requirements
- Functional: remove direct app-level mux usage, remove mux-specific binaries/deps nếu không còn cần
- Non-functional: cleanup phải đo được bằng search/callsite count, không cảm tính

## Architecture
- `mux` nếu còn chỉ được phép nằm trong engine adapter private layer hoặc bị thay hẳn
- App/public runtime không expose `MuxWindowId`, `TabId`, `PaneId` làm model kiến trúc nữa

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/Cargo.toml
- Delete: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/bin/chatminal-mux/main.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/frontend.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/scripting/guiwin.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/scripting/mod.rs

## Implementation Steps
1. Đếm và xóa callsites `mux::*` ở app layer
2. Bỏ binary/dependency không còn dùng
3. Rename type/model còn lộ `Tab/Pane/MuxWindow` ở app layer
4. Chạy full regression gates

## Todo List
- [x] Remove direct mux dependencies from app layer
- [x] Delete mux-specific binary when dead
- [x] Rename public models to session/surface/layout
- [x] Run regression + cleanup review

## Current Progress
- `chatminal.gui.gui_window_for_mux_window` đã được thay bằng `chatminal.gui.gui_window_for_window_id`
- `GuiWin` public id đã đổi sang `window_id: u64`
- `GuiWin` đã expose `active_session_id()` và `active_surface_id()` để scripting đi theo session model mới
- `chatminal-mux-lua` đã được nối sang API mới để không giữ tên public cũ ở bridge desktop path
- `frontend -> termwindow` notification/binding path đã đổi sang `DesktopWindowId`
- `TabInformation.window_id` đã đổi sang `DesktopWindowId`
- `PaneInformation.pane_id` đã được neutralize thành `u64` và thêm `leaf_id`
- `GuiWin` không còn expose `engine_window()` hay `active_tab()`
- `GuiWin` selection/action helpers đã chuyển sang `pane_id: u64`; public scripting surface không còn trả `MuxPane/MuxTab/MuxWindow`
- `UnixDomain::serve_command()` đã chuyển default compatibility host sang `chatminald`
- `apps/chatminal-desktop` đã bỏ `[[bin]] chatminal-mux`; wrapper binary `src/bin/chatminal-mux/*` đã bị xóa khỏi desktop package
- `chatminal_session_surface.rs` và `spawn.rs` public/helper boundary đã nhận `DesktopWindowId`; việc đổi sang `MuxWindowId` chỉ còn ở private conversion trong file
- `termwindow` session-mode callsites chính đã đổi sang `DesktopWindowId` khi đi qua session surface helpers
- `overlay::start_overlay` và `start_overlay_pane` đã neutralize callback id sang `u64`; confirm-close overlay path không còn buộc callsite app-facing giữ `TabId`/`PaneId`
- `TabInformation` / `PaneInformation` public model đã gom phần lớn `Mux` lookup vào helper nội bộ thay vì lặp conversion trực tiếp ở từng field getter
- `GuiWin` không còn gọi `Mux` trực tiếp cho pane selection/action/escapes path; `active_workspace` đã chuyển thành snapshot field từ `TermWindow`
- overlay prompt/confirm/selector và Lua event trampoline không còn expose `MuxPane`; callback công khai đã chuyển sang `pane_id: u64`
- `crates/chatminal-lua-bridge` đã được dọn tiếp theo public vocabulary mới:
  - đổi helper `tab_session_id/tab_surface_id/tab_active_leaf_id` sang `surface_*`
  - đổi internal public-facing structs `SpawnTab/TabInfo/PaneInfo` sang `SpawnSurface/SurfaceInfo/LeafInfo`
  - giữ alias compatibility cũ cho script/config hiện tại
- `SurfaceRef` / `LeafRef` public diagnostics đã đổi sang wording `surface host tab` và `leaf host pane`, giảm lộ public `tab/pane` trong log lỗi và `ToString`
- `TermWindowNotif` selection/action routing đã đổi sang `PerformAssignmentForLeafId`, `GetSelectionForLeafId`, `GetSelectionEscapesForLeafId`, `CancelOverlayForLeafId`
- `chatminal_session_surface.rs` helper app-facing đã đổi naming:
  - `session_id_for_tab` -> `session_id_for_host_surface`
  - `session_surface_tab` -> `host_surface_for_session`
  - `host_tab_for_surface` -> `host_surface_for_public_surface`
  - `move_session_leaf_to_new_tab` -> `move_session_leaf_to_new_surface`
- Residual mux ids còn lại tập trung chủ yếu ở `termwindow` private engine core, engine adapter, và runtime internals; không còn là blocker của app/public architecture

## Success Criteria
- Search app layer không còn direct mux public callsites
- Session architecture là public source of truth duy nhất
- Code review pass, test gates pass
- Cleanup được chứng minh bằng grep/build/test thay vì cảm giác

## Completion Notes
- `chatminal-mux` wrapper binary đã bị loại khỏi desktop package và default compatibility serve path đã chuyển sang `chatminald`
- Public scripting/window/session overlay surface không còn expose `MuxWindow`/`MuxTab`/`MuxPane`
- `MuxWindowId`/`TabId`/`PaneId` còn sót chỉ nằm trong private engine core hoặc adapter, không còn là public contract của Chatminal
- Validation pass:
  - `cargo test -p chatminal-config`
  - `cargo test -p chatminal-runtime`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml`
  - `cargo check --workspace`
  - `cargo check -p chatminal-lua-bridge`
  - `cargo check --manifest-path apps/chatminal-desktop/Cargo.toml`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml`

## Risk Assessment
- Risk: cleanup xóa nhầm engine-private path còn cần
- Mitigation: chỉ xóa sau khi phase trước đã cutover hoàn toàn và có grep proof

## Security Considerations
- Cleanup không được làm mất process shutdown/close safety
- Phải giữ lifecycle cleanup rõ ràng cho runtime handles

## Validation Gates
- `cargo check --workspace`
- `cargo test -p chatminal-runtime`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml`
- `rg -n "mux::|Mux::|MuxWindowId|TabId|PaneId" apps/chatminal-desktop/src crates/chatminal-runtime/src`
  - expected: zero results ở app/public runtime layer
- `rg -n "mux::|Mux::" crates/chatminal-session-runtime/src --glob '!engine_surface_adapter.rs'`
  - expected: zero results ngoài private adapter; nếu adapter cũng biến mất thì grep toàn crate phải về zero

## Next Steps
- Sau phase này nếu muốn đi sâu hơn nữa thì target kế tiếp không còn là "mux public cleanup" mà là refactor private engine core/backend layer
