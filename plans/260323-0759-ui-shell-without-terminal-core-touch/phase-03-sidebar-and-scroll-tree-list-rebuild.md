# Phase 03 - Sidebar And Scroll Tree List Rebuild

## Context Links
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`

## Overview
- Priority: P1
- Status: pending
- Brief: nâng sidebar thành shell tree-list ổn định, scroll mượt, hit-test chuẩn, visual hierarchy rõ

## Objective
- Polish sidebar profile/session tree thành một scroll-tree-list thực thụ.
- Tách state scroll, row metrics, visual row kinds, và hit-test clipping cho dễ mở rộng.

## Scope
- Row taxonomy: profile, session, empty/error hint, status badge, affordance icon nhẹ.
- Scroll behavior: wheel, clamp, viewport clip, resize recovery, expand/collapse recovery.
- Tree visuals: indentation, active state, hover state, density, scrollbar thumb.

## Files Likely Touched
- Modify: `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_event_helpers.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/box_model.rs`
- Create: sidebar helper under `apps/chatminal-desktop/src/` only if row math needs split
- Delete: none

## Explicit Boundary
- Khong dung terminal core: không đổi session data source contract, không đổi runtime sidebar subscription protocol, không đổi terminal pane semantics.

## Key Insights
- Sidebar render và footer render hiện đang trộn trong `termwindow/render/chatminal_sidebar.rs`.
- Scroll state đã tồn tại tối thiểu trong `chatminal_sidebar/mod.rs`; có thể mở rộng mà không chạm runtime.

## Requirements
- Functional: list dài vẫn scroll được, hit-test đúng item sau clip/translate, không bleed ra footer.
- Non-functional: hover/active visuals nhất quán, không lag khi snapshot refresh.

## Architecture
- Keep `SidebarSnapshot` read-only.
- Move shell concerns thành 3 lớp: row projection, viewport/scroll math, visual element builder.
- Mouse wheel và click dùng cùng projected row bounds.

## Implementation Steps
1. Tách projection từ `SidebarSnapshot -> SidebarTreeRowView`.
2. Chuẩn hóa row height, indentation, clip rect, scrollbar math.
3. Rework click/wheel path để dựa trên projected visible rows.
4. Polish active/error/offline states và tạo room cho icon affordances nhẹ.

## Todo List
- [ ] Split projection from render
- [ ] Add visible-row viewport model
- [ ] Align wheel/click with same bounds
- [ ] Polish scrollbar and hierarchy visuals

## Success Criteria
- Sidebar tree scroll mượt, clamp đúng, không có ghost hitbox ngoài viewport.
- Expand/collapse và snapshot refresh không làm scroll jump sai.
- Visual hierarchy profile/session rõ và active state dễ đọc.

## Risk Assessment
- Risk: duplicate state giữa render tree và sidebar shared state.
- Mitigation: giữ snapshot read-only, chỉ persist scroll/ui expansion ở shell state hiện có.

## Security Considerations
- Không click-target sang session sai vì stale row mapping.
- Không render metadata nhạy cảm ngoài những field snapshot đang expose.

## Next Steps
- Sang Phase 04 để đồng bộ session bar và footer chrome với sidebar mới.
