# Phase 03 - Frontend Explorer Root Flow

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-02-persistence-session-explorer-state.md](phase-02-persistence-session-explorer-state.md)
- Next: [phase-04-test-regression-and-doc-sync.md](phase-04-test-regression-and-doc-sync.md)

## Overview
- Priority: P1
- Status: pending
- Effort: 5h
- Goal: UX explorer bắt buộc set root theo từng session, state tách profile/cwd.

## Key Insights
- `App.svelte` đang quản state session và cache theo `session_id`; có thể reuse pattern map keyed session.
- `session.cwd` hiện hiển thị cho terminal metadata; không được dùng làm root explorer mặc định.

## Requirements
- Mở explorer ở session chưa có root -> bắt buộc chọn root trước khi browse.
- Root explorer đổi bằng action user “Change root”, không auto.
- State explorer map theo `session_id`; switch profile không làm lẫn root giữa session.
- Không có logic follow `cwd` terminal.

## Architecture
- Mở rộng type contract trong `frontend/src/lib/types.ts`.
- Trong `App.svelte`:
  - map state explorer keyed by `session_id`
  - panel/set-root form cho session chưa có root
  - invoke command set/get/list explorer
- Khi đổi session:
  - load explorer state theo session đó
  - render tree chỉ trong phạm vi root

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/frontend/src/lib/types.ts`
- Modify `/home/khoa2807/working-sources/chatminal/frontend/src/App.svelte`

## Implementation Steps
1. Thêm type explorer payload/response.
2. Thêm UI block “Select root folder” (không skip).
3. Bind explorer state per-session bằng map keyed by `session_id`.
4. Gọi backend để set root và load tree entries.
5. Thêm explicit button “Change root” cho user-driven update.
6. Audit code để chắc không còn fallback từ `session.cwd`.

## Todo List
- [ ] Add frontend explorer types.
- [ ] Implement mandatory root selection UI.
- [ ] Bind explorer state cache by session id.
- [ ] Add explicit change-root action.
- [ ] Remove any cwd-coupled explorer fallback.

## Success Criteria
- Session A/B có root explorer riêng, switch qua lại không lẫn.
- Đổi `cwd` terminal không đổi view explorer.
- Explorer luôn yêu cầu root cho session chưa set.

## Risk Assessment
- Risk: race condition khi switch session nhanh trong lúc load explorer.
- Mitigation: guard response bằng `activeSessionId` trước khi apply state.

## Security Considerations
- Frontend không tự canonical path; mọi validate nằm backend.

## Next Steps
- Chạy test/regression + sync docs (Phase 04).

## Unresolved Questions
- None.
