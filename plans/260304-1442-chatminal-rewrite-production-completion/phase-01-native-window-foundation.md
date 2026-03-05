# Phase 01 - Native Window Foundation

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/docs/system-architecture.md](/home/khoa2807/working-sources/chatminal/docs/system-architecture.md)

## Overview
- Priority: P1
- Status: Completed
- Mục tiêu: chuyển `chatminal-app` từ CLI/TUI scaffold sang native window app tối thiểu, vẫn dùng daemon-first.

## Key Insights
- Dashboard/attach hiện hữu đủ để debug runtime nhưng chưa là UX production.
- Nên giữ terminal-first, không mở rộng feature phụ ở phase này.

## Requirements
- Functional:
1. Mở app bằng window native.
2. Hiển thị sidebar session + active pane.
3. Chuyển session, attach session, nhập lệnh cơ bản.
- Non-functional:
1. Startup nhanh (<2s local warm state).
2. Không crash khi resize liên tục.

## Architecture
- Window shell + input loop trong `chatminal-app`.
- Reuse protocol client + pane registry + wezterm core adapter.
- UI state machine tách khỏi transport để test unit.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs`
3. `/home/khoa2807/working-sources/chatminal/README.md`
4. `/home/khoa2807/working-sources/chatminal/Makefile`
- Create:
1. `apps/chatminal-app/src/window/native_window_wezterm.rs`
2. `apps/chatminal-app/src/window/native_window_wezterm_controller.rs`
3. `apps/chatminal-app/src/window/native_window_wezterm_actions.rs`
- Delete:
1. `dashboard-*` command tạm sau khi window flow ổn định (deprecate rồi xóa ở phase 05)

## Implementation Steps
1. Tạo window bootstrap + render tick loop.
2. Ghép workspace hydrate vào state window.
3. Ghép input routing (key/paste/resize) vào session active.
4. Thêm action switch session và tạo session.
5. Thêm smoke tests cho state reducer.

## Todo List
- [x] Tạo module `window` cho native window baseline.
- [x] Hiển thị sidebar + active pane trong window.
- [x] Wire switch/create session + send input từ UI.
- [x] Debounce resize + cache render để giảm jank/freeze baseline.
- [x] Thêm test reducer/state machine riêng cho window flow.
- [x] Bổ sung GUI e2e headless (xvfb) cho smoke automation.

## Success Criteria
- App window mở được, attach session đang chạy, gõ lệnh nhận output realtime.
- Session switch không mất state pane.
- Không còn phụ thuộc dashboard read-only cho use case chính.

## Risk Assessment
- Risk: chọn renderer sai gây complexity tăng nhanh.
- Mitigation: ưu tiên renderer tối thiểu, không theme-heavy.

## Security Considerations
- Không mở IPC network path.
- Validate mọi input UI trước khi gửi daemon command.

## Next Steps
- Bàn giao sang Phase 02 để tối ưu hot path daemon.

## Unresolved questions
- None.
