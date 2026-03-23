# Phase 05 - Overlay Shell And Interaction Routing Polish

## Context Links
- `apps/chatminal-desktop/src/overlay/mod.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `apps/chatminal-desktop/src/overlay/quickselect.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_state_helpers.rs`

## Overview
- Priority: P2
- Status: pending
- Brief: polish overlay shell UX và đảm bảo overlay attach/cancel/resize dựa trên render-scope seam hiện có, không lan sang runtime core

## Objective
- Làm overlay behavior nhất quán hơn về sizing, padding, focus affordance, close/cancel path.
- Cô lập overlay shell polish khỏi runtime ownership.

## Scope
- Overlay chrome: launcher, quickselect, copy/confirm/prompt family.
- Overlay lifecycle shell: spawn, resize, cancel, active-state visual, background dimming nếu cần.
- Interaction routing: active overlay vs active pane vs active render target.

## Files Likely Touched
- Modify: `apps/chatminal-desktop/src/overlay/mod.rs`
- Modify: `apps/chatminal-desktop/src/overlay/launcher.rs`
- Modify: `apps/chatminal-desktop/src/overlay/quickselect.rs`
- Modify: `apps/chatminal-desktop/src/overlay/confirm.rs`
- Modify: `apps/chatminal-desktop/src/overlay/prompt.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_state_helpers.rs`
- Delete: none

## Explicit Boundary
- Khong dung terminal core: không đổi overlay terminal implementation internals, không đổi runtime overlay protocol, không đổi pane IO semantics.

## Key Insights
- Overlay bootstrap seam đã khá rõ ở `overlay/mod.rs` + `desktop_termwindow_host_runtime_helpers.rs`.
- Rủi ro lớn nhất nằm ở stale render scope sizing/cancel path, không phải ở core logic.

## Requirements
- Functional: overlay attach đúng scope, resize đúng bounds, cancel path sạch, focus visual rõ.
- Non-functional: giảm visual drift giữa overlay types, không thêm lag cho input path.

## Architecture
- Keep existing `start_overlay`/assign/cancel flow.
- Thêm shell-level overlay style contract và scope sizing helper.
- Overlay visual states phải consume geometry contract Phase 02 thay vì tự tính lẻ.

## Implementation Steps
1. Audit overlay family và group common chrome concerns.
2. Chuẩn hóa bounds/padding/close affordance/focus visuals.
3. Rework resize/cancel shell flow để dùng helper chung.
4. Verify overlay-active path không break mouse/key routing sang terminal content.

## Todo List
- [ ] Inventory overlay chrome differences
- [ ] Add shared overlay shell contract
- [ ] Align resize/cancel/focus shell behavior
- [ ] Verify launcher/quickselect/copy/confirm parity

## Success Criteria
- Overlay family nhìn và hành xử nhất quán hơn.
- Resize/cancel không leave stale overlay state.
- Không có thay đổi ở runtime/core overlay internals.

## Risk Assessment
- Risk: visual refactor chạm nhầm behavior path của launcher/quickselect.
- Mitigation: giữ behavior logic cũ, chỉ bọc qua shared shell helpers và test từng overlay family.

## Security Considerations
- Không leak overlay input sang pane không active.
- Không giữ stale overlay handles sau close/cancel.

## Next Steps
- Sang Phase 06 để thêm iconography và micro-animation nhẹ trên shell surfaces.
