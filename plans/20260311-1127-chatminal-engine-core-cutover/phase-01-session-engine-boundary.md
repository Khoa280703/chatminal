# Phase 01 - Session Engine Boundary

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/engine_surface_adapter.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_focus_manager.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_spawn_manager.rs

## Overview
- Priority: P0
- Status: completed
- Brief: tạo facade execution-core mới để desktop không còn biết adapter cụ thể nào đang đứng sau session runtime

## Key Insights
- Desktop hiện còn new trực tiếp `ChatminalEngineSurfaceAdapter`, đây là coupling chặn mọi migration engine thật về sau
- `SessionFocusManager` và `SessionSpawnManager` đã là orchestration primitives tốt; phase này chỉ cần bọc chúng trong một facade ổn định hơn
- Phase này chưa thay execution core, chỉ thay boundary để phase sau có chỗ cắm engine mới

## Requirements
- Functional: thêm `SessionEngine` facade đủ để desktop query/focus/spawn/move/close session surface mà không biết adapter implementation
- Non-functional: không đổi behavior runtime hiện tại; build/test hiện có phải pass

## Architecture
- `SessionEngine` là high-level contract cho app layer
- `ChatminalMuxSessionEngine` là implementation tạm thời, bọc `ChatminalEngineSurfaceAdapter`
- Desktop helpers trong `chatminal_session_surface.rs` chỉ được instantiate facade mới

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/engine_surface_adapter.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs

## Implementation Steps
1. Define `SessionEngine` trait and result model for desktop-facing operations
2. Implement `ChatminalMuxSessionEngine` using existing adapter/focus/spawn managers
3. Replace direct adapter construction in desktop helper module with engine facade construction
4. Add unit tests for facade behavior with fake adapter or fake engine internals
5. Run compile/test gates

## Todo List
- [x] Add `SessionEngine` facade module
- [x] Add mux-backed facade implementation
- [x] Cut desktop helper module to facade
- [x] Add/adjust tests
- [x] Run gates

## Success Criteria
- `apps/chatminal-desktop/src/chatminal_session_surface.rs` no longer constructs `ChatminalEngineSurfaceAdapter` directly
- `SessionEngine` facade is the only desktop-facing execution-core entrypoint
- Existing tests still pass

## Risk Assessment
- Risk: facade becomes just a rename with no leverage
- Mitigation: facade must own desktop-facing operations, not just expose raw adapter

## Security Considerations
- Không làm thay đổi process spawning semantics ở phase này
- Không thay đổi persistence/session ownership boundary

## Next Steps
- Sau phase này bắt đầu dựng in-process session core state song song với mux-backed engine
