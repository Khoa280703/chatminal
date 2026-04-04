---
phase: 04
status: completed
priority: medium
effort: low
risk: low
---

# Phase 04: Sync Active Docs And Prune Dead Surface

## Overview
Đồng bộ active docs với code reality sau phases 01-03, đồng thời xóa dead helpers/comments/tests bị unlock bởi cleanup. Trọng tâm là active docs, không rewrite lịch sử dự án.

## Findings Covered
- Finding 4: docs drift ở `system-architecture.md` và `codebase-summary.md`
- Dead/duplicate surface unlocked bởi findings 1, 2, 5, 6 sau khi code cutover xong

## Scope
- `docs/system-architecture.md`
- `docs/codebase-summary.md`
- `docs/project-changelog.md`
- helper/tests/comments dead được mở khóa bởi phases trước

## Requirements
- Active docs phải phản ánh current architecture, không trộn narrative cũ mâu thuẫn với code hiện tại
- Chỉ sửa changelog/roadmap khi chúng nói sai active reality hoặc cần ghi nhận closeout đáng kể
- Không churn archive/history docs vô ích

## Architecture
- Docs active là entrypoint cho contributor hiện tại, nên phải ưu tiên current truth hơn forensic timeline dài dòng
- Dead helper/comment chỉ bị xóa sau khi code path canonical đã chốt, tránh dọn quá sớm rồi phải reintroduce

## Related Code Files
- Modify:
  - `docs/system-architecture.md`
  - `docs/codebase-summary.md`
  - `docs/project-changelog.md` nếu cần
  - tests/comments/helpers unlocked bởi phases 01-03

## Implementation Steps
1. Rewrite gọn các section active đang nói sai về ownership, `MuxHandle`, `Weak<HostRuntimeRoot>`, compat aliases, và desktop facade reality.
2. Remove hoặc re-scope comments/tests/helpers chỉ còn bám narrative cũ.
3. Ghi rõ intentional keeps còn lại để tránh reopen debt low-ROI.
4. Chốt docs links/cross-references sau cleanup.

## Todo List
- [x] Fix `docs/system-architecture.md`
- [x] Fix `docs/codebase-summary.md`
- [x] Update changelog nếu closeout đủ lớn
- [x] Remove dead comments/tests/helpers unlocked by previous phases
- [x] Add short intentional-keeps note if any compat seam remains

## Success Criteria
- Active docs không còn claim mâu thuẫn với code hiện tại
- Contributors đọc docs không còn bị dẫn về ownership/compat story cũ
- Dead docs/comments/helpers unlocked bởi cleanup đã được dọn

## Risk Assessment
- Risk thấp; chủ yếu là mất forensic context nếu rewrite quá mạnh
- Mitigation: giữ timeline ở changelog/plan archive, giữ active docs ngắn và đúng

## Security Considerations
- Không có security impact trực tiếp

## Next Steps
- Phase 05 verify toàn bộ repo và khóa grep guards
