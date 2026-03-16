# Phase 03 - Desktop Mutation Routing Cutover

## Context Links
- `apps/chatminal-desktop/src/chatminal_desktop_session.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/desktop_spawn.rs`
- `apps/chatminal-desktop/src/desktop_mouse_actions.rs`
- `apps/chatminal-desktop/src/desktop_overlay_actions.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_*`

## Overview
- Priority: P0
- Status: completed
- Brief: chuyển toàn bộ desktop mutation path sang `chatminal-runtime` facade; desktop không còn mutate product state bằng logic riêng.

## Key Insights
- Đây là phase quan trọng nhất cho thin client conversion.
- Chỉ giữ desktop-side logic cho input interpretation và render invalidation.

## Requirements
- Desktop actions về profile/session/layout phải đi qua `chatminal_runtime::...` facade.
- Loại bỏ desktop-side mutation orchestration cho `active_session`, `workspace_layout`, `session clone/split`.
- Giảm trách nhiệm của `chatminal_desktop_session.rs` xuống wrapper mỏng hoặc xóa dần.

## Architecture
- Desktop action flow đích: `TermWindow/UI -> chatminal_runtime facade -> runtime core`.
- Không cho action flow đi trực tiếp `desktop -> session-runtime` trừ engine-only helper nội bộ bị runtime facade gọi lại.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/chatminal_desktop_session.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_spawn.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_mouse_actions.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_overlay_actions.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_actions_impl.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_session_close_helpers.rs`

## Implementation Steps
1. Migrate create/activate/close/clone/split/focus actions.
2. Migrate profile switch/create/delete actions.
3. Migrate persist-history/session setting actions.
4. Xóa hoặc làm mỏng các desktop helpers chỉ còn forwarding.

## Todo List
- [x] Session mutations batch đầu đã đi qua runtime facade
- [x] Profile mutations batch đầu đã đi qua runtime facade
- [x] Workspace layout mutations batch đầu đã đi qua runtime facade
- [x] Desktop helper dead paths được delete/refactor

## Success Criteria
- Desktop không còn product mutation bypass runtime core.
- `chatminal_desktop_session.rs` không còn là application orchestrator thứ hai.

## Risk Assessment
- Risk: event ordering gãy khi action path đổi.
- Mitigation: giữ integration tests cho activate/close/split/clone và verify active session/layout sau mỗi action.

## Security Considerations
- Phải giữ validation trước khi close/delete/switch để tránh state persistence sai profile.

## Next Steps
- Phase 04 chuyển query/snapshot path sang một nguồn duy nhất.
