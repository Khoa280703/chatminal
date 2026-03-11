# Phase 04 - TermWindow Routing Migration

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/clipboard.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/paneselect.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mouseevent.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/spawn.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/resize.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/paint.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs

## Overview
- Priority: P0
- Current status: pending
- Brief: chuyển `termwindow` sang route hoàn toàn bằng `session_id/surface_id/leaf_id`, bỏ lookup `Mux::get().get_tab/get_pane` trong active session flow.

## Key Insights
- `termwindow` đang là nơi còn nhiều callsite `mux` nhất trong active desktop flow
- Không cần xóa toàn bộ `mux` usage của `termwindow` ngay lập tức; chỉ cần bóc hết session-mode path trước
- Tách helper theo mode là cách an toàn để tránh gãy flow legacy/compat nếu còn tồn tại

## Requirements
- Functional:
  - active session lookup từ session host/state
  - focus routing dùng `surface_id/leaf_id`
  - close routing dùng session engine commands
  - spawn source resolution dùng session active leaf thay vì host pane
- Non-functional: không hồi quy selection, input focus, split navigation, resize behavior

## Architecture
- Tạo cụm helper mới trong `termwindow` cho session mode:
  - `active_session_context()`
  - `resolve_active_leaf()`
  - `focus_leaf_by_id()`
  - `close_active_session_surface()`
  - `spawn_from_active_session_leaf()`
- Dùng host abstraction từ Phase 03 làm nguồn duy nhất
- `Mux` còn lại trong `termwindow` chỉ được phép ở non-session compatibility slices đã được comment rõ

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/clipboard.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/paneselect.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mouseevent.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/spawn.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/resize.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/paint.rs

## Implementation Steps
1. Tách các helper session-mode hiện còn dùng `Mux` sang helper bọc session host/state
2. Chuyển active lookup, focus, close, move, split, spawn source resolution sang helper mới
3. Chuyển resize/invalidation path sang session surface metadata thay vì host tab metadata
4. Giảm dần field/state cache cũ `surface_state`, `leaf_state` nếu đã dư
5. Thêm test cho routing helpers quan trọng

## Todo List
- [ ] Active lookup không gọi `get_tab/get_pane` trong session-mode path
- [ ] Focus/close/move/split route bằng session engine commands
- [ ] Resize/spawn source dùng active leaf thật
- [ ] Session-mode helpers độc lập khỏi host tab metadata

## Success Criteria
- Grep `Mux::get|get_tab|get_pane` trong `termwindow` giảm về zero cho session-mode path
- Chuyển session, split leaf, close surface, move leaf không còn phụ thuộc host tab identity
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass

## Risk Assessment
- Risk: `termwindow` file lớn, dễ lẫn session path và compatibility path
- Mitigation: refactor helper trước, thay callsite sau, verify từng lát cắt nhỏ

## Security Considerations
- Không đổi quyền thực thi shell; chỉ đổi route nội bộ

## Next Steps
- Sau khi `termwindow` sạch active path, Phase 05 sẽ dọn frontend/overlay/actions quanh nó
