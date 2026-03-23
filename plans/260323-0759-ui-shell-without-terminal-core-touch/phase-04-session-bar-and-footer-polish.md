# Phase 04 - Session Bar And Footer Polish

## Context Links
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`
- `apps/chatminal-desktop/src/termwindow/render/fancy_tab_bar.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`

## Overview
- Priority: P2
- Status: pending
- Brief: polish session bar và footer để chrome nhất quán, rõ hierarchy, không tranh focus với terminal content

## Objective
- Nâng chất session bar + footer thành product chrome đồng bộ với sidebar/layout contract.
- Làm rõ what is status vs what is navigation vs what is telemetry.

## Scope
- Session bar: spacing, truncation, active state, close affordance, new-session affordance.
- Footer: label/value rhythm, density, session/profile telemetry arrangement, divider treatment.
- Mouse/hover: top bar và footer không conflict với terminal content area.

## Files Likely Touched
- Modify: `apps/chatminal-desktop/src/tabbar.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/fancy_tab_bar.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Modify: `apps/chatminal-desktop/src/desktop_commands.rs`
- Delete: none

## Explicit Boundary
- Khong dung terminal core: không đổi tab/session switching semantics, không thêm runtime commands mới, không đổi telemetry source contracts ngoài việc hiển thị lại.

## Key Insights
- Session bar hiện vừa là compatibility shell vừa là product navigation; plan này chỉ polish visual/interaction shell.
- Footer content đã có metrics snapshot và session/profile labels; có thể sắp xếp lại không cần đụng metrics backend.

## Requirements
- Functional: active session/session view vẫn switch đúng, close/new affordance không bị shrink quá nhỏ.
- Non-functional: text truncation đẹp, hover rõ, footer đọc nhanh hơn, không flicker khi window resize.

## Architecture
- Session bar giữ source data cũ, chỉ refactor view composition.
- Footer dùng same chrome geometry contract từ Phase 02.
- Shared chrome tokens: padding, divider, hover bg, active bg, muted/value colors.

## Implementation Steps
1. Audit session bar item types và slot priorities.
2. Tách chrome token/padding constants để session bar và footer dùng chung.
3. Rebalance right/left status, close hit area, button affordance.
4. Refine footer grouping và ellipsis rules cho session/profile/metrics.

## Todo List
- [ ] Rework session bar spacing and hit areas
- [ ] Align footer layout with chrome contract
- [ ] Normalize visual tokens across top/bottom chrome
- [ ] Verify top/bottom session bar modes

## Success Criteria
- Session bar nhìn ổn định hơn với nhiều session và title dài.
- Footer rõ hierarchy, không lấn terminal content, không lệch với sidebar/footer bounds.
- Hover/click areas đủ lớn và không overlap terminal hit-test.

## Risk Assessment
- Risk: giữ compatibility session bar path nhưng visual refactor làm lệch old config behaviors.
- Mitigation: tách visual-only changes khỏi assignment routing, verify config matrix.

## Security Considerations
- Không để close/switch hit area bắn sai assignment do truncation/hitbox mismatch.
- Không hiển thị metrics/session metadata ngoài dữ liệu shell đang có sẵn.

## Next Steps
- Sang Phase 05 để polish overlay shell và interaction routing quanh render scopes.
