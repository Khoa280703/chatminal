# Phase 03 - Desktop Session Host Bootstrap

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/domain.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/session_pane.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/pane.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/mod.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine_shared.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_event_bus.rs

## Overview
- Priority: P0
- Current status: pending
- Brief: tạo desktop host path mới để một `session surface` được attach/render như thực thể gốc, không cần bọc trong `mux::Tab`; desktop render boundary vẫn có thể tạm dùng `mux::Pane` compatibility object nếu cần.

## Key Insights
- Đây là điểm thay đổi nhận thức hệ thống: desktop không còn “mượn host tab để hiện terminal”
- `ChatminalSessionPane` đã có pattern event-driven tốt; cần nâng nó từ consumer phụ thành host chính
- Theo code hiện tại, `ChatminalSessionPane` vẫn là `mux::Pane` compatibility object cho render loop; phase này bỏ `host tab`, không bắt buộc bỏ `Pane` trait ngay
- Bootstrap tốt ở phase này sẽ làm các phase `termwindow` phía sau chủ yếu là đổi routing, không phải đổi engine nữa

## Requirements
- Functional:
  - desktop attach được một `session surface` trực tiếp từ session engine shared state
  - mỗi visible leaf có pane consumer đúng với `leaf_id`
  - late attach vẫn seed được state từ replay output
- Non-functional: không làm đổi behavior terminal core, selection, scrollback, input pipeline

## Architecture
- Tạo host abstraction mới ở desktop, ví dụ `DesktopSessionHost` hoặc tương đương, chịu trách nhiệm:
  - bind `session_id + surface_id`
  - tạo/bỏ pane consumers theo layout snapshot
  - subscribe `SessionEventHub`
  - phản chiếu active leaf/layout changes cho `termwindow`
- `chatminal_session_surface.rs` từ bridge `session <-> host tab` chuyển thành bridge `desktop window <-> session host`
- `ChatminalRuntimePane` cũ giữ lại tạm cho compatibility/reference, không còn là active host path

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/domain.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/session_pane.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/mod.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/*

## Implementation Steps
1. Định nghĩa desktop session host abstraction mới
2. Chuyển `focus_or_spawn_chatminal_session_surface` và helper tương đương sang host mới
3. Khi attach surface, tạo pane consumers từ layout snapshot thay vì `Tab.iter_panes()`
4. Đồng bộ active leaf, visible leaf list, close lifecycle qua `SessionEventHub`
5. Seed terminal state từ replay output + session snapshots
6. Đảm bảo invalidation/redraw chỉ dùng session events

## Todo List
- [ ] Có host abstraction desktop cho session-native surface
- [ ] `chatminal_session_surface` không còn trả `Arc<Tab>` cho active path
- [ ] Pane consumers attach từ layout snapshot thật
- [ ] Redraw/invalidation bám event hub

## Success Criteria
- Desktop mở/chuyển session mà không cần lookup host tab
- `chatminal_session_surface.rs` active path không còn map `session -> Tab`
- Các thao tác attach lại surface không duplicate hay split-brain state
- Desktop vẫn render ổn định dù boundary render tạm thời còn dùng `ChatminalSessionPane: mux::Pane`

## Risk Assessment
- Risk: chưa đủ host abstraction để `termwindow` tiêu thụ dễ dàng
- Mitigation: giữ shim adapter cục bộ trong phase này, chỉ phục vụ `termwindow` migration kế tiếp

## Security Considerations
- Không thêm persistence mới; chỉ đổi host bootstrap trong process

## Next Steps
- Phase 04 sẽ cắt toàn bộ `termwindow` routing sang source of truth session host mới
