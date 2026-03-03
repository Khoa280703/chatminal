# Phase 03 - Frontend Realtime Refresh Flow

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-02-watcher-lifecycle-cleanup.md](phase-02-watcher-lifecycle-cleanup.md)
- Next: [phase-04-manual-qa-release-gates.md](phase-04-manual-qa-release-gates.md)

## Overview
- Priority: P1
- Status: pending
- Effort: 3h
- Goal: Nhận event realtime từ backend và refresh explorer tree/file đúng session, không race.

## Key Insights
- App đã có pattern `listen(...)` + unlisten trong `onDestroy`.
- Explorer đã có cache dirs (`explorerTreeEntriesByDir`) + open file preview flow.
- Cần giữ guard `activeSessionId`/seq để drop stale async work.

## Requirements
- Thêm listener cho event `explorer/fs-changed`.
- Chỉ xử lý event của active session hiện tại.
- Debounce/throttle refresh ở frontend để tránh invoke burst.
- Nếu file đang mở bị đổi, auto re-read preview (không ghi đè state session sai).

## Architecture
- Types (`frontend/src/lib/types.ts`):
  - Add `SessionExplorerFsChangedEvent`.
- App (`frontend/src/App.svelte`):
  - Add `unlistenExplorerFsChanged`.
  - Add scheduler `queueExplorerRealtimeRefresh(payload)` để coalesce event.
  - Refresh strategy MVP:
    - `full_resync=true` -> reload root + expanded dirs.
    - else -> reload parent dirs liên quan changed paths.
  - Nếu `open_file_path` bị ảnh hưởng -> `openExplorerFile(..., { persistState: false })`.

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/frontend/src/lib/types.ts`
- Modify `/home/khoa2807/working-sources/chatminal/frontend/src/App.svelte`

## Implementation Steps
1. Add event type và listener trong `setupEventListeners`.
2. Add refresh queue/debounce state (timer + revision guard).
3. Implement selective dir refresh, fallback full refresh.
4. Update `onDestroy` cleanup unlisten + clear timer.

## Todo List
- [ ] Add explorer fs event type.
- [ ] Add listener with active-session filter.
- [ ] Add debounced refresh queue.
- [ ] Add opened-file preview auto-refresh.

## Success Criteria
- Explorer UI cập nhật gần realtime khi file system đổi ngoài app.
- Không nhảy state sai khi user đổi session trong lúc event đến.
- Không tạo vòng lặp persist state không cần thiết.

## Risk Assessment
- Risk: race giữa refresh async và user actions (expand/open/change root).
- Mitigation: dùng session check + refresh sequence, drop stale promises.

## Security Considerations
- Frontend không tự trust path ngoài payload normalized.
- Không auto-open file mới ngoài root/session hiện tại.

## Next Steps
- Manual QA + release gates + docs sync (Phase 04).

## Unresolved Questions
- None.
