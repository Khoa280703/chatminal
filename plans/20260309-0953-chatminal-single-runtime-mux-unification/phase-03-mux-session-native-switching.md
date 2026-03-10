# Phase 03 - Mux Session Native Switching

## Context Links
- [plan.md](./plan.md)
- [terminal_chatminal_gui_proxy.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-app/src/terminal_chatminal_gui_proxy.rs)
- [chatminal_ipc_mux_domain.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-app/src/window_chatminal_gui/chatminal_ipc_mux_domain.rs)
- [termwindow/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-chatminal-desktop/src/termwindow/mod.rs)

## Overview
- Priority: P0
- Status: completed
- Brief: bỏ spawn proxy tab/process khi switch session, thay bằng session runtime native trong mux/GUI.

## Key Insights
- Overhead lớn nhất hiện tại là `Mux::spawn_tab_or_window(... proxy-desktop-session ...)` rồi đóng tab cũ.
- `proxy-desktop-session` đang làm 3 việc: activate session, bridge input/output, poll resize.
- Cả 3 việc này nên trở thành pane/runtime adapter in-process.

## Requirements
- Session switch không spawn process mới.
- Input/output/resize đi thẳng từ GUI tới runtime.
- Mux chỉ dùng như engine/window/tab/pane manager, không dùng command bridge ngoài process.

## Architecture
- Tạo embedded session domain trong GUI:
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/pane_domain.rs`
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/pane_bridge.rs`
- `TermWindow` giữ mapping `session_id -> pane/tab handle`.
- Khi switch session:
  - nếu pane đã tồn tại: focus pane đó
  - nếu chưa có: tạo pane native từ runtime handle, không spawn process ngoài

## Related Code Files
- Modify:
  - `apps/chatminal-chatminal-desktop/src/termwindow/mod.rs`
  - `apps/chatminal-chatminal-desktop/src/termwindow/render/*` nếu cần UI state mới
  - `apps/chatminal-chatminal-desktop/src/main.rs`
- Create:
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/pane_domain.rs`
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/pane_bridge.rs`
- Delete:
  - `apps/chatminal-app/src/terminal_chatminal_gui_proxy.rs`
  - `apps/chatminal-app/src/window_chatminal_gui/chatminal_ipc_mux_domain.rs`
  - `apps/chatminal-app/src/window_chatminal_gui/chatminal_ipc_mux_domain_tests.rs`
  - `apps/chatminal-app/src/window_chatminal_gui/mod.rs`

## Implementation Steps
1. Mô hình hoá session runtime handle thành pane source native cho GUI.
2. Đổi `switch_chatminal_session()` từ spawn command sang focus/create pane runtime.
3. Chuyển input/output/resize path từ proxy loop sang direct callbacks/channels.
4. Thêm test cho session switching không spawn process.

## Todo List
- [ ] Thiết kế embedded pane domain
- [ ] Bỏ proxy bridge ở session switch
- [ ] Thay resize/input/output polling bằng event path trực tiếp
- [ ] Viết test native switching

## Success Criteria
- `rg -n "proxy-desktop-session" apps/chatminal-chatminal-desktop apps/chatminal-app` chỉ còn compatibility docs/tests hoặc bằng 0.
- Session switch không tạo process mới.
- Resize không cần loop poll terminal size qua proxy bridge.

## Risk Assessment
- Risk: integration sâu với model pane/tab của Chatminal GUI.
- Mitigation: làm domain/pane bridge riêng, không rải logic qua nhiều file UI.

## Security Considerations
- Input path phải giữ backpressure policy tương đương hiện tại.
- Detach/close session phải cleanup pane/runtime handles chuẩn.

## Next Steps
- Sau phase này, desktop terminal hot path không còn phụ thuộc `apps/chatminal-app`.
