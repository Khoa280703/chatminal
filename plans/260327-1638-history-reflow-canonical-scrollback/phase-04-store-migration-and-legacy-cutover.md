# Phase 04 - Store Migration And Legacy Cutover

## Context Links
- [plan.md](./plan.md)
- `crates/chatminal-store/src/schema.rs`
- `crates/chatminal-store/src/lib.rs`

## Overview
- Priority: P2
- Status: pending
- Brief: thêm schema canonical, giữ compat đọc legacy trong thời gian ngắn, rồi cắt đường cũ.

## Key Insights
- Legacy `scrollback_chunks` không chứa soft-wrap metadata, nên migration lossless là không thể.
- Cách an toàn là compat-read legacy, canonical-write new data, rồi cleanup sau khi đủ tự tin.

## Requirements
- Functional requirements:
  1. App đọc được cả dữ liệu legacy và canonical trong giai đoạn chuyển tiếp.
  2. Dữ liệu mới chỉ ghi schema canonical.
  3. Có command/path clear data vẫn hoạt động bình thường.
- Non-functional requirements:
  1. Migration idempotent.
  2. Không khóa app lâu khi mở DB cũ lớn.

## Architecture
- Thêm table mới ví dụ `scrollback_records` hoặc equivalent schema canonical.
- Snapshot reader ưu tiên canonical; fallback legacy nếu session chưa có canonical data.
- Khi confidence đủ: xóa writer cũ, rồi xóa reader cũ, cuối cùng mới cân nhắc drop table legacy.

## Related Code Files
- Modify:
  - `crates/chatminal-store/src/schema.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `Makefile` nếu cần clean thêm table mới
- Delete:
  - legacy path chỉ xóa ở bước cleanup cuối phase.

## Implementation Steps
1. Add schema migration cho canonical table/index.
2. Add dual-read strategy.
3. Switch writer sang canonical-only.
4. Add cleanup step cho clear history / clear all data.
5. Sau validation, remove dead legacy writer/reader.

## Todo List
- [ ] Thêm schema migration.
- [ ] Dual-read path.
- [ ] Canonical-only writer.
- [ ] Cleanup commands sync table mới.
- [ ] Xóa dead legacy path sau validate.

## Success Criteria
- DB cũ vẫn mở được.
- Session mới không còn phụ thuộc `scrollback_chunks`.
- Không còn duplicate logic lưu history song song kéo dài quá lâu.

## Risk Assessment
- Song song 2 schema quá lâu sẽ sinh trash code.
- Giảm thiểu bằng đặt explicit cutover criteria và cleanup ngay sau validate.

## Security Considerations
- Migration không được reintroduce raw control sequences cũ vào active snapshot.

## Next Steps
- Khóa chất lượng bằng tests/manual checklist ở Phase 05.

## Unresolved Questions
- Có cần tool migration offline riêng hay online-on-open là đủ?
