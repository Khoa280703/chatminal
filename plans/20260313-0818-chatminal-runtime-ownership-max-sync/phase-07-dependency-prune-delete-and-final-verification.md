# Phase 07 - Dependency Prune Delete And Final Verification

## Context Links
- `Cargo.toml`
- `apps/chatminal-desktop/Cargo.toml`
- `crates/chatminal-runtime/src/*`
- `crates/chatminal-session-runtime/src/*`
- `apps/chatminal-desktop/src/*`

## Overview
- Priority: P0
- Status: completed
- Brief: delete code chết sau cutover, dọn dependency graph, khóa các gate chứng minh kiến trúc đã đồng bộ tối đa.

## Key Insights
- Chỉ khi delete được helper/boundary cũ và grep gate sạch theo đúng scope mục tiêu thì mới gọi là complete.
- Phase này là nơi chống overclaim.

## Requirements
- Xóa helper/callsite không còn dùng sau facade + snapshot cutover.
- Dọn imports/dependencies chéo không còn hợp lệ.
- Chốt grep/build/test gates cho ownership direction.
- Chỉ mark complete nếu desktop product path không còn bypass `chatminal-runtime` facade.

## Architecture
- Sau phase này, dependency direction phải nhìn rõ từ compile graph và call graph.

## Related Code Files
- Refactor: `Cargo.toml`
- Refactor: `apps/chatminal-desktop/Cargo.toml`
- Refactor: `crates/chatminal-runtime/src/*`
- Refactor: `crates/chatminal-session-runtime/src/*`
- Refactor: `apps/chatminal-desktop/src/*`
- Delete: dead desktop/runtime helpers phát sinh sau Phase 02-06

## Implementation Steps
1. Delete forwarding/helper paths đã vô dụng.
2. Prune dependency/import graph.
3. Chạy grep gates cho ownership bypass còn sót:
   - desktop import trực tiếp `chatminal_session_runtime`
   - desktop direct mutation/query call vào `chatminal-runtime` internals ngoài facade wrappers
   - host adapter ownership logic còn dính `active_profile`, `active_session`, `workspace_layout`
4. Chạy full build/test gates.
5. Mark plan complete chỉ khi mọi gate xanh.

## Todo List
- [x] Dead helper paths deleted
- [x] Dependency/import graph pruned
- [x] Ownership grep gates sạch theo scope mục tiêu
- [x] Full build/test gates pass

## Success Criteria
- Desktop thin client thật sự.
- Runtime core là application orchestrator thật sự.
- Session runtime là execution subsystem thật sự.
- Host runtime là engine adapter thật sự.

## Risk Assessment
- Risk: bỏ sót dead path ít dùng khiến compile graph vẫn sạch nhưng behavior chưa sạch.
- Mitigation: grep theo cả symbol ownership và mutation verbs; giữ desktop/runtime tests bắt buộc.

## Security Considerations
- Không delete nhầm path còn phục vụ compatibility command quan trọng nếu chưa được migrate.

## Next Steps
- Plan complete.
