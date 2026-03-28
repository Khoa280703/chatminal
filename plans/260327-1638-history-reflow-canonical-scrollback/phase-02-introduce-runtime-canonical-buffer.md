# Phase 02 - Introduce Runtime Canonical Buffer

## Context Links
- [plan.md](./plan.md)
- [phase-01-lock-canonical-scrollback-contract.md](./phase-01-lock-canonical-scrollback-contract.md)
- `crates/chatminal-runtime/src/state/session_event_processor.rs`
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-store/src/lib.rs`
- `crates/chatminal-store/src/schema.rs`

## Overview
- Priority: P1
- Status: completed
- Brief: thêm schema canonical tối thiểu và route persist mới từ runtime output sang canonical records.

## Key Insights
- Nếu chưa có schema canonical ở phase này thì reopen path vẫn bị khóa ở legacy text cũ.
- Writer đúng hiện tại vẫn là `SessionEvent::Output`, nhưng rollout còn phải cover `session_set_persist()` flush path.

## Requirements
- Functional requirements:
  1. Có canonical table/index tối thiểu trong store.
  2. Output mới đi qua reducer rồi persist thành canonical records.
  3. `session_set_persist()` flush live output cũng phải đi qua canonical writer, không đi legacy writer.
  4. Legacy rows cũ vẫn còn đọc được nhưng không được ghi thêm mới.
- Non-functional requirements:
  1. Không làm regress seq/status publishing.
  2. Không làm regress prompt dedupe / volatile sequence stripping hiện có.

## Architecture
- Tách pipeline rõ ràng:
  - normalize raw output
  - reduce thành visible shell text ops
  - materialize `CommittedLine` / `OpenFragment`
  - persist canonical records
- Schema tối thiểu đề xuất:
  - `scrollback_records(session_id, seq, ord, kind, text, ts)`
  - `kind in ('line', 'fragment')`
  - index theo `(session_id, seq desc, ord asc)`
- Writer rules:
  - canonical-only cho mọi seq mới
  - legacy table không ghi mới sau cutover

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state/session_event_processor.rs`
  - `crates/chatminal-runtime/src/state/native_api.rs`
  - `crates/chatminal-runtime/src/state.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `crates/chatminal-store/src/schema.rs`
- Create:
  - `crates/chatminal-runtime/src/state/canonical_scrollback.rs` nếu cần.
- Delete:
  - chưa xóa reader/writer legacy ở phase này.

## Implementation Steps
1. Add canonical schema migration.
2. Thêm canonical writer API ở store.
3. Tách reducer/materializer helper trong runtime.
4. Wire `SessionEvent::Output` sang canonical writer.
5. Wire `session_set_persist()` flush path sang canonical writer.
6. Add tests cho prompt fragment, newline, carriage return, backspace, erase-in-line.
7. Assert ordering ổn định khi một chunk sinh nhiều canonical records.

## Todo List
- [x] Thêm schema migration canonical.
- [x] Thêm store writer mới.
- [x] Wire output path mới.
- [x] Wire persist-toggle flush path mới.
- [x] Cover seq/status invariants.
- [x] Test reducer semantics tối thiểu.
- [x] Test `(seq, ord)` ordering semantics.

## Success Criteria
- Mọi history mới của session đều nằm ở canonical table.
- Không còn đường ghi mới nào phụ thuộc `scrollback_chunks`.
- Reopen path đã có dữ liệu đúng để Phase 03 đọc.

## Risk Assessment
- Chunk boundary có thể cắt giữa prompt/input fragment.
- Toggle persist giữa chừng có thể tạo mixed history ngay trong một session.
- Một chunk có thể emit nhiều records nên nếu thiếu `ord` sẽ rebuild sai thứ tự.
- Giảm thiểu bằng reducer tail state + unified `(seq, ord)` semantics.

## Security Considerations
- Sanitization vẫn chạy trước persist.
- Không lưu control sequences volatile đã biết gây title/cursor regression.

## Next Steps
- Xây unified reader + width-aware renderer trong Phase 03.

## Unresolved Questions
- Không có; retention contract đã phải chốt ở Phase 01.
