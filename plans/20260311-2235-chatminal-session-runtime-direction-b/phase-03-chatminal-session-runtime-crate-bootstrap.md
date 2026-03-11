# Phase 03 - Chatminal Session Runtime Crate Bootstrap

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/Cargo.toml
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state.rs

## Overview
- Priority: P0
- Status: completed
- Brief: tạo crate live graph mới và nhốt mux vào adapter tạm

## Key Insights
- Nếu tách crate quá sớm mà chưa có facade ở desktop sẽ gây nổ call sites lớn
- Crate mới cần nhỏ, không bê toàn bộ desktop dependency graph vào ngay
- Mux ở phase này chỉ được phép tồn tại trong `EngineSurfaceAdapter`

## Requirements
- Functional: crate mới cung cấp session surface registry, session layout snapshot, event bus cơ bản
- Non-functional: compile độc lập, không phụ thuộc UI module

## Architecture
- New crate: `crates/chatminal-session-runtime`
- Initial modules:
  - `session_ids.rs`
  - `workspace_host.rs`
  - `session_surface.rs`
  - `session_snapshot.rs`
  - `session_event_bus.rs`
  - `engine_surface_adapter.rs`
- Adapter tạm sẽ wrap mux APIs cũ
- Crate mới là owner duy nhất của:
  - `surface_id`
  - `leaf_id`
  - `layout_node_id`
- Snapshot contract ban đầu phải chốt ngay:
  - snapshot là immutable value object
  - public targeting dùng stable ids, không dùng index
  - snapshot có field version hoặc compatibility marker để tránh churn silent ở Phase 05/06

## Related Code Files
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/Cargo.toml
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_ids.rs
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/engine_surface_adapter.rs
- Modify: /Users/khoa2807/development/2026/chatminal/Cargo.toml
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/Cargo.toml

## Implementation Steps
1. Thêm crate mới vào workspace
2. Define stable id types và snapshot compatibility contract trước khi migrate call sites
3. Move desktop facade từ Phase 02 vào crate mới từng phần
4. Bọc mux calls vào adapter file duy nhất
5. Giữ adapter private; desktop chỉ consume public snapshot/service API

## Todo List
- [ ] Tạo crate mới
- [ ] Define snapshot types
- [ ] Define stable id types và lifecycle semantics
- [ ] Define adapter boundary
- [ ] Cho desktop phụ thuộc crate mới

## Success Criteria
- Có crate mới build pass
- Desktop session facade dùng crate mới thay vì helper local
- Không có call site mới đụng mux ngoài adapter
- `surface_id/leaf_id/layout_node_id` không bị định nghĩa lại ở desktop/runtime crates

## Risk Assessment
- Risk: dependency cycle giữa desktop và runtime crates
- Mitigation: crate mới chỉ phụ thuộc engine/runtime primitives, không phụ thuộc desktop shell

## Security Considerations
- Không expose external API mới
- Event bus nội bộ phải tránh leak state không cần thiết

## Validation Gates
- `cargo check --manifest-path crates/chatminal-session-runtime/Cargo.toml`
- `cargo check -p chatminal-desktop`
- `rg -n "pub (struct|type) .*Id|pub enum .*Id" crates/chatminal-session-runtime/src apps/chatminal-desktop/src crates/chatminal-runtime/src`
  - expected: public stable id definitions chỉ xuất hiện trong `crates/chatminal-session-runtime/src`

## Next Steps
- Sang Phase 04 để nối business runtime với live graph crate mới
