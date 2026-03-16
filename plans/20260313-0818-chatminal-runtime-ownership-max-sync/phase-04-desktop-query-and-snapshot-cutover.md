# Phase 04 - Desktop Query And Snapshot Cutover

## Context Links
- `apps/chatminal-desktop/src/frontend.rs`
- `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs`
- `apps/chatminal-desktop/src/chatminal_render/mod.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_state_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_positioned_session_helpers.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: desktop chỉ đọc product state từ một snapshot/subscription stream duy nhất thay vì ghép từ nhiều lớp.

## Key Insights
- Mutation đã qua runtime core nhưng query còn phân tán thì vẫn chưa đồng bộ thật.
- Desktop phải render từ một `desktop snapshot` ổn định, không tự tổng hợp `workspace + runtime + host` ở nhiều nơi.

## Requirements
- Chuẩn hóa snapshot cho desktop gồm:
  - active profile
  - sessions của profile hiện tại
  - active session
  - workspace layout + active view
  - runtime render mapping cần thiết cho session views
- Desktop subscription chỉ còn một surface công khai.
- Loại bỏ desktop-side ad-hoc recompute của product state.
- Xóa ownership persistence local của desktop cho workspace layout; `chatminal_layout/workspace_store.rs` phải bị delete hoặc hạ xuống cache thuần đọc/ghi qua runtime facade.

## Architecture
- `chatminal-runtime` cung cấp snapshot application-level.
- Desktop dùng snapshot để render sidebar/session bar/layout state.
- Engine render data chỉ được join ở boundary rõ ràng, không lẫn ownership product state.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/frontend.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_render/mod.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- Refactor: `apps/chatminal-desktop/src/tabbar.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/mod.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_state_helpers.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_positioned_session_helpers.rs`
- Refactor: `crates/chatminal-runtime/src/api/mod.rs`

## Implementation Steps
1. Định nghĩa desktop snapshot contract.
2. Migrate frontend/sidebar/session bar sang snapshot này.
3. Migrate termwindow query helpers khỏi direct multi-source lookup.
4. Gỡ `persist_layout/load_persisted_layout` khỏi desktop ownership path.
5. Xóa local caches/state trùng ownership nếu có.

## Todo List
- [x] Desktop snapshot contract batch đầu đã có `workspace + layout`
- [x] Frontend/sidebar render từ snapshot duy nhất
- [x] Termwindow query helpers không còn product recompute rải rác
- [x] Desktop không còn tự persist workspace layout ngoài runtime facade
- [x] Build/test cho snapshot refresh và active focus

## Success Criteria
- Desktop render product state từ một nguồn công khai duy nhất.
- Không còn query path vừa đọc runtime core vừa đọc host adapter để ra business state.

## Risk Assessment
- Risk: snapshot quá nặng hoặc invalidation quá nhiều.
- Mitigation: tách rõ product snapshot và render-only detail; cache ở runtime core thay vì desktop.

## Security Considerations
- Snapshot không được làm lộ state ngoài active profile/workspace context.

## Next Steps
- Phase complete.
