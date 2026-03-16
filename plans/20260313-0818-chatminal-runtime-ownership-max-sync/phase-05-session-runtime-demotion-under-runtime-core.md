# Phase 05 - Session Runtime Demotion Under Runtime Core

## Context Links
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `crates/chatminal-runtime/src/state/runtime_lifecycle.rs`
- `crates/chatminal-session-runtime/src/session_engine.rs`
- `crates/chatminal-session-runtime/src/engine_runtime_adapter.rs`
- `crates/chatminal-session-runtime/src/workspace_host.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: `chatminal-session-runtime` trở thành execution subsystem nội bộ của runtime core, không còn bị desktop coi như application layer ngang cấp.

## Key Insights
- Đây không nhất thiết là merge crate; mục tiêu là dependency direction một chiều.
- `chatminal-runtime` phải là orchestrator, `chatminal-session-runtime` là worker/execution engine.

## Requirements
- Đảm bảo desktop không gọi trực tiếp application semantics từ `chatminal-session-runtime`.
- Gom session-runtime orchestration entrypoints vào `chatminal-runtime`.
- Chuẩn hóa mapping `session_id -> runtime_id -> terminal_instance_id` chỉ qua runtime core.

## Architecture
- `chatminal-runtime` sở hữu `SessionEngineShared` / execution bridge lifecycle.
- `chatminal-session-runtime` expose engine primitives + adapters, không own app lifecycle contract.

## Related Code Files
- Refactor: `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- Refactor: `crates/chatminal-runtime/src/state/runtime_lifecycle.rs`
- Refactor: `crates/chatminal-session-runtime/src/session_engine.rs`
- Refactor: `crates/chatminal-session-runtime/src/engine_runtime_adapter.rs`
- Refactor: `crates/chatminal-session-runtime/src/workspace_host.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`

## Implementation Steps
1. Audit direct desktop references tới `chatminal-session-runtime`.
2. Move orchestration entrypoints về `chatminal-runtime`.
3. Simplify `chatminal-session-runtime` public API về execution concerns.
4. Verify desktop chỉ còn nhận execution effects qua runtime facade.

## Todo List
- [x] Desktop không còn lộ trực tiếp `SessionRuntimeLookup/SessionRuntimeState` ở desktop-facing path
- [x] Desktop không gọi application semantics từ session-runtime
- [x] Runtime core own execution bridge lifecycle
- [x] Session-runtime public API được thu gọn
- [x] Runtime/session integration tests pass

## Success Criteria
- Dependency direction rõ: runtime core -> session runtime, không ngược, không chéo qua desktop.
- Session runtime trở thành lớp dưới của app core.

## Risk Assessment
- Risk: đụng nhiều test và adapter path.
- Mitigation: migrate theo facade từng nhóm API, giữ adapter contract ổn định cho host runtime.

## Security Considerations
- Không được làm runtime handle sống ngoài ownership của runtime core.

## Next Steps
- Phase complete.
