# Phase 06 - Adapter Bypass And Active Path Removal

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/engine_surface_adapter.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs

## Overview
- Priority: P0
- Current status: pending
- Brief: khi desktop và runtime core đã đi hoàn toàn bằng session-native path, loại `EngineSurfaceAdapter` khỏi active path và thu hẹp nó xuống compatibility/test shim hoặc xóa hẳn.

## Key Insights
- Đây là phase chứng minh mình đã thực sự bỏ host tab, không chỉ đổi route ở ngoài
- Chỉ được bắt đầu phase này khi grep gates ở desktop/runtime active path đã gần về zero
- Nếu Phase 06 làm quá sớm, app sẽ gãy attach/focus/render đồng loạt
- Bỏ adapter ở đây nghĩa là bỏ `EngineSurfaceAdapter`/`host surface` bridge của session flow; không đồng nghĩa gỡ luôn mọi `mux::Pane` compatibility object khỏi desktop render loop

## Requirements
- Functional:
  - active desktop path không instantiate `ChatminalEngineSurfaceAdapter`
  - `ChatminalMuxSessionEngine` không còn là type active chính
  - `chatminal_session_surface` không còn phụ thuộc `Arc<Tab>` ở active path
- Non-functional: giữ test-only shim nếu cần để tránh phá quá nhiều một lúc

## Architecture
- Tách `SessionEngine` thành core-native implementation rõ ràng
- Nếu cần, chuyển adapter cũ vào module `compat` riêng với feature/test guard
- Loại helper `host_surface_*` khỏi active API; thay bằng `surface_*` thật

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- Delete or move: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/engine_surface_adapter.rs

## Implementation Steps
1. Chuyển desktop construction path sang core-native engine type
2. Bỏ `ChatminalMuxSessionEngine` khỏi active wiring
3. Refactor/xóa helper `host_surface_for_session`, `host_surface_for_public_surface`, `host_surface_id_for_public_surface`
4. Giữ adapter cũ ở test/compat module riêng nếu vẫn cần tạm thời
5. Chạy grep gate chứng minh active path không còn adapter

## Todo List
- [ ] Active desktop path không tạo adapter
- [ ] `chatminal_session_surface` không còn host-surface bridge ở active flow
- [ ] Adapter cũ bị xóa hoặc chuyển sang compat-only slice
- [ ] Grep gate adapter pass

## Success Criteria
- `EngineSurfaceAdapter|ChatminalEngineSurfaceAdapter` biến mất khỏi active runtime path
- Session core là implementation thật duy nhất mà desktop dùng
- Không còn `host tab` trong mental model của active runtime
- Nếu còn `mux` ở desktop sau phase này thì nó chỉ còn ở render/notification compatibility boundary, không còn ở session execution path

## Risk Assessment
- Risk: một số test/unit helper đang generic over adapter
- Mitigation: giữ shim test adapter riêng, không kéo lại vào active code

## Security Considerations
- Không thay đổi trust boundary; chỉ xóa lớp compatibility nội bộ

## Next Steps
- Phase 07 sẽ prune dependency graph và xóa code chết còn lại
