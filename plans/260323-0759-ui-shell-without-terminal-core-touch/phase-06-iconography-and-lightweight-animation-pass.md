# Phase 06 - Iconography And Lightweight Animation Pass

## Context Links
- `apps/chatminal-desktop/src/desktop_commands.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
- `apps/chatminal-desktop/src/termwindow/render/paint.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`

## Overview
- Priority: P3
- Status: pending
- Brief: thêm icon affordance và motion rất nhẹ để chrome có tín hiệu rõ hơn nhưng không làm terminal content thành sân khấu

## Objective
- Dùng iconography và animation nhỏ để tăng clarity cho sidebar/session bar/footer/overlay actions.
- Reuse animation scheduling sẵn có, tránh tạo animation engine mới.

## Scope
- Icon affordances cho create/close/expand/status items ở shell chrome.
- Motion nhẹ: hover fade, active pulse rất nhỏ, scrollbar/thumb easing, badge transition.
- Không animate terminal content cells hay parser-driven paint path.

## Files Likely Touched
- Modify: `apps/chatminal-desktop/src/desktop_commands.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Modify: `apps/chatminal-desktop/src/tabbar.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/paint.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/mod.rs`
- Delete: none

## Explicit Boundary
- Khong dung terminal core: không animate terminal cell/state semantics, không thêm render invalidation path vào parser/core, không thay icon/title protocol.

## Key Insights
- `has_animation` đã tồn tại trong `TermWindow`; nên tận dụng cho micro-interactions shell-only.
- `desktop_commands.rs` đã có icon vocabulary phong phú; có thể reuse thay vì invent mới.

## Requirements
- Functional: affordance rõ hơn nhưng không đổi action semantics.
- Non-functional: animation subtle, bounded, không spam invalidate, không ảnh hưởng scroll/input hot path.

## Architecture
- Shell motion tokens: duration, easing, allowed surfaces.
- Motion only on chrome overlays/components, not terminal pane contents.
- Icon selection lấy từ existing command/icon vocabulary hoặc local constants.

## Implementation Steps
1. Chốt icon affordance map cho sidebar/session bar/footer/overlay.
2. Thêm shell motion policy dựa trên `has_animation` đã có.
3. Gắn fade/hover/active transitions cho chrome components đủ giá trị.
4. Verify reduced repaint footprint và không ảnh hưởng pane render cadence.

## Todo List
- [ ] Define icon affordance map
- [ ] Define allowed animation surfaces
- [ ] Reuse existing scheduling path
- [ ] Verify no hot-path regressions

## Success Criteria
- Chrome UI rõ action hơn nhờ icon affordance hợp lý.
- Motion nhìn có chủ đích, rất nhẹ, không gây distraction.
- Không tăng repaint quá mức và không đụng terminal content path.

## Risk Assessment
- Risk: animation leak sang paint hot path làm render churn.
- Mitigation: gate strictly shell surfaces only, coalesce frame scheduling, keep durations short.

## Security Considerations
- Không encode state nhạy cảm vào icon/status visuals mới.
- Không để animation che mất destructive affordance như close/confirm.

## Next Steps
- Sang Phase 07 để dựng regression gates và rollout freeze.
