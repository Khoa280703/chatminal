# Phase 02 - Watcher Lifecycle + Cleanup

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-01-backend-watcher-event-contract.md](phase-01-backend-watcher-event-contract.md)
- Next: [phase-03-frontend-realtime-refresh-flow.md](phase-03-frontend-realtime-refresh-flow.md)

## Overview
- Priority: P1
- Status: pending
- Effort: 3h
- Goal: Đảm bảo watcher bind/unbind đúng thời điểm, không leak khi session/root đổi hoặc app đóng.

## Key Insights
- Service đã có lifecycle points rõ: `set_active_session`, `set_session_explorer_root`, `close_session`, `shutdown_graceful`.
- UI chỉ focus một active session; MVP an toàn nhất là watcher cho active session.

## Requirements
- Stop watcher cũ trước khi bind watcher mới.
- Rebind watcher khi active session đổi.
- Rebind watcher khi root session active đổi.
- Cleanup watcher khi close session active, delete profile active, shutdown app.

## Architecture
- Trong `PtyService` thêm state watcher tập trung (Mutex<Option<...>>).
- Thêm helper nội bộ:
  - `refresh_active_explorer_watcher()`
  - `start_explorer_watcher(session_id, root_path)`
  - `stop_explorer_watcher()`
- Hook vào flow:
  - Sau `set_active_session(...)`.
  - Trong `set_session_explorer_root(...)` nếu session đó đang active.
  - Đầu `shutdown_graceful()` để stop chắc chắn.

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`

## Implementation Steps
1. Tạo watcher runtime struct chứa stop channel + join handle.
2. Implement stop logic idempotent (gọi nhiều lần vẫn an toàn).
3. Tại mỗi lifecycle hook, gọi `refresh_active_explorer_watcher()`.
4. Validate behavior khi root unset/invalid: stop watcher và không panic.

## Todo List
- [ ] Add centralized watcher runtime state.
- [ ] Add start/stop/refresh lifecycle helpers.
- [ ] Wire helpers into session/root/shutdown flows.
- [ ] Add logging cho rebind/stop failures.

## Success Criteria
- Không còn watcher cũ chạy sau khi đổi session/root.
- Close/quit không để thread treo.
- Không có panic do stop/join race.

## Risk Assessment
- Risk: deadlock nếu stop watcher trong lúc giữ lock sessions/persistence.
- Mitigation: tách lock scope nhỏ, không join thread khi giữ lock lớn.

## Security Considerations
- Không dùng shared mutable state không lock.
- Cleanup phải fail-safe, không block shutdown vĩnh viễn.

## Next Steps
- Consume event ở frontend và refresh explorer state có guard race (Phase 03).

## Unresolved Questions
- None.
