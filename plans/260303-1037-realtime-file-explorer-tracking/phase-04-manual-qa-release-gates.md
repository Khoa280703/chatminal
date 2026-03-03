# Phase 04 - Manual QA + Release Gates

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-03-frontend-realtime-refresh-flow.md](phase-03-frontend-realtime-refresh-flow.md)

## Overview
- Priority: P2
- Status: pending
- Effort: 2h
- Goal: Chốt checklist test thủ công MVP, regression gate, và docs update tối thiểu.

## Key Insights
- Feature này nhạy với race/lifecycle hơn là unit logic thuần.
- Regression chính nằm ở terminal runtime và explorer command contract đang có.

## Requirements
- Test realtime create/modify/delete/rename từ external tool.
- Test burst events không làm freeze UI.
- Test watcher cleanup qua switch session/root + close app.
- Chạy build/test command chuẩn project trước merge.

## Architecture
- QA split theo nhóm:
  - Functional realtime.
  - Performance/stability under burst.
  - Lifecycle cleanup.
  - Cross-platform smoke.
- Docs sync sau khi pass:
  - `README.md`
  - `docs/system-architecture.md`
  - `docs/project-changelog.md`

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/README.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`

## Implementation Steps
1. Run backend test/build smoke: `cargo test --manifest-path src-tauri/Cargo.toml`.
2. Run frontend build smoke: `npm --prefix frontend run build`.
3. Execute manual scenario matrix (below) và capture lỗi.
4. Update docs theo contract/event mới.

## Todo List
- [ ] Run test/build gates.
- [ ] Complete manual checklist on target OS.
- [ ] Verify no watcher leak when quitting app.
- [ ] Update docs + changelog.

## Success Criteria
- No crash/panic/leak qua toàn bộ scenario checklist.
- UI realtime đúng session, đúng root, đúng file preview.
- Docs phản ánh event contract mới.

## Risk Assessment
- Risk: không có automation cho watcher integration.
- Mitigation: checklist tay rõ ràng + lặp lại trên Linux/macOS.

## Security Considerations
- Re-verify root escape guard vẫn pass sau khi thêm watcher paths.

## Manual Scenario Matrix
1. External create/delete/rename file dưới root đang mở.
2. External rename thư mục chứa file đang expanded.
3. External modify file đang preview.
4. 500+ file changes trong 10s (script/touch loop).
5. Change root liên tục 5 lần, xác nhận chỉ root mới cập nhật.
6. Switch session A/B nhanh khi cả 2 có root khác nhau.
7. Close active session đang watch.
8. Delete active profile có session đang watch.
9. Quit app from tray -> restart -> workspace vẫn load bình thường.

## Unresolved Questions
- None.
