# Phase 01 - Lock Canonical Scrollback Contract

## Context Links
- [plan.md](./plan.md)
- `crates/chatminal-runtime/src/state.rs`
- `crates/chatminal-runtime/src/state/session_event_processor.rs`
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `crates/chatminal-store/src/lib.rs`

## Overview
- Priority: P1
- Status: completed
- Brief: chốt model canonical, retention semantics, preview/restore contract và mixed-source rollout trước khi code.

## Key Insights
- Nếu canonical model còn mơ hồ, Phase 02-04 sẽ lại quay về heuristic vá prompt/history.
- Nếu preview và restore dùng chung một contract, width-aware reopen sẽ lại làm vỡ preview semantics.
- Nếu không chốt mixed-source strategy sớm, rollout sẽ mất hoặc duplicate history trong cùng một session.

## Requirements
- Functional requirements:
  1. Có canonical model cụ thể, đủ để render lại shell history ở width bất kỳ.
  2. Có reducer semantics tối thiểu cho shell path: `\r`, backspace, erase-in-line, prompt redraw dedupe.
  3. Có contract tách riêng cho preview và restore.
  4. Có strategy rõ cho session chứa cả legacy rows và canonical rows.
  5. Chốt rõ phần nào của main-screen redraw được support, phần nào là non-goal.
- Non-functional requirements:
  1. KISS; không làm full VT replay engine trong store.
  2. Giữ tương thích rollout với dữ liệu cũ.

## Architecture
- Canonical records:
  - `CommittedLine { seq, ord, ts, text }`
  - `OpenFragment { seq, ord, ts, text }`
  - một chunk output có thể sinh nhiều records; thứ tự chuẩn là `(seq, ord)`
- Retention contract:
  - retention tính theo số `CommittedLine`
  - `OpenFragment` không tính như full retained line nhưng luôn được giữ nếu là tail hiện tại
- Preview contract:
  - API preview chọn `N` logical lines cuối + optional trailing `OpenFragment`
  - sau đó mới render ra text theo width preview nếu caller cần
- Restore contract:
  - API restore lấy full logical snapshot + trailing `OpenFragment`
  - render theo `cols` hiện tại của runtime trước khi hydrate engine
- Mixed-source contract:
  - unified snapshot builder merge legacy chunks và canonical records theo `seq`
  - nếu canonical đã có bất kỳ records nào cho `seq = N` thì bỏ legacy chunk `seq = N`
  - trong canonical source, records được sort theo `(seq, ord)`
  - dữ liệu legacy chỉ read-only; dữ liệu mới chỉ ghi canonical

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state.rs`
  - `crates/chatminal-runtime/src/state/session_event_processor.rs`
  - `crates/chatminal-runtime/src/state/runtime_bridge.rs`
  - `crates/chatminal-runtime/src/state/native_api.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `crates/chatminal-store/src/schema.rs`
- Create:
  - có thể tách `canonical_scrollback.rs` nếu cần.
- Delete:
  - chưa xóa gì ở phase này.

## Implementation Steps
1. Audit mọi read/write path đang assume `snapshot.content` là source of truth.
2. Chốt canonical structs, serializer format và reducer rules.
3. Chốt retention semantics theo logical lines.
4. Chốt API split: preview snapshot riêng, restore snapshot riêng.
5. Chốt merge semantics cho mixed legacy/canonical rows.
6. Document explicit non-goal cho alt-screen/TUI exact replay và main-screen multi-line redraw/progress UIs.

## Todo List
- [x] Liệt kê toàn bộ read/write path của history.
- [x] Chốt canonical structs và reducer semantics.
- [x] Chốt retention/preview/restore contracts.
- [x] Chốt mixed-source merge rules.
- [x] Chốt rollout policy cho legacy rows.
- [x] Chốt supported redraw subset vs non-goals.

## Success Criteria
- Phase sau không còn ambiguity về data model.
- Không còn open question kiểu `line_count giữ hay bỏ`.
- Không còn assumption session chỉ ở một nguồn dữ liệu.

## Risk Assessment
- Model quá nghèo sẽ không đủ cho restore đúng.
- Model quá giàu sẽ thành terminal recorder trá hình.
- Giảm thiểu: chỉ support shell path với reducer semantics tối thiểu cần thiết và explicit non-goal cho multi-line redraw.

## Security Considerations
- Giữ sanitize/strip control sequences volatile trước khi persist.
- Không để restore path tái sinh title/private mode/cursor state từ DB.

## Next Steps
- Implement schema canonical tối thiểu và writer path mới trong Phase 02.

## Unresolved Questions
- Có cần nén payload canonical ngay từ bản đầu không?
