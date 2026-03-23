# Phase 06 - Icon Motion And Micro Feedback

## Context Links
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/colorease.rs`

## Overview
- Priority: P2
- Current status: pending
- Brief: thêm motion và feedback rất nhẹ cho icon/state changes của shell UI để desktop feel tinh hơn, nhưng tuyệt đối không đụng cadence của terminal rendering core.

## Key Insights
- `desktop_termwindow_render_pane.rs` đã có vài hook animation scheduling hiện hữu.
- `tabbar.rs` và sidebar render đang có nhiều icon/text state tĩnh, phù hợp để thêm micro-feedback nhẹ.
- Đây là phase dễ lạm phát scope; phải giữ motion low-cost, event-driven, idle-friendly.

## Mục tiêu
- Thêm hover/active transitions nhẹ cho icon/button/session chips.
- Tạo feedback tinh tế cho expand/collapse, new-session CTA, overlay open/close affordance.
- Giữ CPU/GPU cost thấp, không có animation loop vô hạn khi idle.

## Phạm vi
- Opacity/color/position easing nhẹ ở shell UI.
- Icon state transitions cho sidebar actions, session bar indicators, footer ambient hints nếu hợp lý.
- Không làm motion ở glyph stream của terminal content.

## Files Likely Touched
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/colorease.rs`

## Requirements
- Motion phải event-driven, duration ngắn, không làm drop responsiveness.
- Không được thay đổi cadence render của pane content.
- Có fallback sạch khi animation disabled hoặc state không hỗ trợ.

## Architecture
- Scheduling tận dụng hook animation có sẵn ở shell render layer.
- Motion state giữ cục bộ ở UI surface; không thêm dependency vào runtime/store.
- Color easing/transition logic đi qua helper nhẹ, không dựng subsystem animation mới.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/tabbar.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- Modify: `apps/chatminal-desktop/src/colorease.rs`
- Create: none
- Delete: none

## Implementation Steps
1. Chọn tối đa 3-4 interaction points thật đáng giá để animate.
2. Gắn easing ngắn cho hover/active/expand/open-close states.
3. Ràng buộc animation lifecycle với invalidate/frame scheduling hiện có.
4. Kiểm tra idle path để bảo đảm không giữ animation pending vô hạn.
5. Smoke test CPU/GPU trên sidebar-heavy và overlay-heavy path.

## Todo List
- [ ] Motion chỉ xuất hiện ở shell UI
- [ ] Không có loop animation vô hạn
- [ ] Hover/active feedback rõ nhưng không lòe loẹt
- [ ] Performance khi idle giữ nguyên

## Success Criteria
- UI có cảm giác sống hơn nhưng vẫn “terminal-first”.
- Không có regression về input latency hoặc pane redraw cadence.
- Motion đủ nhẹ để không gây distraction khi làm việc lâu.

## Risk Assessment
- Risk: tái dùng animation hook sai chỗ làm pane render bị redraw quá mức.
- Mitigation: giới hạn motion ở chrome quads/UI elements, đo bằng idle behavior và manual smoke.

## Security Considerations
- Không liên quan auth/data.
- Tránh motion che khuất confirm/cancel states của overlay destructive.

## Không Đụng Terminal Core
- No-touch tuyệt đối: `crates/chatminal-terminal-core/**`
- No-touch tuyệt đối: `crates/chatminal-runtime/src/state/**`
- No-touch tuyệt đối: terminal glyph parser/render pipeline semantics
- Không animate terminal text stream, cursor semantics, scrollback behavior

## Next Steps
- Phase 07 verify toàn bộ shell polish và sync docs nếu behavior/UI contract thay đổi đáng kể.
