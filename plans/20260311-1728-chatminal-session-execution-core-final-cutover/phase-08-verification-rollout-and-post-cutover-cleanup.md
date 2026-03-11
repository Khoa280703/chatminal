# Phase 08 - Verification Rollout And Post-Cutover Cleanup

## Context Links
- /Users/khoa2807/development/2026/chatminal/README.md
- /Users/khoa2807/development/2026/chatminal/docs/system-architecture.md
- /Users/khoa2807/development/2026/chatminal/docs/codebase-summary.md
- /Users/khoa2807/development/2026/chatminal/docs/project-changelog.md

## Overview
- Priority: P1
- Current status: pending
- Brief: chạy verification cuối, đóng migration, dọn app data dev nếu cần, rồi đồng bộ tài liệu ở mức cần thiết sau khi code thật đã chuyển xong.

## Key Insights
- Phase này chỉ có ý nghĩa khi Phase 01-07 đã qua toàn bộ gate kỹ thuật
- User đang ưu tiên code trước docs; vì vậy docs sync chỉ làm sau khi migration đã thực sự hoàn tất
- Cần tách rõ validation hard gate và cleanup dev-data để tránh trộn lỗi code với state cũ

## Requirements
- Functional:
  - full regression pass
  - session switching/spawn/split/close smoke pass
  - app mở lại không dựa vào host tab state cũ
- Non-functional: docs chỉ cập nhật phần phản ánh kiến trúc thật đã xong; không thêm speculative docs

## Architecture
- Validation gồm 4 lớp:
  1. Grep proof
  2. Build/test proof
  3. Runtime smoke proof
  4. Optional doc sync proof

## Related Code Files
- Modify if needed: /Users/khoa2807/development/2026/chatminal/README.md
- Modify if needed: /Users/khoa2807/development/2026/chatminal/docs/system-architecture.md
- Modify if needed: /Users/khoa2807/development/2026/chatminal/docs/codebase-summary.md
- Modify if needed: /Users/khoa2807/development/2026/chatminal/docs/project-changelog.md

## Implementation Steps
1. Chạy grep gates cuối cho adapter/mux/tab/pane ở active runtime path
2. Chạy build/test matrix
3. Chạy manual smoke: create session, switch session, split leaf, close leaf, reopen app, restore state
4. Nếu cần cho dev cycle mới, clear app data cũ sau khi đã xác nhận code pass
5. Cập nhật README/docs tối thiểu để phản ánh execution core mới
6. Viết checklist hậu migration và các việc còn lại thuộc engine-private low-ROI cleanup

## Todo List
- [ ] Grep gates cuối pass
- [ ] Build/test gates cuối pass
- [ ] Manual smoke pass
- [ ] App data dev được clear có kiểm soát nếu cần
- [ ] Docs phản ánh đúng kiến trúc mới nếu code đã chốt

## Success Criteria
- Có thể nói chính xác: active runtime path chỉ còn `session`, không còn `host tab`
- Repo build/test ổn định sau cleanup
- Không còn cần giải thích “session facade over tab” cho desktop runtime nữa

## Risk Assessment
- Risk: test pass nhưng manual smoke lộ race khi attach/detach session host
- Mitigation: luôn chạy smoke chuyển session nhiều lần và reopen app sau cleanup data

## Security Considerations
- Nếu clear app data dev, phải làm có chủ đích và chỉ sau khi user đồng ý trong turn triển khai tương ứng

## Next Steps
- Sau khi plan này hoàn tất, cleanup sâu hơn nữa chỉ còn nằm ở engine-private crates và không còn ảnh hưởng đến active product architecture
