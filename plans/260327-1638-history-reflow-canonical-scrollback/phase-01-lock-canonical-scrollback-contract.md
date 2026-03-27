# Phase 01 - Lock Canonical Scrollback Contract

## Context Links
- [plan.md](./plan.md)
- `crates/chatminal-runtime/src/state.rs`
- `crates/chatminal-runtime/src/state/session_event_processor.rs`
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `crates/chatminal-store/src/lib.rs`

## Overview
- Priority: P1
- Status: pending
- Brief: định nghĩa model scrollback mới sao cho width-independent nhưng vẫn đủ thực dụng để giao với runtime/store hiện tại.

## Key Insights
- Root cause thật nằm ở persisted text đã-wrap, không nằm ở render shell.
- Không có soft-wrap metadata trong schema hiện tại, nên legacy data không thể tái tạo hoàn hảo.
- Phase đầu nên tối ưu cho shell scrollback thường; alt-screen/TUI cần scope riêng.

## Requirements
- Functional requirements:
  1. Có một canonical record format đại diện cho output history không phụ thuộc width hiện tại.
  2. Contract đủ để build preview text và restore text cho width bất kỳ.
  3. Runtime và store cùng dùng một vocabulary.
- Non-functional requirements:
  1. KISS, không biến runtime thành terminal recorder đầy đủ.
  2. Tương thích song song với legacy data trong thời gian cutover.

## Architecture
- Thêm model mới kiểu `CanonicalScrollbackSnapshot` trong `chatminal-runtime` hoặc shared boundary crate.
- Record đề xuất tối thiểu:
  - `segments`: text fragments đã normalize
  - `hard_break`: cờ newline thật
  - `seq` / `ts`
  - `trailing_fragment` cho prompt/input line chưa commit newline
- Preview/restore sẽ render từ canonical records sang text theo width được yêu cầu.

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state.rs`
  - `crates/chatminal-runtime/src/state/session_event_processor.rs`
  - `crates/chatminal-runtime/src/state/runtime_bridge.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `crates/chatminal-store/src/schema.rs`
- Create:
  - không bắt buộc; có thể tách `canonical_scrollback.rs` nếu file lớn.
- Delete:
  - chưa xóa gì ở phase này.

## Implementation Steps
1. Audit chính xác các nơi đang assume `snapshot.content` là source of truth.
2. Chốt struct canonical và serializer format.
3. Chốt rule normalize: strip volatile sequences giữ nguyên như hiện tại, nhưng không inject width-based wraps vào persisted form.
4. Chốt contract render ra preview text và restore text.
5. Document explicit non-goal cho alt-screen/TUI exact replay.

## Todo List
- [ ] Liệt kê toàn bộ read/write path của history.
- [ ] Chốt canonical struct và naming.
- [ ] Chốt boundary API cho preview/restore.
- [ ] Chốt compat strategy cho dữ liệu legacy.

## Success Criteria
- Có spec rõ để phase sau code không đoán semantics.
- Không còn ambiguity giữa hard newline thật và soft wrap do width cũ ở dữ liệu mới.

## Risk Assessment
- Rủi ro lớn nhất: chọn model quá nghèo, sau đó lại không đủ cho restore/reflow.
- Giảm thiểu: scope shell scrollback trước, alt-screen explicit non-goal.

## Security Considerations
- Giữ sanitize/strip control sequences volatile trước khi persist.
- Không mở thêm surface cho control-sequence injection từ DB restore.

## Next Steps
- Implement runtime canonical buffer writer trong Phase 02.

## Unresolved Questions
- Canonical unit nên là line/fragment hay raw output event đã normalize?
