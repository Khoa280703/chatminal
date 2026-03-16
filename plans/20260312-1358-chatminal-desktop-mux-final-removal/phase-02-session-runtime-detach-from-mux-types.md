# Phase 02 - Session Runtime Detach From Mux Types

## Context Links
- `crates/chatminal-session-runtime/src/lib.rs`
- `crates/chatminal-session-runtime/src/session_layout_tree.rs`
- `crates/chatminal-session-runtime/src/engine_runtime_adapter.rs`
- `crates/chatminal-session-runtime/src/session_engine.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: biến `chatminal-session-runtime` thành crate first-party thuần, không import `mux::*` trong active code.

## Key Insights
- Hiện `session_layout_tree` và `engine_runtime_adapter` vẫn bám trực tiếp `mux::Tab`/`Pane`.
- Đây là choke point; không cắt từ đây thì desktop không thể sạch compile graph.

## Requirements
- `session_layout_tree` chỉ dùng snapshot first-party, không parse `PaneNode`.
- `EngineRuntimeAdapter` đổi contract từ host-tab/pane sang first-party engine facade.
- `SessionEngine` không expose `host_tab_for_session`.

## Architecture
- Tạo `terminal_instance_id` first-party thay cho `LeafId` ở desktop-facing path.
- Tạo `EngineSessionFacade` trả `RuntimeSnapshot`, `RenderSnapshot`, `InstanceSnapshot`.
- Tách module chuyển đổi engine-private sang snapshot first-party vào crate desktop hoặc engine-private adapter riêng.

## Related Code Files
- Refactor: `crates/chatminal-session-runtime/src/lib.rs`
- Refactor: `crates/chatminal-session-runtime/src/session_layout_tree.rs`
- Refactor: `crates/chatminal-session-runtime/src/engine_runtime_adapter.rs`
- Refactor: `crates/chatminal-session-runtime/src/session_engine.rs`
- Refactor: `crates/chatminal-session-runtime/src/session_ids.rs`
- Delete or rewrite: mọi helper `host_tab_*`, `pane_*` trong crate này

## Implementation Steps
1. Freeze id model mới: `TerminalInstanceId`, `RenderNodeId`.
2. Đổi `session_layout_tree` sang pure first-party snapshot builder.
3. Đổi adapter trait để không lộ `Tab`/`Pane`/`PaneId`.
4. Xoá helpers `host_tab_for_session`, `host_tab_session_id`, `pane_metadata_*`.
5. Cập nhật tests và grep gates.

## Todo List
- [x] Add first-party ids
- [x] Remove `mux` imports khỏi session-runtime
- [x] Rewrite layout snapshot builder
- [x] Rewrite engine adapter contract
- [x] Fix tests

## Success Criteria
- `rg -n "use mux::|mux::" crates/chatminal-session-runtime/src` trả về zero active lines.
- Session runtime compile/test pass mà không cần `mux` dependency.

## Risk Assessment
- Risk: engine adapter quá gắn với `termwindow`.
- Mitigation: đẩy engine-private glue xuống desktop render layer, không giữ ở session-runtime.

## Security Considerations
- Không đổi persistence format nếu chưa cần; nếu đổi id serialized thì phải có migration rõ ràng.

## Next Steps
- Sang Phase 03 để desktop có render model first-party thay thế `mux::Tab`.
