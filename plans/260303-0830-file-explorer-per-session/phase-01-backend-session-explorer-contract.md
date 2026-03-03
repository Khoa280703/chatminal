# Phase 01 - Backend Session Explorer Contract

## Context Links
- Plan: [plan.md](plan.md)
- Next: [phase-02-persistence-session-explorer-state.md](phase-02-persistence-session-explorer-state.md)
- Runtime docs: `/home/khoa2807/working-sources/chatminal/README.md`

## Overview
- Priority: P1
- Status: pending
- Effort: 4h
- Goal: Tạo API backend cho explorer root theo session, tách hoàn toàn khỏi terminal `cwd`.

## Key Insights
- `Session` hiện chỉ có `cwd`, chưa có explorer state.
- Tauri command layer (`main.rs`) chưa expose command nào cho file explorer.
- `chatminal-cwd-sync` đang đồng bộ `cwd`; không được reuse cho explorer root.

## Requirements
- Có command set root theo `session_id`, validate path là directory hợp lệ.
- Có command đọc explorer state/session.
- Có command list entries trong phạm vi root.
- Nếu chưa set root: explorer API trả lỗi rõ ràng, không fallback `cwd`.

## Architecture
- Thêm model payload/response trong `src-tauri/src/models.rs`:
  - `SetSessionExplorerRootPayload { session_id, root_path }`
  - `SessionExplorerState { session_id, root_path }`
  - `ListExplorerEntriesPayload { session_id, relative_path }`
- Thêm service methods trong `src-tauri/src/service.rs`:
  - `set_session_explorer_root`
  - `get_session_explorer_state`
  - `list_session_explorer_entries`
- Path guard:
  - canonicalize root + target
  - reject target outside root
  - reject non-directory root

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/models.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/main.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`

## Implementation Steps
1. Định nghĩa contract mới trong `models.rs`.
2. Expose command mới trong `main.rs` + `generate_handler!`.
3. Bổ sung field explorer root vào struct `Session`.
4. Viết logic validate/canonical path và scope guard.
5. Đảm bảo không có code path nào derive root từ `session.cwd`.

## Todo List
- [ ] Add explorer payload/response models.
- [ ] Add tauri commands for set/get/list explorer state.
- [ ] Add path-scope validation helper.
- [ ] Return explicit error when root not set.

## Success Criteria
- Command set/get/list explorer hoạt động với session hợp lệ.
- Root chỉ đổi khi gọi command set root.
- CWD sync worker không động vào explorer root.

## Risk Assessment
- Risk: path canonicalization khác nhau theo OS.
- Mitigation: dùng canonical path và normalize string trước compare.

## Security Considerations
- Chặn traversal (`..`) qua canonical check.
- Không trả file metadata ngoài root.

## Next Steps
- Persist explorer root vào DB (Phase 02).

## Unresolved Questions
- None.
