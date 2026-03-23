# Phase 04 - Session Bar And Footer Refinement

## Context Links
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/system_metrics.rs`

## Overview
- Priority: P1
- Current status: pending
- Brief: tinh lại session bar và footer để đồng bộ typography, spacing, status density, hover states, và role của chúng trong shell chrome.

## Key Insights
- `tabbar.rs` đã là source cho session bar state + status text parsing.
- Footer hiện đang build ngay trong `chatminal_sidebar.rs` với metrics + session/profile summary.
- Hai vùng này đang hợp lệ về chức năng nhưng visual language chưa khóa chung.

## Mục tiêu
- Đồng bộ tone giữa session bar và footer.
- Giảm noise ở status text nhưng giữ thông tin hữu ích.
- Làm rõ hierarchy: session navigation ở trên, health/ambient info ở dưới.

## Phạm vi
- Session item affordance, new-session CTA, hover/active styling.
- Footer label/value balance, separator rhythm, truncation strategy.
- Status density theo window width nhỏ/lớn.

## Files Likely Touched
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/system_metrics.rs`

## Requirements
- Không đổi session switching semantics.
- Không đổi metrics sampling/backing source.
- Footer phải degrade gracefully khi width hẹp, không đè vào content hay sidebar.

## Architecture
- `tabbar.rs` tiếp tục giữ data-to-line composition cho session bar.
- `termwindow/render/tab_bar.rs` và render shell consume theme/spacing thống nhất.
- Footer composition ở `chatminal_sidebar.rs` dùng cùng visual token với session bar nhưng không share logic mù quáng.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/tabbar.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Modify: `apps/chatminal-desktop/src/system_metrics.rs`
- Create: none
- Delete: none

## Implementation Steps
1. Audit current session bar states: active, inactive, hover, new button, window buttons.
2. Chốt status density rules theo width tiers.
3. Refactor footer composition để label/value/truncation rõ hơn.
4. Đồng bộ spacing, muted/accent palette, edge paddings giữa top và bottom chrome.
5. Verify hover/click layout với sidebar-enabled mode.

## Todo List
- [ ] Session bar visual hierarchy rõ
- [ ] Footer không quá dày thông tin
- [ ] Small-width layout không vỡ
- [ ] Hover/active styling nhất quán top-bottom chrome

## Success Criteria
- User nhìn biết ngay session nào active, button nào tạo session, footer đang báo gì.
- Footer không cắt xấu hoặc chồng lên sidebar/content.
- Không phát sinh regression ở integrated title buttons/session bar mapping.

## Risk Assessment
- Risk: sửa session bar text composition có thể làm lệch width calculation.
- Mitigation: giữ logic parse/status width trong `tabbar.rs`, chỉ tinh composition và fallback rules.

## Security Considerations
- Không tăng surface command execution.
- Không expose thêm runtime metadata nhạy cảm ở footer.

## Không Đụng Terminal Core
- No-touch tuyệt đối: `crates/chatminal-terminal-core/**`
- No-touch tuyệt đối: `crates/chatminal-runtime/src/state/**`
- No-touch tuyệt đối: `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- Không đổi alert/progress source, không đổi runtime event model

## Next Steps
- Phase 05 sẽ thống nhất overlay visual/layering để không phá session bar và footer chrome mới.
