# Phase 01 - Product Model Freeze And Compatibility Boundary

## Context Links
- `README.md`
- `crates/chatminal-session-runtime/src/*`
- `apps/chatminal-desktop/src/chatminal_runtime/session_host.rs`
- `apps/chatminal-desktop/src/chatminal_session_surface.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: chốt product model mới và cài foundation types/state đầu tiên để các phase sau không tiếp tục đổ thêm logic vào `surface/leaf`.

## Key Insights
- Kiến trúc hiện tại đã session-native ở execution path nhưng split model vẫn là `surface -> leaf`.
- Nếu không freeze boundary ngay, mỗi bugfix tiếp theo sẽ lại đào sâu vào `leaf` semantics.

## Requirements
- Functional: định nghĩa `SessionViewId`, `WorkspaceNodeId`, `WorkspaceLayoutState`, `SessionViewSnapshot`.
- Non-functional: compile sạch, test mutation cơ bản, không làm đổi behavior runtime hiện tại.

## Architecture
- `session-runtime` thêm workspace-layout model độc lập với `surface/leaf`.
- Desktop phase đầu chỉ đọc model mới; chưa cutover render.

## Related Code Files
- Modify: `crates/chatminal-session-runtime/src/lib.rs`
- Modify: `crates/chatminal-session-runtime/src/session_ids.rs`
- Create: `crates/chatminal-session-runtime/src/workspace_layout.rs`

## Implementation Steps
1. Thêm id types cho `session_view` và `workspace_node`.
2. Tạo state snapshot + mutation cho split/attach/close/focus.
3. Viết test collapse tree và active-view fallback.
4. Export public API để desktop phase sau consume.

## Todo List
- [x] Add ids
- [x] Add layout state
- [x] Add tests
- [x] Run runtime crate tests

## Success Criteria
- Runtime crate có model mới compile được.
- Có test chứng minh split/close view chạy đúng.

## Risks
- Naming đụng với `LayoutNodeId` cũ.
- Mitigation: dùng prefix `WorkspaceNodeId` riêng.
