# Phase 02 - Persistence Session Explorer State

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-01-backend-session-explorer-contract.md](phase-01-backend-session-explorer-contract.md)
- Next: [phase-03-frontend-explorer-root-flow.md](phase-03-frontend-explorer-root-flow.md)

## Overview
- Priority: P1
- Status: pending
- Effort: 3h
- Goal: Lưu explorer root theo session trong SQLite, migration an toàn với DB cũ.

## Key Insights
- `sessions` table đã là source of truth per-session; phù hợp nhất để gắn explorer root.
- Existing migration pattern dùng `PRAGMA table_info + ensure_*` helpers.

## Requirements
- Thêm cột `explorer_root` cho `sessions`.
- Existing sessions mặc định `NULL` (chưa set root).
- Load/upsert session phải đọc/ghi explorer_root.
- Không lưu explorer state theo profile/app_state key.

## Architecture
- Schema change (additive):
  - `ALTER TABLE sessions ADD COLUMN explorer_root TEXT` (nếu chưa có).
- Update structs:
  - `PersistedSession`, `SessionRecord`, và `Session` runtime mirror.
- Update flows:
  - `load_sessions_for_profile` -> hydrate explorer root.
  - `upsert_session` -> persist explorer root.

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/persistence.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`

## Implementation Steps
1. Thêm migration helper `ensure_session_explorer_root_column`.
2. Mở rộng SQL `INSERT/UPDATE/SELECT` cho `explorer_root`.
3. Mở rộng struct record/session mapping.
4. Verify restore workspace không fail với DB cũ.

## Todo List
- [ ] Add migration helper for new column.
- [ ] Extend session read/write SQL.
- [ ] Map explorer_root through hydrate and persist paths.
- [ ] Validate backward compatibility with existing DB.

## Success Criteria
- Restart app vẫn giữ root explorer đúng theo từng session.
- Session chưa set root vẫn load bình thường, explorer trả trạng thái “root chưa chọn”.

## Risk Assessment
- Risk: quên update một query gây state mismatch runtime/DB.
- Mitigation: grep toàn bộ `INSERT INTO sessions` / `SELECT ... FROM sessions` để sync đủ.

## Security Considerations
- Dữ liệu path chỉ persist sau khi đã validate ở service layer.

## Next Steps
- Kết nối UI với API mới và enforce UX “không skip” (Phase 03).

## Unresolved Questions
- None.
