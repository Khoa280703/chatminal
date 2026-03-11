# Chatminal Engine Core Cutover

Status: In Progress
Goal: thay execution core `chatminal-mux` bằng Chatminal session engine thật, giữ desktop/runtime app-facing ổn định trong suốt migration.

## Phases
- Phase 01 - Session Engine Boundary: dựng facade `SessionEngine` và cắt desktop khỏi adapter private hiện tại
- Phase 02 - In-Process Session Core State: thêm session-engine state/store nội bộ cho surface/leaf/layout/process registry
- Phase 03 - Leaf PTY Runtime Bootstrap: tạo leaf runtime thật với PTY/parser/state thay cho `mux::Pane`
- Phase 04 - Session Commands Cutover: chuyển focus/spawn/split/move/close sang session engine thật
- Phase 05 - Desktop Subscription And Render Cutover: desktop consume event/snapshot trực tiếp từ session engine
- Phase 06 - Mux Removal: xóa `chatminal-mux` dependency khỏi active runtime path và dọn code cũ

## Progress
- Phase 01: completed
- Phase 02: completed
- Phase 03: completed
- Phase 04: in_progress
- Phase 05: in_progress
- Phase 06: pending

## Key Dependencies
- `chatminal-runtime` tiếp tục giữ business/session/profile/store ownership
- `chatminal-session-runtime` trở thành execution core thật
- `apps/chatminal-desktop` chỉ nói qua session-engine facade
- terminal parser/render core không đổi behavior trong migration này

## Done When
- desktop app không còn gọi trực tiếp engine adapter kiểu `mux` nữa
- session engine có thể tự spawn/focus/split/close mà không cần `chatminal-mux`
- `crates/chatminal-mux` có thể gỡ khỏi runtime graph active
