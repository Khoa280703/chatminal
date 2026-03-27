# Phase 02 - Introduce Runtime Canonical Buffer

## Context Links
- [plan.md](./plan.md)
- [phase-01-lock-canonical-scrollback-contract.md](./phase-01-lock-canonical-scrollback-contract.md)
- `crates/chatminal-runtime/src/state/session_event_processor.rs`
- `crates/chatminal-runtime/src/state/native_api.rs`

## Overview
- Priority: P1
- Status: pending
- Brief: thay đường append history hiện tại bằng canonical writer trong runtime, trước khi nghĩ đến migration toàn bộ DB.

## Key Insights
- Nơi writer đúng hiện tại là `SessionEvent::Output` trong `session_event_processor.rs`.
- Nếu vẫn persist `chunk_text` text-based ở đây thì mọi phase sau đều chỉ là vá ngọn.

## Requirements
- Functional requirements:
  1. Output mới phải được normalize và append vào canonical buffer thay vì append thẳng text đã-wrap.
  2. `persist_history=false` path vẫn dùng `live_output` như hiện tại.
  3. `persist_history=true` path phải đi qua canonical writer duy nhất.
- Non-functional requirements:
  1. Không làm regress seq/status publishing.
  2. Không làm regress prompt dedupe / volatile sequence stripping đã fix gần đây.

## Architecture
- Tách `normalize output -> canonical append -> persistence adapter` thành một pipeline rõ ràng.
- `SessionEntry` nên giữ đủ metadata để merge trailing fragment với chunk kế tiếp khi cần.
- `Store` expose API kiểu `append_canonical_scrollback_record(...)` thay cho `append_scrollback_chunk(...)` trên path mới.

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state/session_event_processor.rs`
  - `crates/chatminal-runtime/src/state.rs`
  - `crates/chatminal-store/src/lib.rs`
- Create:
  - `crates/chatminal-runtime/src/state/canonical_scrollback.rs` nếu cần tách logic.
- Delete:
  - chưa xóa `append_scrollback_chunk`; giữ compat tạm thời.

## Implementation Steps
1. Tách helper normalize output khỏi logic broadcast event.
2. Thêm canonical append builder để nhập chunk output thành records width-independent.
3. Wires store writer mới dưới feature path chính.
4. Giữ legacy writer chỉ cho fallback đọc dữ liệu cũ, không dùng cho ghi mới.
5. Thêm unit tests cho prompt fragment / newline / trailing fragment.

## Todo List
- [ ] Tách helper canonical append.
- [ ] Wire persist path mới.
- [ ] Cover seq/status invariants.
- [ ] Test duplicate prompt + trailing fragment.

## Success Criteria
- Dữ liệu mới ghi ra DB/runtime không chứa width-dependent wraps do persistence layer tạo ra.

## Risk Assessment
- Chunk boundary có thể cắt giữa prompt/input fragment.
- Giảm thiểu bằng trailing fragment state machine nhỏ, không dựa vào regex mù.

## Security Considerations
- Sanitization vẫn chạy trước persist.
- Không lưu control sequences volatile đã biết gây title/cursor regression.

## Next Steps
- Dùng canonical buffer này để render snapshot/preview theo width hiện tại ở Phase 03.

## Unresolved Questions
- Có nên giữ line_count equivalent trong schema mới hay derive khi query preview?
