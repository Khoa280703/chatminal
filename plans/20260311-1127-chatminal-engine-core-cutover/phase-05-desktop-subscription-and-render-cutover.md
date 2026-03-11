# Phase 05 - Desktop Subscription And Render Cutover

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_event_bus.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine_shared.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/pane.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs

## Overview
- Priority: P0
- Status: in_progress
- Brief: desktop render/input sẽ subscribe trực tiếp session engine event/snapshot thay vì chỉ dựa vào runtime client + `mux` adapter snapshot

## Key Insights
- Desktop render cutover chỉ khả thi khi session engine mới phát được event/output stream ổn định và giữ shared state xuyên suốt theo window
- `ChatminalRuntimePane` hiện là reference implementation tốt cho event-driven pane ở desktop; cutover mới nên tái dùng pattern đó
- Replay raw output là điều kiện cần để pane mới seed terminal state chính xác khi attach muộn hơn thời điểm spawn

## Current Progress
- Đã có `SessionEventHub` + `SessionEventSubscription` trong `chatminal-session-runtime`
- `SessionEngineShared` giờ giữ shared runtime resources theo window: core state, registry, id allocator, event hub, leaf runtime event channel
- Leaf runtime event đã được map sang `SessionRuntimeEvent::{LeafOutput,LeafExited,LeafError}`
- Detached surface core path publish được `SurfaceAttached`
- Leaf runtime giữ raw output replay và `ChatminalSessionPane` đã dùng path này để seed terminal state khi attach
- `ChatminalRuntimeDomain` active path giờ tạo `ChatminalSessionPane` trực tiếp từ `DaemonState` execution core attachment (`surface_id + leaf_id + SessionEngineShared`), không còn render active pane qua `RuntimeEvent + snapshot` path cũ
- `ChatminalRuntimePane` cũ hiện chỉ còn giữ làm test/reference path; binary thường không còn dùng làm active consumer
- `chatminal_session_surface` không còn giữ `SessionEngineShared` riêng theo window; shell helper giờ lấy thẳng shared execution state từ `DaemonState`, giúp desktop host path và execution core nhìn cùng một session graph
- `ChatminalSessionPane` giờ dùng `pane_id == leaf_id` khi có thể, để các đường script/action cũ nhìn thấy session leaf identity thay vì pane id tương thích tạm
- Một số active-path action (`spawn` source pane resolution, pane select, mouse focus, activate pane by index/direction) đã chuyển từ `tab -> session_id` lookup sang `active_session_id` của session layer
- `TermWindow::{active_session_id,active_surface_tab,active_surface_id,active_leaf_id}` giờ ưu tiên session lookup thật thay vì chỉ bám sidebar snapshot hoặc `Tab` metadata
- `TermWindow::active_pane_id()` trong session mode giờ trả `leaf_id` của session layer, không còn ưu tiên host pane id
- close routing (`close_chatminal_session_for_tab`) giờ resolve session từ session lookup + host tab resolution, không còn đọc `session_id` trực tiếp từ `Tab`
- `get_tab_information` giờ build session tab bridge từ `snapshot.sessions + session lookup + session_surface_tab`, thay vì quét `window.iter()` rồi suy session ngược từ từng `Tab`
- `PaneInformation.leaf_id` giờ đọc từ pane metadata khi có, thay vì luôn suy bằng `pane_id`
- `active_surface_id()` và `TabInformation.surface_id/tab_id` trong session mode giờ ưu tiên `chatminal_surface_id` từ pane metadata thật, giảm lệ thuộc host tab identity

## Todo List
- [x] Dựng event hub/subscription nội bộ cho session engine mới
- [x] Giữ shared session-engine state theo desktop window thay vì tạo mới mỗi call
- [x] Lưu raw output replay cho leaf runtime
- [x] Tạo desktop pane consume `SessionEventHub` trực tiếp
- [x] Seed pane từ replay output khi attach muộn
- [x] Bỏ lệ thuộc `ChatminalRuntimePane` khỏi session surface path active
- [ ] Chuyển `focus_or_spawn_chatminal_session_surface` sang consumer path mới

## Risks
- Desktop hiện vẫn render qua `Mux::Tab/Panes`; muốn bỏ hẳn cần bridge host mới hoặc pane consumer mới đủ tương thích
- Nếu seed pane bằng replay không đầy đủ, có thể sinh lệch state so với runtime thật; cần verify kỹ bằng smoke sau khi wire

## Next Steps
- Dùng `ChatminalSessionPane` cho một luồng desktop kiểm soát được trước khi cắt adapter path cũ
- Sau đó chuyển `focus_or_spawn_chatminal_session_surface` sang consumer path mới
