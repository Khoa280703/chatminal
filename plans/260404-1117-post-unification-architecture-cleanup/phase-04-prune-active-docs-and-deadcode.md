---
phase: 04
status: pending
priority: medium
effort: low
risk: low
---

# Phase 04: Prune Active Docs And Deadcode

## Overview
Sau khi phases 01-03 xong, dọn active docs và dead helpers/tests đã mất lý do tồn tại.

## Why This Phase Exists
- Active docs hiện khá dài và còn lẫn nhiều narrative của các wave cũ.
- Sau khi compat/history cleanup xong sẽ xuất hiện dead helpers/tests/comments mới.
- Đây là phase low-risk để chốt lại source tree gọn hơn.

## Scope
- `docs/system-architecture.md`
- `docs/codebase-summary.md`
- `docs/project-changelog.md`
- helper/tests/comments được unlock bởi phases 01-03

## Requirements
- Chỉ dọn active docs scope.
- Không rewrite archive/history docs chỉ để đổi vocabulary hoặc làm đẹp.

## Implementation Steps
1. Rút gọn active docs để phản ánh current architecture thay vì lưu toàn bộ chiến sử cleanup trong file active.
2. Xóa helper/tests comments chỉ còn phục vụ compat/history path đã cắt.
3. Giữ roadmap/changelog/archive như lịch sử; chỉ sửa nếu chúng nói sai active reality.
4. Chốt “intentional keeps” ngắn gọn để lần sau không mở lại tranh luận low-ROI.

## Done Criteria
- Active docs ngắn hơn và phản ánh đúng reality sau phases 01-03.
- Dead helper/test/comment unlocked bởi cleanup đã bị xóa.
- Archive/history docs không bị churn không cần thiết.

## Risk / Tradeoff
- Risk: dọn docs quá mạnh làm mất forensic context.
- Tradeoff: active docs phải tối ưu cho current contributors; history để ở roadmap/changelog/archive.
