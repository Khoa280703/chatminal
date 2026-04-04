---
phase: 03
status: completed
priority: high
effort: medium
risk: high
---

# Phase 03: Retire Legacy Scrollback Read Path

## Overview
Đưa scrollback/history steady-state về canonical-only read path. `scrollback_chunks` legacy nếu còn giữ chỉ được giữ như migration/forensics seam hữu hạn, không còn nằm trong mỗi lần rebuild logical snapshot.

## Closeout
- Steady-state read path đã canonical-only
- Legacy chunks còn lại chỉ là migration residue/backfill-once seam cho DB cũ

## Findings Covered
- Finding 3: canonical scrollback read-path vẫn merge legacy `scrollback_chunks`

## Scope
- `crates/chatminal-runtime/src/state/canonical_scrollback.rs`
- `crates/chatminal-runtime/src/state/*` callers build/load snapshot
- `crates/chatminal-store/src/lib.rs`
- `crates/chatminal-store/src/schema.rs`
- runtime/store tests cho scrollback persistence

## Requirements
- Read path thường xuyên phải dựa trên canonical records duy nhất
- Legacy chunks nếu còn được giữ thì phải nằm ngoài steady-state read path
- Migration strategy phải explicit: eager backfill, one-shot repair, hoặc drop-legacy-after-cutover

## Architecture
- Canonical store: `scrollback_records`
- Legacy store: `scrollback_chunks` chỉ còn là migration/repair/archive seam nếu thật sự cần
- `build_logical_snapshot(...)` không đọc từ 2 storage models trong steady state nữa

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state/canonical_scrollback.rs`
  - `crates/chatminal-store/src/lib.rs`
  - runtime/store tests liên quan
- Possible delete/prune:
  - helper/query legacy read path không còn caller sau cutover
  - schema cleanup follow-up nếu phase chốt drop table/index trong wave này

## Implementation Steps
1. Audit writer path để xác nhận canonical records đã đủ dữ liệu cho restore/reflow/history preview.
2. Chọn migration strategy explicit cho `scrollback_chunks`:
   - backfill-once rồi đọc canonical-only
   - repair tool riêng rồi steady-state read canonical-only
   - hoặc keep table archive nhưng runtime không đọc mặc định
3. Cập nhật `build_logical_snapshot(...)` để bỏ dual-read steady-state.
4. Dọn store helpers/tests/metrics theo strategy đã chọn.
5. Verify với history restore, resize/reflow, prefix/prompt-sensitive cases, và persisted Claude/full-screen style outputs.

## Todo List
- [x] Audit canonical writer completeness
- [x] Chốt migration strategy cho legacy chunks
- [x] Remove dual-read from logical snapshot build
- [x] Update runtime/store tests for canonical-only steady state
- [x] Decide retain vs drop `scrollback_chunks` schema in this wave

## Success Criteria
- `build_logical_snapshot(...)` không còn merge legacy chunks trên every snapshot rebuild
- History restore/reflow vẫn đúng behavior sau restart/resize
- Legacy chunks không còn là hidden source of truth thứ hai

## Risk Assessment
- Risk: làm gãy restore của các session cũ hoặc các case output/prompt khó
- Mitigation: phase này bắt buộc có verification matrix riêng cho persisted sessions, resize/reflow, và full-screen TUIs/text outputs

## Security Considerations
- Không có auth risk trực tiếp; nhưng migration phải tránh mất dữ liệu scrollback của user

## Next Steps
- Sau cutover mới dọn docs/history narrative an toàn ở phase 04
