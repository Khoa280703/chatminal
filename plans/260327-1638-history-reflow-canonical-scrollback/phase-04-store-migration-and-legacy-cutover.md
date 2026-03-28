# Phase 04 - Store Migration And Legacy Cutover

## Context Links
- [plan.md](./plan.md)
- `crates/chatminal-store/src/schema.rs`
- `crates/chatminal-store/src/lib.rs`

## Overview
- Priority: P2
- Status: completed
- Brief: cleanup rollout sau khi canonical writer/reader đã chạy ổn; cắt dần legacy path mà không tạo trash code.

## Key Insights
- Legacy `scrollback_chunks` không có đủ metadata để migration lossless, nên cleanup phải ưu tiên safety hơn purity.
- Vì mixed-source đã được support ở Phase 03, phase này chỉ cleanup và chốt policy, không được thay semantics nữa.

## Requirements
- Functional requirements:
  1. DB cũ vẫn mở được trong giai đoạn chuyển tiếp.
  2. Legacy rows chỉ còn read-only compat, không còn writer path.
  3. Clear history / clear all data phải dọn cả canonical lẫn legacy tables.
  4. Có tiêu chí rõ để bỏ reader legacy ở release sau.
- Non-functional requirements:
  1. Migration idempotent.
  2. Không khóa app lâu khi mở DB cũ lớn.

## Architecture
- Giữ dual-read trong một giai đoạn ngắn.
- Legacy cleanup policy:
  - release N: canonical-write + dual-read
  - release N+1 hoặc sau khi đủ confidence: bỏ writer/branch dead, cân nhắc bỏ reader legacy
- Nếu cần migration offline thì tách tool riêng; không ép online migration nặng trong startup path.

## Related Code Files
- Modify:
  - `crates/chatminal-store/src/schema.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `Makefile` nếu cần clean thêm table mới
- Delete:
  - legacy writer/path dead sau khi validate xong.

## Implementation Steps
1. Audit lại toàn bộ writer path cũ và xóa dead writers.
2. Đồng bộ clear history / clear all data cho cả hai nguồn.
3. Document release policy cho dual-read window.
4. Xóa branch/compat shims không còn reachable.
5. Nếu đủ confidence, chuẩn bị PR/plan follow-up để drop reader legacy.
6. Document rollout window rõ ràng:
   - release hiện tại: `canonical-write + dual-read`
   - chỉ drop legacy reader sau khi manual reopen/resize validation pass trên DB cũ và không còn report duplicate/missing history từ session mixed-source

## Todo List
- [x] Audit dead legacy writer paths.
- [x] Sync clear commands cho canonical + legacy.
- [x] Document rollout window.
- [x] Xóa unreachable compat shims.
- [x] Chốt criteria để drop reader legacy.

## Success Criteria
- Không còn writer mới nào chạm `scrollback_chunks`.
- Cleanup commands đúng cho cả canonical và legacy.
- Không còn duplicate logic lưu history song song kéo dài vô hạn.

## Risk Assessment
- Giữ dual-read quá lâu sẽ sinh trash code.
- Cắt reader legacy quá sớm sẽ làm mất history DB cũ.
- Giảm thiểu bằng rollout window rõ ràng theo release.

## Security Considerations
- Không được reintroduce raw control sequences cũ qua migration/cleanup scripts.

## Next Steps
- Khóa chất lượng bằng tests/manual checklist ở Phase 05.

## Unresolved Questions
- Có cần migration offline riêng ở release sau hay không.
