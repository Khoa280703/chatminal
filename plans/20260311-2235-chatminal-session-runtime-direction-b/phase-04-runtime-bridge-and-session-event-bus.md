# Phase 04 - Runtime Bridge And Session Event Bus

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state/runtime_lifecycle.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state/session_event_processor.rs

## Overview
- Priority: P0
- Status: completed
- Brief: business runtime trở thành owner của live session graph qua bridge chính thức

## Key Insights
- Store/business lifecycle và live surface lifecycle hiện đang trộn trong `DaemonState`
- Đây là phase chuyển ownership, không phải phase UI
- Sau phase này desktop không nên tự spawn/focus session surface bằng logic ad-hoc nữa

## Requirements
- Functional: session create/activate/close/history clear phải đi qua runtime bridge mới
- Non-functional: giữ contract cũ cho CLI/TUI compatibility trong lúc migrate

## Architecture
- `chatminal-runtime` giữ metadata/workspace/store
- `chatminal-session-runtime` giữ live graph + event stream
- Bridge chịu trách nhiệm:
  - reconcile stored session state với live surface state
  - publish event hợp nhất cho desktop/UI
- Source-of-truth rules:
  - `chatminal-runtime` authoritative cho `active_session_id` ở mức workspace/business
  - `chatminal-session-runtime` authoritative cho `active_surface_id` và `active_leaf_id` trong session đang attach
  - bridge là nơi duy nhất được phép commit chuyển đổi giữa `active_session_id` và live focus
  - khi mismatch:
    - startup/restore: runtime chọn session, bridge yêu cầu session runtime attach/focus đúng surface
    - user focus leaf trong cùng session: session runtime update leaf focus, bridge publish unified event nhưng không đổi session
    - user switch session: runtime đổi `active_session_id` trước, sau đó bridge mới focus live surface tương ứng
    - close active surface: bridge chọn session kế tiếp theo runtime policy rồi mới publish final active state

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state/runtime_lifecycle.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state/session_event_processor.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/client.rs

## Implementation Steps
1. Thêm bridge object từ runtime sang session runtime crate
2. Chuyển create/activate/close live surface khỏi desktop ad-hoc path
3. Hợp nhất active session event với live focus event
4. Giữ backward compatibility cho passive workspace load
5. Viết test cho startup restore, mismatch reconcile, close race, focus-within-session

## Todo List
- [x] Add runtime bridge
- [x] Route session lifecycle through bridge
- [x] Add unified event publication
- [x] Preserve CLI/TUI compatibility
- [x] Lock deterministic ordering cho session switch và close flows

## Success Criteria
- Session lifecycle có một owner rõ ràng
- Desktop không còn trực tiếp giữ live surface lifecycle policy
- Runtime tests pass cho session create/activate/close cơ bản
- Có test chứng minh không còn split-brain giữa `active_session_id` và `active_surface_id`

## Risk Assessment
- Risk: split-brain giữa store active session và live focused surface
- Mitigation: define single reconcile point trong bridge

## Security Considerations
- Cẩn thận race khi close session đang có output pending
- Không làm mất data store/history do reorder event

## Validation Gates
- `cargo test -p chatminal-runtime`
- `cargo test --manifest-path crates/chatminal-session-runtime/Cargo.toml`
- `cargo test --manifest-path apps/chatminald/Cargo.toml`

## Next Steps
- Sang Phase 05 để chuyển split/focus/spawn graph semantics
