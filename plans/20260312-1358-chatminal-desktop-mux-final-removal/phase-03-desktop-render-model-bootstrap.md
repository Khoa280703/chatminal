# Phase 03 - Desktop Render Model Bootstrap

## Context Links
- `apps/chatminal-desktop/src/chatminal_runtime/session_host.rs`
- `apps/chatminal-desktop/src/chatminal_desktop_session.rs`
- `apps/chatminal-desktop/src/termwindow/layout_render.rs`
- `apps/chatminal-desktop/src/termwindow/render/*`

## Overview
- Priority: P0
- Status: completed
- Brief: thay lớp `mux::Tab` compatibility bằng render model first-party của Chatminal.

## Key Insights
- `DesktopSessionHost` hiện vẫn tạo `render_tabs: HashMap<RuntimeId, Arc<Tab>>`.
- Đây là shim lớn nhất còn sót trong desktop active path.

## Requirements
- Tạo `ChatminalRenderTree`, `ChatminalRenderPane`, `ChatminalRenderSplit`, `ChatminalRenderCursor`.
- `DesktopSessionHost` trả snapshot/render state first-party.
- `chatminal_desktop_session` không còn `render_tab_for_session`.
- Render model mới không được expose `Arc<dyn Pane>` hay wrapper tương đương ở API app path.
- Render tree phải biểu diễn nhiều session cùng hiển thị trong layout; không biểu diễn split nội bộ của một session.

## Architecture
- Module mới: `apps/chatminal-desktop/src/chatminal_render/*`
- `session_host` giữ registry `runtime_id -> render_tree`.
- Render tree chỉ chứa metadata first-party + terminal buffer handle first-party; nếu còn cần glue xuống engine thì giữ private phía dưới, không lộ qua render API.
- Mỗi render leaf tương ứng một `terminal_instance` gắn với đúng một session.

## Related Code Files
- Create: `apps/chatminal-desktop/src/chatminal_render/*`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/session_host.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_desktop_session.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/layout_render.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/render/*`
- Refactor: `apps/chatminal-desktop/src/scrollbar.rs`

## Implementation Steps
1. Introduce render tree structs.
2. Move geometry/split computation from `Tab` shim sang render tree builder.
3. Replace `PositionedPane/PositionedSplit/Tab` plumbing trong desktop session helpers.
4. Add tests cho render tree from session/layout snapshots.

## Todo List
- [x] Add render tree module
- [x] Replace render tab registry
- [x] Replace desktop session helpers
- [x] Replace layout render plumbing
- [x] Add render mapping tests

## Success Criteria
- `session_host.rs` không còn `render_tabs: HashMap<RuntimeId, Arc<Tab>>`.
- `chatminal_desktop_session.rs` không còn import `mux::tab::*`.

## Risk Assessment
- Risk: geometry regression khi bỏ `Tab::iter_panes`.
- Mitigation: snapshot-based tests cho split ratios, active instance, pane order.

## Security Considerations
- Không ảnh hưởng security; chỉ render plumbing.

## Next Steps
- Sang Phase 04 để `termwindow` và action routing chỉ nói với render tree first-party.
