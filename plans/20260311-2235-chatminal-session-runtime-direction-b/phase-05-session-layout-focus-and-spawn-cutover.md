# Phase 05 - Session Layout Focus And Spawn Cutover

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/paneselect.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/resize.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/spawn.rs

## Overview
- Priority: P1
- Status: completed
- Brief: thay `pane/split/tab focus` semantics bằng `SessionLayoutNode/SessionSurface`

## Key Insights
- Đây là phase nuốt complexity thật của mux
- Không thể xóa pane tree nếu app vẫn còn split terminal
- Sau phase này app layer không nên gọi `Pane` làm model chính nữa

## Requirements
- Functional: split, focus, spawn, close hoạt động qua session graph mới
- Non-functional: không regress terminal interaction, resize, overlay targeting

## Architecture
- Introduce:
  - `SessionLayoutNode`
  - `SessionLeaf`
  - `SessionSplit`
  - `SessionFocusManager`
  - `SessionSpawnManager`
- Adapter chuyển xuống engine backend ở dưới nếu còn cần
- Phase này phải xuất được runtime APIs đủ cho Phase 06:
  - traversal theo `layout_node_id`
  - resolve active render target theo `leaf_id`
  - lookup hit-target hoặc logical target không dựa index
  - stale-id behavior rõ khi close/split rollback xảy ra

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/paneselect.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/resize.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/spawn.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_layout_tree.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_focus_manager.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_spawn_manager.rs

## Implementation Steps
1. Define layout tree types và snapshot API
2. Move focus operations vào focus manager mới
3. Move split/spawn operations vào spawn manager mới
4. Chuyển paneselect/resize/spawn desktop paths sang API mới
5. Chốt readiness contract cho renderer/input/overlay trước khi sang Phase 06

## Todo List
- [x] Define layout node graph
- [x] Add focus manager
- [x] Add spawn manager
- [x] Replace pane-centric desktop flows
- [x] Publish traversal/render-target/stale-id APIs cho Phase 06

## Success Criteria
- Desktop split/focus/spawn không cần gọi tab/pane-centric public API
- Session surface có active leaf rõ ràng
- Resize/paneselect vẫn chạy ổn
- Phase 06 không phải phát minh thêm runtime ids hoặc layout traversal contract mới

## Risk Assessment
- Risk: focus bug, split tree desync, overlay target sai
- Mitigation: chuyển từng path nhỏ, thêm tests snapshot/focus order

## Security Considerations
- Không để stale handle sống sau close/split rollback
- Giới hạn access bằng stable ids, không by-index

## Validation Gates
- `cargo check -p chatminal-desktop`
- `cargo test --manifest-path crates/chatminal-session-runtime/Cargo.toml`
- `rg -n "PaneId|TabId|tab_idx" apps/chatminal-desktop/src/termwindow/paneselect.rs apps/chatminal-desktop/src/termwindow/resize.rs apps/chatminal-desktop/src/spawn.rs`

## Next Steps
- Sang Phase 06 để đổi renderer/input/overlay sang session graph mới
