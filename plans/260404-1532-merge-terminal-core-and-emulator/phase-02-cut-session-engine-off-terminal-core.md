# Phase 02 - Cut Session Engine Off Terminal Core

## Context Links
- [phase-01-freeze-single-terminal-contract.md](/Users/khoa2807/development/2026/chatminal/plans/260404-1532-merge-terminal-core-and-emulator/phase-01-freeze-single-terminal-contract.md)
- [leaf_runtime.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime.rs)
- [session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs)
- [session_pane.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs)

## Overview
- Priority: P1
- Status: done
- Brief: đổi active desktop/session runtime khỏi `chatminal-terminal-core` để product path chỉ còn một terminal type system.

## Key Insights
- `leaf_runtime.rs` đã dùng song song `chatminal_terminal_core::TerminalSize` và `engine_term::{Terminal, TerminalSize as IoTerminalSize}`; đây là duplicate seam rõ nhất.
- `session_host.rs`, `session_pane.rs`, `execution_bridge.rs`, `session_engine_*` đang phải widen/convert qua lại chỉ vì tồn tại hai type layers.
- Nếu phase này xong sạch, Phase 03 gần như chỉ còn Cargo/docs/delete.

## Requirements
- Replace mọi `chatminal_terminal_core::TerminalSize` active path bằng `engine_term::TerminalSize`.
- Remove conversion helpers/aliases `CoreTerminalSize` không còn cần.
- Giữ nguyên runtime behavior, scrollback, resize, input encoding, và replay semantics.

## Architecture
- `SessionEngineShared`, `LeafRuntime`, `SessionPane`, `SessionHost` cùng nói chung một `TerminalSize` type.
- `engine_term::Terminal` và `engine_term::TerminalSize` trở thành cặp canonical trong session-native runtime path.
- Không expose thêm compat alias public mới nếu chỉ để giữ code cũ compile.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime_threads.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/session_engine_shared.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/session_engine_core.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime_registry.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`
- Modify tests under `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/*tests.rs`

## Implementation Steps
1. Đổi import/type alias trong session-engine sang `engine_term::TerminalSize`.
2. Xóa các bridge conversion `CoreTerminalSize` -> `IoTerminalSize` và ngược lại; dùng một type trực tiếp.
3. Dọn test helpers khởi tạo size để không còn mix hai crate.
4. Chạy compile/test ở desktop package ngay khi phase này xong trước khi qua delete wave.

## Todo List
- [x] Session-engine chỉ còn import `engine_term::TerminalSize`
- [x] `session_host.rs` bỏ `CoreTerminalSize`
- [x] `session_pane.rs` bỏ `CoreTerminalSize`
- [x] `execution_bridge.rs` bỏ dependency `chatminal-terminal-core`
- [x] Desktop tests xanh sau migration

## Success Criteria
- `rg -n "chatminal_terminal_core::" apps/chatminal-desktop/src/desktop_host_runtime` trả về zero cho active source.
- Session-engine compile xanh mà không cần shim lâu dài.
- Behavior session spawn/resize/history không đổi.

## Risk Assessment
- Risk: đổi type đồng loạt làm rò compile errors sang nhiều module nhỏ.
- Mitigation: migrate theo lane `session_engine -> session_pane -> session_host -> tests`, compile sau mỗi lane.

## Security Considerations
- Không đổi command execution/security model.
- Cần giữ nguyên handling cho PTY resize và input forwarding để không tạo undefined behavior ở terminal process.

## Next Steps
- Sang Phase 03 để xóa crate cũ và dọn dependency graph.
