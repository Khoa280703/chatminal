# Phase 02 - Runtime Facade Consolidation In `chatminal-runtime`

## Context Links
- `crates/chatminal-runtime/src/lib.rs`
- `crates/chatminal-runtime/src/state.rs`
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-runtime/src/state/protocol_adapter.rs`
- `crates/chatminal-runtime/src/server.rs`
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `crates/chatminal-runtime/src/state/runtime_lifecycle.rs`
- `crates/chatminal-runtime/src/state/session_event_processor.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: gom product mutations/queries/subscriptions thành facade application-level duy nhất trong `chatminal-runtime`.

## Key Insights
- Nếu desktop còn phải biết `runtime_id`, `terminal_instance_id`, hoặc `workspace_layout` mutation details thì kiến trúc chưa đồng bộ.
- `chatminal-runtime` phải expose đúng API app cần, không hơn, không kém.

## Requirements
- Tạo/chuẩn hóa facade cho các nhóm API:
  - workspace snapshot/load/save/clear
  - profile create/switch/rename/delete
  - session create/activate/close/clone/persist-history
  - session-view/layout split/focus/close/attach
  - terminal focus/move/close trong ngữ cảnh session
  - desktop subscription/snapshot refresh
- Giữ `chatminal-session-runtime` là internal dependency của facade này.
- Định nghĩa DTO/runtime-owned API types nếu hiện tại desktop-facing path còn lộ trực tiếp `chatminal-session-runtime` ids/snapshots không cần thiết.

## Architecture
- `DaemonState`/runtime state là façade public.
- `state/*` là implementation modules.
- Desktop chỉ gọi `chatminal-runtime` facade qua `apps/chatminal-desktop/src/chatminal_runtime/*`.

## Related Code Files
- Refactor: `crates/chatminal-runtime/src/lib.rs`
- Refactor: `crates/chatminal-runtime/src/state.rs`
- Refactor: `crates/chatminal-runtime/src/state/native_api.rs`
- Refactor: `crates/chatminal-runtime/src/state/protocol_adapter.rs`
- Refactor: `crates/chatminal-runtime/src/server.rs`
- Refactor: `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- Refactor: `crates/chatminal-runtime/src/state/runtime_lifecycle.rs`
- Refactor: `crates/chatminal-runtime/src/state/session_event_processor.rs`
- Refactor: `crates/chatminal-runtime/src/api/mod.rs`

## Implementation Steps
1. Chốt public facade methods và input/output types.
2. Di chuyển product mutation logic về `chatminal-runtime`.
3. Định nghĩa runtime-owned desktop DTOs ở nơi cần để giảm lộ type nội bộ execution.
4. Đảm bảo `workspace_layout` persistence theo profile được điều khiển duy nhất từ đây.
5. Chuẩn hóa subscription/event surface cho desktop và daemon/protocol path.

## Todo List
- [x] Facade methods được định nghĩa đủ cho desktop layout/session mutation batch đầu
- [x] Product mutations workspace layout đã được gom về runtime core
- [x] Desktop-facing facade không lộ type session-runtime ngoài scope cần thiết
- [x] Runtime subscriptions thống nhất cho desktop
- [x] Test unit cho facade mới

## Success Criteria
- Desktop đã có thể dùng một facade duy nhất cho product logic.
- `chatminal-runtime` là owner rõ ràng của profile/session/layout lifecycle.

## Risk Assessment
- Risk: façade quá bé khiến desktop còn phải bypass.
- Mitigation: Phase 01 inventory là checklist bắt buộc trước khi đóng phase.

## Security Considerations
- Mọi API mutation phải giữ validation hiện có cho session/profile ids và persistence.

## Next Steps
- Phase complete.
