# Phase 06 - Desktop Renderer Input And Overlay Cutover

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mouseevent.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/fancy_tab_bar.rs

## Overview
- Priority: P1
- Status: completed
- Brief: desktop shell chỉ bind bằng session graph snapshots, bỏ tab-centric renderer/input path

## Key Insights
- Đây là phase xóa semantics `tab` khỏi UX thật sự
- `TabInformation` và `TabBarItem::Tab` chỉ nên còn như compatibility shell tạm, rồi xóa ở phase cuối
- Overlay/input hiện bám nhiều vào active tab/pane; phải đổi sang active session surface/leaf

## Requirements
- Functional: session bar, input routing, overlay targeting, focus visuals chạy bằng session ids/surface ids
- Non-functional: giữ terminal core nguyên vẹn

## Architecture
- Replace tab-centric snapshot with session-centric snapshot in desktop shell
- Route input/overlay by `session_id` + `surface_id` + `leaf_id`
- Renderer lấy active render target từ `SessionSurfaceSnapshot`
- Phase này chỉ consume contract đã chốt từ Phase 05; không được tự tạo thêm runtime ownership rule mới trong UI layer

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mouseevent.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/tabbar.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/fancy_tab_bar.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/*.rs

## Implementation Steps
1. Thay session bar source data bằng session graph snapshot trực tiếp
2. Chuyển click/close/input routing sang ids ổn định của session surface
3. Chuyển overlay targeting khỏi active mux tab/pane
4. Xóa path tab-centric khỏi desktop shell khi đã có replacement
5. Giữ terminal render/input core nguyên vẹn; chỉ đổi binding identity và target resolution

## Todo List
- [x] Replace top bar snapshot source
- [x] Replace input routing ids
- [x] Replace overlay target ids
- [x] Remove `TabBarItem::Tab` from session UX path
- [x] Verify mọi UI target đều đi qua `session_id/surface_id/leaf_id`

## Success Criteria
- Desktop shell không còn semantics `tab` ở UX path
- Overlay/input/session bar chạy theo session graph mới
- Build và manual QA pass
- Không phải thêm mới runtime contract ngoài những gì đã chốt ở Phase 05

## Risk Assessment
- Risk: input gửi sai surface, overlay attach nhầm leaf
- Mitigation: cutover từng feature flag nội bộ và giữ tests/manual smoke

## Security Considerations
- Route input bằng stable ids để tránh cross-session injection bug
- Đảm bảo close session cleanup hủy hết overlay/input handles liên quan

## Validation Gates
- `cargo check -p chatminal-desktop`
- `make smoke-window`
- `rg -n "TabInformation|TabBarItem::Tab|active tab|active pane|PaneId|TabId" apps/chatminal-desktop/src/termwindow apps/chatminal-desktop/src/tabbar.rs apps/chatminal-desktop/src/overlay`
  - expected: zero results trong session UX path sau cutover; compatibility shell nếu còn phải bị cô lập ngoài các path này

## Next Steps
- Sang Phase 07 để remove mux public surface và cleanup cứng
