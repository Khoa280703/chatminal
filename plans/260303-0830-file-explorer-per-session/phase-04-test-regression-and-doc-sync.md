# Phase 04 - Test, Regression, and Docs Sync

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-03-frontend-explorer-root-flow.md](phase-03-frontend-explorer-root-flow.md)

## Overview
- Priority: P1
- Status: pending
- Effort: 2h
- Goal: Chốt chất lượng bằng test runtime + checklist regression + cập nhật docs.

## Key Insights
- Repo hiện chưa có frontend test runner; gate thực tế là Rust tests + frontend build + manual scenario.

## Requirements
- Có test cho path guard và persistence explorer_root.
- Không regression profile/session hiện tại.
- Cập nhật docs contract mới cho explorer per-session.

## Architecture
- Backend tests (Rust):
  - validate root path reject invalid/outside root
  - persistence migration/additive column compatibility
  - restore session giữ explorer_root
- Frontend verification:
  - build pass
  - manual flow matrix theo requirement chốt

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/persistence.rs`
- Modify `/home/khoa2807/working-sources/chatminal/README.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`

## Implementation Steps
1. Thêm/điều chỉnh Rust tests cho persistence + service explorer guards.
2. Chạy `cargo test --manifest-path src-tauri/Cargo.toml`.
3. Chạy `npm --prefix frontend run build`.
4. Manual regression checklist:
   - set root mandatory
   - per-session isolation
   - no cwd follow
   - profile switch isolation
5. Update docs + changelog.

## Todo List
- [ ] Add/update Rust tests for explorer root behavior.
- [ ] Execute cargo test and frontend build.
- [ ] Run manual acceptance checklist.
- [ ] Update docs and changelog.

## Success Criteria
- Test/build pass.
- 4 requirement chốt đều pass trong manual regression.
- Docs phản ánh contract mới và behavior “user-driven root change”.

## Risk Assessment
- Risk: test setup khó do service/persistence coupling.
- Mitigation: ưu tiên unit test ở layer helper/persistence, tránh test PTY runtime path.

## Security Considerations
- Validate lại path-boundary rules trước khi release.

## Next Steps
- Ready for implementation execution.

## Unresolved Questions
- None.
