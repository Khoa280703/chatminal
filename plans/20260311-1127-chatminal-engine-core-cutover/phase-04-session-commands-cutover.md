# Phase 04 - Session Commands Cutover

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine_core.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_core_ids.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_registry.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs

## Overview
- Priority: P0
- Status: in_progress
- Brief: cắt các command runtime quan trọng sang session engine thật từng lát cắt, bắt đầu bằng detached surface spawn/close không qua `mux`

## Key Insights
- Command cutover nên tách execution core trước, rồi mới wire desktop/render path; làm ngược sẽ dễ gãy UI hiện tại
- `StatefulSessionEngine` cần có allocator id riêng cho core path để tránh va chạm với id snapshot từ `mux`
- Detached surface path cho phép kiểm chứng lifecycle thật của execution core mới mà chưa phụ thuộc tab/window/pane của engine cũ

## Requirements
- Functional: session engine có command path thật cho create/close surface cơ bản mà không cần `mux`
- Non-functional: không làm gãy desktop adapter path hiện tại; test hiện có phải pass

## Architecture
- `SessionCoreIdAllocator` cấp `surface_id`/`leaf_id`/`layout_node_id` cho core path mới
- `StatefulSessionEngine::spawn_detached_surface` tạo layout single-leaf, sync `SessionCoreState`, rồi spawn `LeafRuntime` thật qua `LeafRuntimeRegistry`
- `StatefulSessionEngine::close_detached_surface` kill runtime handles và prune metadata khỏi `SessionCoreState`
- Adapter-backed `SessionEngine` trait path hiện vẫn giữ nguyên để desktop chưa bị ảnh hưởng

## Related Code Files
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_core_ids.rs
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine_core.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs

## Current Progress
- Đã thêm `SessionCoreIdAllocator` cho core-only ids
- Đã thêm `spawn_detached_surface` tạo `SessionSurfaceState` single-leaf hoàn toàn từ session engine mới
- Đã thêm `close_detached_surface` kill leaf runtimes và clear core state
- Đã tách `SessionEngineShared` để giữ `core_state`/`leaf_runtimes`/`core_ids` theo window, không bị reset mỗi call
- Đã thêm `SessionEventHub` và nối detached core path vào event stream nội bộ
- Đã thêm raw output replay cho leaf runtime để phục vụ pane attach/render phase sau
- Đã có tests xác nhận detached surface spawn/close dùng runtime mới và không dựa vào adapter
- `chatminal-runtime` không còn spawn PTY qua `SessionRuntime` ở active path; `DaemonState::{session_create,session_activate,session_input_write,session_resize,session_close}` giờ đi qua detached surface/leaf runtime của `chatminal-session-runtime`
- Đã thêm generation-aware bridge từ `SessionRuntimeEvent::{LeafOutput,LeafExited,LeafError}` quay lại `SessionEvent` để giữ nguyên business/store pipeline phía trên
- Desktop domain active hiện đã attach pane từ runtime core mới, nhưng desktop session-surface/window-host path vẫn còn dùng adapter `EngineSurfaceAdapter` để quản shell surface trong `termwindow`

## Todo List
- [x] Thêm core-only id allocator cho execution path mới
- [x] Thêm detached surface spawn/close command path qua runtime mới
- [x] Chuyển active `chatminal-runtime` command path sang detached surface/leaf runtime mới
- [ ] Migrate `ensure_session_surface` sang core path khi desktop/render side đã sẵn sàng consume surface mới
- [ ] Thêm split/focus/move command trên layout tree nội bộ thay cho adapter/mux
- [ ] Gỡ dần adapter-backed command usage khỏi `apps/chatminal-desktop`

## Success Criteria
- Session engine có command path thật đầu tiên chạy không qua `mux`
- Core path mới quản được lifecycle create/close surface + leaf runtime
- Test suite `chatminal-session-runtime` và `chatminal-desktop` vẫn xanh

## Risk Assessment
- Command cutover hiện mới dừng ở detached surface; chưa có bridge để desktop render/attach surface này
- Split/focus/move cần layout tree mutable và subscription path mới, nếu làm vội sẽ chồng hai source of truth

## Security Considerations
- Detached core path vẫn chạy command local qua PTY như runtime hiện tại; chưa thêm IPC/network boundary mới
- Id allocator nội bộ chỉ phục vụ in-process state, không lộ ra persistence boundary

## Next Steps
- Phase 04 tiếp theo: đưa `ensure_session_surface` vào core path có kiểm soát và thêm mutable layout operations cơ bản
- Phase 05: cho desktop consume event/snapshot trực tiếp từ core path mới để bỏ adapter render path
