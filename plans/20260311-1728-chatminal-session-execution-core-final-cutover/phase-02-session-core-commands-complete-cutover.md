# Phase 02 - Session Core Commands Complete Cutover

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine_core.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_layout_tree.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_registry.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_focus_manager.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_spawn_manager.rs

## Overview
- Priority: P0
- Current status: pending
- Brief: làm cho `chatminal-session-runtime` tự xử lý toàn bộ command path cốt lõi mà không cần adapter `mux`/host-tab cho active desktop flow.

## Key Insights
- Đây là phase quan trọng nhất; nếu command path chưa session-native thì mọi desktop cutover chỉ là veneer
- Detached surface path đã tồn tại; cần mở rộng từ detached -> attached/live desktop surfaces
- Split/focus/move/close phải mutate cùng một layout tree và cùng một registry runtime, không được lai nửa core nửa adapter

## Requirements
- Functional:
  - session engine tự spawn attached surface
  - tự focus surface/leaf
  - tự split leaf
  - tự move leaf sang surface mới/window mới theo model session
  - tự close leaf/surface và dọn runtime handles
- Non-functional: giữ raw output replay và event stream ổn định cho desktop attach muộn

## Architecture
- `StatefulSessionEngine` trở thành command owner thật
- `SessionCoreState` giữ source of truth cho:
  - session -> surface mapping
  - surface -> layout tree
  - active leaf / active node
  - leaf runtime handles / generations
- `SessionFocusManager` và `SessionSpawnManager` không còn generic over adapter cho active path; adapter chỉ còn optional shim/test path
- Bổ sung command set mới nếu thiếu:
  - `spawn_attached_surface`
  - `split_leaf`
  - `close_leaf`
  - `close_surface`
  - `move_leaf_to_surface`
  - `move_leaf_to_window`
  - `resize_surface_if_needed`

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine_core.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_focus_manager.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_spawn_manager.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_layout_tree.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_registry.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs
- Delete later: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/engine_surface_adapter.rs

## Implementation Steps
1. Hoàn tất attached surface spawn path dùng core ids + leaf runtime thật
2. Chuyển `ensure_session_surface` sang core path cho active desktop consumer
3. Thêm mutable layout operations cho split/move/close/focus trong `SessionCoreState`
4. Emit đầy đủ `SessionEvent` cho attach/detach/output/focus/layout mutation
5. Giữ snapshot builder thuần từ core state, không build từ host tab nữa
6. Viết test cho từng command operation và generation safety

## Todo List
- [ ] `ensure_session_surface` không còn gọi adapter cho active path
- [ ] split/focus/move/close đi qua core state thật
- [ ] layout snapshot lấy hoàn toàn từ session core
- [ ] event stream phản ánh đúng mọi mutation
- [ ] test command matrix xanh

## Success Criteria
- `chatminal-session-runtime` có thể điều khiển lifecycle session surface độc lập với `mux` host-tab path
- Grep active path không còn `spawn_tab_or_window`, `move_pane_to_new_tab`, `focus_pane_and_containing_tab` trong crate này
- `cargo test -p chatminal-session-runtime` pass

## Risk Assessment
- Risk: layout mutation gãy focus invariants hoặc runtime handle cleanup
- Mitigation: test ma trận single-leaf, split 2 leaf, move, close active leaf, close last leaf

## Security Considerations
- Mọi command local shell vẫn đi qua PTY runtime hiện có; không mở thêm IPC surface mới

## Next Steps
- Khi command core ổn định, Phase 03 sẽ thay host bootstrap của desktop sang session-native pane host
