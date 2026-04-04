---
phase: 03
status: pending
priority: high
effort: medium
risk: medium
---

# Phase 03: Cut History Dual-Read

## Overview
Kết thúc steady-state dual-read giữa canonical scrollback và legacy `scrollback_chunks`.

## Why This Phase Exists
- Đây là duplicate architecture còn active trong runtime/store/tests.
- `build_logical_snapshot()` vẫn phải hợp nhất canonical + legacy path.
- Complexity hiện tại không còn hợp lý nếu product đã ổn định trên canonical path.

## Scope
- `crates/chatminal-runtime/src/state/canonical_scrollback.rs`
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `crates/chatminal-runtime/src/state/runtime_lifecycle.rs`
- `crates/chatminal-store/src/lib.rs`
- tests ở `crates/chatminal-runtime/src/state/tests.rs`
- tests ở `crates/chatminal-store/tests/store-workspace.rs`

## Requirements
- Runtime steady-state chỉ đọc canonical + terminal replay.
- Legacy chunks nếu còn giữ chỉ được tồn tại như migration helper hữu hạn, không là read path mặc định.

## Implementation Steps
1. Chọn cutover strategy low-risk: lazy per-session migration hoặc one-shot startup migration, không làm background system phức tạp.
2. Cô lập legacy read vào module/helper migration duy nhất.
3. Đổi `build_logical_snapshot()` sang canonical-first, rồi canonical-only cho steady-state path.
4. Dọn tests/retention/clear-history để bỏ assumption dual-read luôn tồn tại.
5. Chỉ sau khi migration helper ổn định mới tính đến drop table/path legacy ở wave sau hoặc cuối phase nếu cost thấp.

## Done Criteria
- Active runtime path không còn phải merge canonical + legacy mỗi lần load/restore.
- Legacy `scrollback_chunks` không còn là steady-state source of truth.
- Clear/load/restore tests phản ánh canonical-only steady-state rõ ràng.

## Risk / Tradeoff
- Risk: mất history cũ nếu migration sai.
- Tradeoff: chấp nhận giữ một migration seam hẹp hữu hạn còn tốt hơn giữ dual-read vô thời hạn trong runtime path.
