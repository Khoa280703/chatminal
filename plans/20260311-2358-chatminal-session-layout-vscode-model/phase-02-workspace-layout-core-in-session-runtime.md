# Phase 02 - Workspace Layout Core In Session Runtime

## Overview
- Priority: P0
- Status: completed
- Brief: nối workspace layout với session runtime state để desktop có source-of-truth mới cho active view/session attachment.

## Requirements
- `SessionEngineShared` hoặc state lân cận phải giữ được layout theo desktop window/workspace.
- Có API query/command cho create view, split view, attach session, close view, focus view.

## Related Code Files
- Modify: `crates/chatminal-session-runtime/src/session_engine_shared.rs`
- Modify: `crates/chatminal-session-runtime/src/session_core_state.rs`
- Modify: `crates/chatminal-session-runtime/src/runtime_bridge.rs`

## Success Criteria
- Có command/query path hoàn chỉnh cho desktop gọi mà không cần đụng `surface/leaf` public API.
