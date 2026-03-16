# Phase 06 - `desktop_host_runtime` Adapter Shrink And Desktop Simplification

## Context Links
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/pane.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- `apps/chatminal-desktop/src/termwindow/*`

## Overview
- Priority: P0
- Status: completed
- Brief: cắt toàn bộ ownership logic product khỏi `desktop_host_runtime`; desktop/host layer chỉ còn engine adapter, render host, terminal plumbing.

## Key Insights
- Đây là phase làm kiến trúc “sạch” thật sự ở desktop engine boundary.
- `desktop_host_runtime` phải không còn quyết định active session/profile/workspace semantics.

## Requirements
- Xóa logic ownership product khỏi host runtime modules.
- Giữ lại duy nhất các nhóm trách nhiệm:
  - terminal process/render handle
  - engine adapter implementation
  - overlay/render compatibility
  - domain/window/pty backend helpers
- Desktop termwindow/helpers chỉ dùng host layer cho render/terminal IO, không dùng để suy ra business state.
- Loại bỏ việc `desktop_host_runtime/session_host.rs` trực tiếp sở hữu/persist `active_session` hoặc `workspace_layout` như business state bền vững; nếu cần cache thì chỉ là derived runtime cache.

## Architecture
- `desktop_host_runtime` là private adapter package bên trong desktop.
- Tất cả state business phải đã được chuẩn hóa ở runtime core trước khi bước vào adapter này.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/pane.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/mod.rs`

## Implementation Steps
1. Tách app ownership logic khỏi host runtime helpers.
2. Di chuyển mọi active session/profile/layout routing còn sót lên runtime facade.
3. Thu gọn `session_host` thành render/runtime host service, không phải product controller.
4. Gỡ persistence/layout store coupling khỏi `session_host`.
5. Delete dead compatibility helpers phát sinh.

## Todo List
- [x] `desktop_host_runtime` chỉ còn engine adapter concerns
- [x] Desktop termwindow active-session path không còn fallback sang sidebar snapshot
- [x] `session_host` không còn sở hữu `last_active_session_id` business hint; ownership chuyển về runtime facade
- [x] `session_host` không còn sở hữu business persistence/layout state
- [x] Dead helper paths bị delete/refactor
- [x] Desktop tests pass sau shrink

## Success Criteria
- `desktop_host_runtime` không còn là app core trá hình.
- Desktop layer dễ đọc: product state từ runtime snapshot, engine state từ host adapter.

## Risk Assessment
- Risk: gãy render/overlay path vì boundary tách mạnh.
- Mitigation: giữ integration gates cho launcher/copy/selection/render layout trong desktop tests.

## Security Considerations
- Không để terminal handle lifecycle leak khi delete helper paths.

## Next Steps
- Phase complete.
