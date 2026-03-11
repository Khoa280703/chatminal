# Phase 01 - Runtime Boundary Freeze And Inventory

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/engine_surface_adapter.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/frontend.rs

## Overview
- Priority: P0
- Current status: pending
- Brief: đóng băng boundary migration, đếm chính xác mọi callsite active path còn dựa vào `mux/tab/pane`, phân loại thành nhóm command, render, action, overlay, frontend, startup.

## Key Insights
- Vấn đề hiện tại không còn là naming public; vấn đề là execution source of truth
- Nếu không inventory sạch ngay từ đầu sẽ dễ migrate thiếu một nhóm callsite rồi phải vá vòng sau
- `EngineSurfaceAdapter` là marker ranh giới quan trọng: mọi chỗ còn đi qua nó là active path chưa session-native

## Requirements
- Functional: có bản đồ callsite hoàn chỉnh cho active runtime path
- Functional: inventory phải có line refs/owner phase cụ thể, không chỉ danh sách file
- Non-functional: không sửa behavior ở phase này; chỉ freeze kiến trúc và checklist migration

## Architecture
- Chia callsite còn sót thành 6 bucket:
  1. Session engine core commands
  2. Desktop session host/bootstrap
  3. TermWindow routing
  4. Overlay/frontend action routing
  5. Pane/render/update notifications
  6. Startup/dependency/wiring
- Định nghĩa rõ active path và compatibility path

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/plans/20260311-1728-chatminal-session-execution-core-final-cutover/plan.md
- Modify: /Users/khoa2807/development/2026/chatminal/plans/20260311-1728-chatminal-session-execution-core-final-cutover/phase-01-runtime-boundary-freeze-and-inventory.md

## Implementation Steps
1. Grep toàn bộ active path cho `EngineSurfaceAdapter`, `Mux::get`, `get_tab`, `get_pane`, `spawn_tab_or_window`, `move_pane_to_new_tab`, `focus_pane_and_containing_tab`
2. Loại trừ `third_party/`, test-only slices và engine-private crates không nằm trong active desktop flow
3. Gắn từng callsite vào bucket ownership cụ thể
4. Ghi inventory theo line refs/file refs cho từng callsite active; đánh dấu rõ callsite nào là render/compat slice còn lại ngoài phạm vi phase
5. Viết acceptance checklist cho từng bucket để phase sau chỉ việc đốt checklist
6. Xác nhận các file nào sau cùng phải bị xóa hoàn toàn, file nào chỉ cần refactor

## Todo List
- [ ] Hoàn tất callsite inventory có bucket ownership
- [ ] Inventory có line refs cho từng active callsite
- [ ] Đánh dấu active path vs compatibility path
- [ ] Freeze danh sách file target cho từng phase
- [ ] Chốt grep gates dùng lại ở mọi phase sau

## Success Criteria
- Không còn tranh luận mơ hồ “đã bỏ tab chưa”
- Mỗi callsite `mux/tab/pane` trong active path đều có owner phase rõ ràng
- Mỗi callsite active đều có line ref để verify phase completion không phụ thuộc raw grep-zero
- Không có edit code runtime thật ở phase này

## Risk Assessment
- Risk: bỏ sót callsite ẩn trong `termwindow` hoặc `frontend`
- Mitigation: dùng grep theo API cụ thể, không grep theo từ khóa chung chung

## Security Considerations
- Không có thay đổi runtime behavior

## Next Steps
- Chuyển ngay sang Phase 02 sau khi inventory xong và không còn bucket mơ hồ
