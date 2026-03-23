# Phase 03 - Sidebar And Scroll Tree List Polish

## Context Links
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_state_helpers.rs`
- `apps/chatminal-desktop/src/system_metrics.rs`

## Overview
- Priority: P1
- Current status: pending
- Brief: polish sidebar từ data snapshot tới paint tree rows, scroll thumb, hover, active/expanded feedback mà không thay đổi payload/runtime contract bên dưới.

## Key Insights
- `chatminal_sidebar/mod.rs` đã có state sync, expand set, scroll bounds, scroll offset.
- `termwindow/render/chatminal_sidebar.rs` đã có row model, clip rect, scrollbar paint, footer content.
- `desktop_termwindow_mouseevent.rs` đã bắt wheel và click cho sidebar; đây là seam tốt để tinh interaction.

## Mục tiêu
- Làm tree rõ phân cấp profile/session, active state, empty state, error state.
- Cải thiện scroll feel và thumb visibility cho danh sách dài.
- Giữ ordering/session source of truth ở runtime snapshot hiện có.

## Phạm vi
- Tree row spacing, row density, expand/collapse affordance.
- Scrollbar/thumb styling, wheel behavior, hover hit area.
- Active row emphasis, secondary metadata, empty/error copy ngắn gọn.

## Files Likely Touched
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/system_metrics.rs`

## Requirements
- Không đổi subscription API hay snapshot schema.
- Scroll bounds luôn clamp đúng, không jitter khi tree height thay đổi.
- Hover/click areas phải ăn khớp với clip rect và translated tree position.

## Architecture
- State vẫn ở `ChatminalSidebar`.
- Row composition và paint ở `termwindow/render/chatminal_sidebar.rs`.
- Interaction routing ở `desktop_termwindow_mouseevent.rs`.
- Footer metrics chỉ consume read-only data; không kéo business logic mới vào render.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- Create: none
- Delete: none

## Implementation Steps
1. Chuẩn hóa visual states cho profile row, session row, empty/error rows.
2. Tách tree row spacing và thumb style khỏi magic numbers rải rác.
3. Tinh wheel delta, thumb dragging, clamp behavior cho danh sách dài.
4. Làm active/expanded affordance rõ nhưng không thêm animation nặng.
5. Verify hit-test sau khi tree bị translate theo scroll offset.

## Todo List
- [ ] Tree row hierarchy rõ
- [ ] Scroll thumb dễ thấy và không giật
- [ ] Empty/error states nhất quán
- [ ] Click/hover vẫn đúng sau scroll

## Success Criteria
- Sidebar đọc nhanh hơn ở cả ít session và nhiều session.
- Scroll tree không overshoot, không mất active row context.
- Không xuất hiện desync giữa active session trong sidebar và content pane.

## Risk Assessment
- Risk: translate tree rồi clip làm hit-test sai.
- Mitigation: bind hit area vào computed UI items sau clamp, test wheel + click sau scroll.

## Security Considerations
- Chỉ render/read-only UI state.
- Không được thêm action ẩn hoặc destructive shortcut mới trong sidebar.

## Không Đụng Terminal Core
- No-touch tuyệt đối: `crates/chatminal-terminal-core/**`
- No-touch tuyệt đối: `crates/chatminal-runtime/src/state/**`
- No-touch tuyệt đối: `apps/chatminal-desktop/src/desktop_host_runtime/**` trừ khi chỉ đọc public helper đã có; không sửa behavior
- Không đổi session ordering contract, subscription protocol, store schema

## Next Steps
- Phase 04 lấy visual language từ sidebar đã ổn định để thống nhất session bar và footer.
