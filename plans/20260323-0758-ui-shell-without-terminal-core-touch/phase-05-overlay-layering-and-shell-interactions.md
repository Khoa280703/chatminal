# Phase 05 - Overlay Layering And Shell Interactions

## Context Links
- `apps/chatminal-desktop/src/overlay/mod.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `apps/chatminal-desktop/src/overlay/prompt.rs`
- `apps/chatminal-desktop/src/overlay/confirm.rs`
- `apps/chatminal-desktop/src/overlay/confirm_close_pane.rs`
- `apps/chatminal-desktop/src/desktop_overlay_actions.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`

## Overview
- Priority: P1
- Current status: pending
- Brief: làm overlay shell thống nhất về anchor, z-order, cancel behavior, click/focus capture và visual framing, nhưng không thay đổi overlay terminal internals.

## Key Insights
- `overlay/mod.rs` đã có `start_overlay` và cancel scheduling seam tốt.
- `desktop_termwindow_host_runtime_helpers.rs` đang gắn overlay vào render scope/runtime UI state.
- Overlay rất dễ va vào layout/chrome nếu không có shell slot contract rõ từ Phase 02.

## Mục tiêu
- Làm overlay feel như một phần của shell, không như lớp vá tạm.
- Giảm xung đột giữa overlay, sidebar, split resize, session switching.
- Giữ behavior launch/cancel/focus nhất quán.

## Phạm vi
- Overlay frame, margins, background affordance, anchor bounds.
- Escape/cancel/click outside semantics ở shell layer.
- Layer ordering giữa overlay, sidebar tree, session bar, footer, scrollbar, split handles.

## Files Likely Touched
- `apps/chatminal-desktop/src/overlay/mod.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `apps/chatminal-desktop/src/overlay/prompt.rs`
- `apps/chatminal-desktop/src/overlay/confirm.rs`
- `apps/chatminal-desktop/src/overlay/confirm_close_pane.rs`
- `apps/chatminal-desktop/src/desktop_overlay_actions.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`

## Requirements
- Không đổi overlay terminal allocation contract.
- Không đổi action semantics của launcher/copy/confirm, chỉ đổi shell presentation/routing.
- Overlay phải tôn trọng content bounds và sidebar-enabled mode.

## Architecture
- `overlay/*` giữ content-specific UI logic.
- `overlay/mod.rs` giữ spawn/start boundary.
- `desktop_termwindow_host_runtime_helpers.rs` giữ render-scope attachment.
- Mouse/key dismissal logic vẫn ở shell layer, không cấy vào core runtime.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/overlay/mod.rs`
- Modify: `apps/chatminal-desktop/src/overlay/launcher.rs`
- Modify: `apps/chatminal-desktop/src/overlay/prompt.rs`
- Modify: `apps/chatminal-desktop/src/overlay/confirm.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- Create: none
- Delete: none

## Implementation Steps
1. Inventory overlay types và mapping render-scope hiện tại.
2. Chuẩn hóa overlay frame/margin/z-index theo shell slots.
3. Rà click outside, escape, focus transfer, session switch behavior.
4. Tinh visual dim/background/edge treatment mà không làm mờ terminal quá tay.
5. Verify launcher/prompt/confirm trong mode sidebar on/off và split layout.

## Todo List
- [ ] Overlay anchor đúng bounds
- [ ] Cancel semantics nhất quán
- [ ] Z-order không đè sai sidebar/footer
- [ ] Launcher/prompt/confirm trông cùng một họ UI

## Success Criteria
- Overlay mở/đóng mượt, không “kẹt” focus.
- Không có vùng click chết giữa overlay và shell chrome.
- Session switch hoặc close action khi overlay đang mở có behavior dễ đoán.

## Risk Assessment
- Risk: overlay dismissal logic chạm nhiều input path dễ sinh regression.
- Mitigation: giới hạn thay đổi ở shell routing/helpers, không đụng overlay terminal internals.

## Security Considerations
- Confirm overlay không được yếu đi; destructive actions vẫn cần explicit confirmation.
- Không log hoặc expose thêm dữ liệu clipboard/input nhạy cảm.

## Không Đụng Terminal Core
- No-touch tuyệt đối: `crates/chatminal-terminal-core/**`
- No-touch tuyệt đối: `crates/chatminal-runtime/src/state/**`
- No-touch tuyệt đối: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/**`
- Không đổi overlay terminal type, parser, pane runtime internals

## Next Steps
- Phase 06 chỉ thêm motion nhẹ sau khi overlay states và focus behavior đã ổn định.
