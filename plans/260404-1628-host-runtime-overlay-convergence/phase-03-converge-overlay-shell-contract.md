---
phase: 03
status: completed
priority: medium
effort: medium
risk: high
---

# Phase 03: Converge Overlay Shell Contract

## Context Links
- [plan.md](./plan.md)
- [chatminal_runtime/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/mod.rs)
- [desktop_host_runtime/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/mod.rs)
- [overlay directory](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay)
- [termwindow directory](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow)

## Overview
- Priority: P2
- Current status: completed
- Mục tiêu: bỏ `overlay_compat` như một contract song song và đưa overlay/render/input chrome về cùng một shell contract canonical với desktop host runtime.

## Key Insights
- Overlay hiện không còn là debt lớn nhất, nhưng vẫn giữ một lớp vocabulary bridge riêng làm source khó đọc và dễ tạo duplicate boundary.
- Cutover này ảnh hưởng render/hit-test/cursor/scroll contract nên rủi ro cao hơn phase 1-2.
- Chỉ nên làm sau khi host seam phía dưới đã ổn định.

## Requirements
- Overlay panes, overlay terminals, split layouts, renderable dimensions, cursor helpers phải đi qua một contract canonical rõ ràng.
- Không regress overlay render, prompt, confirm, launcher, quickselect, debug overlay, copy/selector flows.
- Không thay feature behavior của overlays.

## Architecture
- Tạo một shell-facing contract canonical cho overlay use cases; contract này phải có owner rõ nằm ở desktop host/runtime surface hiện tại.
- `chatminal_runtime::overlay_compat` không còn là bridge re-export song song; nếu còn giữ chỉ là alias chuyển tiếp ngắn hạn trong cùng module nội bộ.
- Overlay modules và termwindow imports đọc trực tiếp từ canonical shell contract.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- Modify: `apps/chatminal-desktop/src/overlay/*.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/*.rs`
- Modify: render/selection/clipboard modules đang import `overlay_compat`

## Implementation Steps
1. Inventory toàn bộ symbol đang import từ `overlay_compat`.
2. Chốt canonical owner module cho overlay-facing types và helper functions.
3. Di chuyển/re-export tối thiểu sang contract mới, rồi đổi imports ở overlay/termwindow/render modules.
4. Xóa bridge `overlay_compat` rộng; chỉ giữ alias hẹp tạm thời nếu cần cho incremental cutover trong cùng PR wave.
5. Verify overlay render clipping, hit-test, scroll, cursor, launcher, confirm dialogs, quickselect, prompt/debug overlays.

## Todo List
- [x] Lập bảng symbol `overlay_compat` -> canonical owner
- [x] Định nghĩa shell contract canonical
- [x] Chuyển imports ở overlay modules
- [x] Chuyển imports ở termwindow/render modules
- [x] Xóa bridge rộng hoặc hạ xuống alias hẹp nội bộ
- [x] Chạy targeted compile/tests; desktop smoke còn để phase 04

## Success Criteria
- Overlay/termwindow active modules không còn import contract chính từ `overlay_compat`.
- Chỉ còn một shell contract cho overlay render/input types.
- Overlay UI behavior giữ nguyên sau cutover.

## Risk Assessment
- Dễ làm gãy render/hit-test ở các overlay ít dùng.
- Nếu cutover quá rộng một lần, diff lớn và khó isolate regressions.

## Security Considerations
- Không thêm capability thực thi mới cho overlays.
- Giữ nguyên clipboard/download/selection behavior hiện tại.

## Next Steps
- Phase 04 dùng để prune deadcode/docs chỉ sau khi overlay smoke pass.
