## Context Links
- [README](/Users/khoa2807/development/2026/chatminal/README.md)
- [chatminal_sidebar/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_sidebar/mod.rs)
- [termwindow/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs)
- [termwindow/resize.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/resize.rs)
- [desktop_termwindow_mouseevent.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs)
- [termwindow/render/chatminal_sidebar.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs)

## Overview
- Priority: P2
- Status: pending
- Brief: thêm drag-resize cho sidebar desktop bằng shell/UI state nội bộ, không đụng terminal core.

## Key Insights
- Sidebar width hiện là hằng số trong [chatminal_sidebar/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_sidebar/mod.rs).
- Geometry shell lấy width qua [termwindow/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs), nên đây là điểm nối chính.
- Mouse drag hiện đã có pattern sẵn cho split/scrollbar trong [desktop_termwindow_mouseevent.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs); có thể tái dùng.

## Requirements
- Functional: kéo mép phải sidebar để đổi width; update realtime; có cursor resize; clamp min/max.
- Non-functional: không lệch hitbox, không phá render pane/footer/tab bar, không chạm terminal core.

## Related Code Files
- Modify:
  - `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
  - `apps/chatminal-desktop/src/termwindow/mod.rs`
  - `apps/chatminal-desktop/src/termwindow/resize.rs`
  - `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Create: none
- Delete: none

## Implementation Steps
1. Thêm sidebar width state + API set/get/clamp.
2. Thay toàn bộ consumer shell width sang state động.
3. Render resize handle ở mép phải sidebar.
4. Thêm UI item + mouse drag lifecycle cho handle.
5. Verify layout/hit-test bằng `cargo check -p chatminal-desktop` + smoke manual.

## Todo List
- [ ] Add width state and clamp helpers
- [ ] Wire dynamic width into shell bounds and resize padding
- [ ] Add sidebar resize handle rendering
- [ ] Add sidebar resize drag interaction
- [ ] Verify desktop shell layout

## Success Criteria
- Kéo được sidebar mượt, content area co giãn đúng.
- Không cần sửa `crates/chatminal-terminal-core`.
- `cargo check -p chatminal-desktop` pass.

## Risk Assessment
- Render bounds và mouse bounds mismatch.
- Width mới làm lệch tab/footer width calc.

## Security Considerations
- Không có auth/data surface mới.
- Chỉ đổi local GUI state.

## Next Steps
- Nếu UX ổn, cân nhắc persist width qua restart như phase follow-up.

## Unresolved Questions
- Persist width: có cần trong MVP không?
