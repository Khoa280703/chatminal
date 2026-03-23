# Phase 02 - Layout Primitives And Chrome Slots

## Context Links
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
- `apps/chatminal-desktop/src/termwindow/box_model.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs`

## Overview
- Priority: P1
- Current status: pending
- Brief: gom các kích thước/slot layout của sidebar, top chrome, content, footer, overlay anchoring thành bộ primitive rõ ràng để các polish phase sau không tự tính toán lệch nhau.

## Key Insights
- `desktop_termwindow_render_mod.rs` đang giữ offset/padding cho sidebar + top chrome + footer.
- `desktop_termwindow_layout_render.rs` đang map workspace layout sang geometry pane/split.
- `termwindow/render/chatminal_sidebar.rs` đang tự giữ nhiều constant pixel cho header/tree/footer/scrollbar.

## Mục tiêu
- Chuẩn hóa shell slot geometry và spacing token.
- Loại bỏ duplicated pixel math giữa render shell và sidebar/footer paint path.
- Tạo nền cho responsive behavior và clipping ổn định.

## Phạm vi
- Sidebar width/height contract.
- Top chrome height, footer height, content padding, overlay anchor bounds.
- Layout primitive dùng lại cho split lines, tree viewport, footer padding.

## Files Likely Touched
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
- `apps/chatminal-desktop/src/termwindow/box_model.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/chatminal_layout/mod.rs`

## Requirements
- Một nguồn truth cho shell slot sizing.
- Không đổi terminal cell math hay render state contract.
- Footer, sidebar tree clip, overlay bounds phải bám cùng một layout contract.

## Architecture
- `desktop_termwindow_render_mod.rs` giữ shell metrics và padding entry points.
- `desktop_termwindow_layout_render.rs` chỉ consume primitive đã chuẩn hóa, không hardcode thêm.
- `termwindow/render/chatminal_sidebar.rs` chỉ paint theo slot contract, không tính geometry độc lập lần hai.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/box_model.rs`
- Create: none
- Delete: none

## Implementation Steps
1. Audit toàn bộ constant slot/chrome/footer/sidebar đang tồn tại.
2. Gom token/kích thước vào shell primitive layer hiện có.
3. Chuyển sidebar/footer/overlay anchor sang consume primitive chung.
4. Rà lại split render và content padding để không double-count offset.
5. Thêm verify cases cho narrow width và tall/short viewport.

## Todo List
- [ ] Có shared shell metrics rõ ràng
- [ ] Sidebar/tree/footer dùng cùng slot contract
- [ ] Không còn duplicated pixel math đáng kể
- [ ] Overlay bounds bám shell layout mới

## Success Criteria
- Resize window không làm sidebar/footer/content lệch nhau.
- Tree clip rect, footer rect, overlay rect không chồng sai vùng content terminal.
- Phase 03-06 có thể polish UI mà không viết thêm geometry hacks.

## Risk Assessment
- Risk: đổi slot math làm lệch hit-test hoặc split drag.
- Mitigation: chạm `desktop_termwindow_mouseevent.rs` chỉ khi cần align coordinates; không đổi interaction semantics.

## Security Considerations
- Chỉ là geometry/render shell.
- Không thêm mutation path mới sang runtime.

## Không Đụng Terminal Core
- No-touch tuyệt đối: `crates/chatminal-terminal-core/**`
- No-touch tuyệt đối: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/**`
- No-touch tuyệt đối: `crates/chatminal-runtime/src/workspace_layout.rs`
- Không đổi terminal size negotiation, pane runtime ownership, render-target contract

## Next Steps
- Phase 03 dùng primitives mới để sửa sidebar tree và scroll behavior nhất quán.
