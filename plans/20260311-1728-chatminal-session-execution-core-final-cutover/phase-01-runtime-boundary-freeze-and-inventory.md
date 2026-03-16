# Phase 01 - Runtime Boundary Freeze And Inventory

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/engine_surface_adapter.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/frontend.rs

## Overview
- Priority: P0
- Current status: pending
- Brief: đóng băng boundary migration, đếm chính xác mọi callsite active path còn dựa vào `mux/tab/pane`, phân loại thành nhóm command, render, action, overlay, frontend, startup.

## Key Insights
- Vấn đề hiện tại không còn là naming public; vấn đề là execution source of truth
- Nếu không inventory sạch ngay từ đầu sẽ dễ migrate thiếu một nhóm callsite rồi phải vá vòng sau
- `EngineSurfaceAdapter` là marker ranh giới quan trọng: mọi chỗ còn đi qua nó là active path chưa session-native

## Requirements
- Functional: có bản đồ callsite hoàn chỉnh cho active runtime path
- Functional: inventory phải có line refs/owner phase cụ thể, không chỉ danh sách file
- Non-functional: không sửa behavior ở phase này; chỉ freeze kiến trúc và checklist migration

## Architecture
- Chia callsite còn sót thành 6 bucket:
  1. Session engine core commands
  2. Desktop session host/bootstrap
  3. TermWindow routing
  4. Overlay/frontend action routing
  5. Pane/render/update notifications
  6. Startup/dependency/wiring
- Định nghĩa rõ active path và compatibility path

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/plans/20260311-1728-chatminal-session-execution-core-final-cutover/plan.md
- Modify: /Users/khoa2807/development/2026/chatminal/plans/20260311-1728-chatminal-session-execution-core-final-cutover/phase-01-runtime-boundary-freeze-and-inventory.md

## Implementation Steps
1. Grep toàn bộ active path cho `EngineSurfaceAdapter`, `Mux::get`, `get_tab`, `get_pane`, `spawn_tab_or_window`, `move_pane_to_new_tab`, `focus_pane_and_containing_tab`
2. Loại trừ `third_party/`, test-only slices và engine-private crates không nằm trong active desktop flow
3. Gắn từng callsite vào bucket ownership cụ thể
4. Ghi inventory theo line refs/file refs cho từng callsite active; đánh dấu rõ callsite nào là render/compat slice còn lại ngoài phạm vi phase
5. Viết acceptance checklist cho từng bucket để phase sau chỉ việc đốt checklist
6. Xác nhận các file nào sau cùng phải bị xóa hoàn toàn, file nào chỉ cần refactor

## Callsite Inventory

### Bucket 1 — Session Engine Core Commands (Phase 02)
**File: `crates/chatminal-session-runtime/src/engine_surface_adapter.rs`** (toàn bộ file)
- L91,97,126,140,152,192,205,207,210,227,228,252,253,276,277,301,302,314,341,359,360 — `Mux::get()` + `get_tab()` calls
- L153 — `spawn_tab_or_window`
- L239 — `focus_pane_and_containing_tab`
- L317,320 — `move_pane_to_new_tab`
- L329 — `focus_pane_and_containing_tab`

**File: `crates/chatminal-session-runtime/src/session_engine.rs`**
- L275 — `pub type ChatminalMuxSessionEngine = StatefulSessionEngine<ChatminalEngineSurfaceAdapter>`
- L277–285 — `impl ChatminalMuxSessionEngine { host_surface_for_session, host_surface_session_id }`
- L280 — `Mux::get().get_tab(host_surface_id)`

**File: `crates/chatminal-session-runtime/src/session_spawn_manager.rs`**
- L17 — generic over `EngineSurfaceAdapter`

**File: `crates/chatminal-session-runtime/src/session_focus_manager.rs`**
- L10,19,28,38,51 — generic over `EngineSurfaceAdapter`

### Bucket 2 — Desktop Session Host/Bootstrap (Phase 03)
**File: `apps/chatminal-desktop/src/chatminal_session_surface.rs`**
- L27–31 — `session_engine()` creates `ChatminalMuxSessionEngine` with adapter → active creation path
- L80–82 — `host_surface_for_session()` returns `Arc<Tab>` in active API
- L84–95 — `host_surface_for_public_surface()` returns `Arc<Tab>`
- L97–101 — `host_surface_id_for_public_surface()`
- L68–78 — `session_id_for_host_surface()` — lookup từ Tab identity

**File: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`**
- L55 — `Mux::get()` in domain registration

### Bucket 3 — TermWindow Routing (Phase 04)
**File: `apps/chatminal-desktop/src/termwindow/mod.rs`**
- L331,335,789,810,832,935,937,1149,1263,1951,1953,2052,2178,2193,2219,2253,2278,2455,2606,2621,2623,2666,2893,2928,2964,2981,3101,3185,3300,3756,3773,3808,3817,3962,3986,4020,4056,4064,4216,4389,4433 — `Mux::get()`, `get_tab()`, `get_pane()`

**File: `apps/chatminal-desktop/src/termwindow/paneselect.rs`**
- L166,232,264,270 — `Mux::get()`, `move_pane_to_new_tab`, `focus_pane_and_containing_tab`

**File: `apps/chatminal-desktop/src/termwindow/resize.rs`** — L319

**File: `apps/chatminal-desktop/src/termwindow/clipboard.rs`** — L48–49

### Bucket 4 — Overlay/Frontend Action Routing (Phase 05)
**File: `apps/chatminal-desktop/src/frontend.rs`**
- L41,59,83,84,107,243,251,332,349,434,460,488 — `Mux::get()`, `focus_pane_and_containing_tab`, `spawn_tab_or_window`

**File: `apps/chatminal-desktop/src/overlay/launcher.rs`** — L78
**File: `apps/chatminal-desktop/src/overlay/confirm_close_pane.rs`** — L30,50,69
**File: `apps/chatminal-desktop/src/overlay/quickselect.rs`** — L941–942
**File: `apps/chatminal-desktop/src/overlay/copy.rs`** — L122

### Bucket 5 — Pane/Render/Update Notifications (KEEP — render compat)
- `chatminal_runtime/session_pane.rs:172,180,189,195,312,316,320` — `Mux::get().notify()`, `record_input_for_current_identity`
- `chatminal_runtime/pane.rs:210,214,219,227,361,366,371` — same
- `termwindow/render/paint.rs:254` — `record_focus_for_current_identity`

### Bucket 6 — Startup/Dependency/Wiring (Phase 07)
- `apps/chatminal-desktop/src/main.rs:178,230,261,295,352,449`
- `apps/chatminal-desktop/src/update.rs:114`
- `apps/chatminal-desktop/src/spawn.rs:48,132,146`

## Active vs Compatibility
- **Active path**: Buckets 1, 2, 3, 4, 6 — phải migrate
- **Compatibility path**: Bucket 5 — giữ nguyên cho render loop

## Todo List
- [x] Hoàn tất callsite inventory có bucket ownership
- [x] Inventory có line refs cho từng active callsite
- [x] Đánh dấu active path vs compatibility path
- [x] Freeze danh sách file target cho từng phase
- [x] Chốt grep gates dùng lại ở mọi phase sau

## Success Criteria
- Không còn tranh luận mơ hồ “đã bỏ tab chưa”
- Mỗi callsite `mux/tab/pane` trong active path đều có owner phase rõ ràng
- Mỗi callsite active đều có line ref để verify phase completion không phụ thuộc raw grep-zero
- Không có edit code runtime thật ở phase này

## Risk Assessment
- Risk: bỏ sót callsite ẩn trong `termwindow` hoặc `frontend`
- Mitigation: dùng grep theo API cụ thể, không grep theo từ khóa chung chung

## Security Considerations
- Không có thay đổi runtime behavior

## Next Steps
- Chuyển ngay sang Phase 02 sau khi inventory xong và không còn bucket mơ hồ
