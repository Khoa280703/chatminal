# Phase 03 - Restore Preview And Resize Reflow

## Context Links
- [plan.md](./plan.md)
- [phase-02-introduce-runtime-canonical-buffer.md](./phase-02-introduce-runtime-canonical-buffer.md)
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`

## Overview
- Priority: P1
- Status: pending
- Brief: đổi các path đọc history sang render từ canonical model theo width đang dùng lúc preview/restore.

## Key Insights
- `store.session_snapshot()` hiện chỉ `join` text chunks cũ, nên width đã đóng băng từ lúc persist.
- `execution_bridge.spawn_handle()` lấy `initial_scrollback` từ `read_session_snapshot(..., usize::MAX)`; đây là điểm restore quan trọng nhất.

## Requirements
- Functional requirements:
  1. `session_snapshot_get` phải build text từ canonical buffer theo width hiện tại hoặc width caller truyền vào.
  2. Restore/reopen phải hydrate terminal bằng text render từ canonical model ở width hiện tại.
  3. Resize cửa sổ sau restore không còn lộ rõ ranh giới “live content reflow, history đứng yên”.
- Non-functional requirements:
  1. Không đưa width logic xuống UI shell.
  2. Không làm tăng coupling giữa desktop và store.

## Architecture
- Thêm renderer nhỏ `canonical_scrollback -> rendered_text(width)` ở runtime/store boundary.
- Preview API có thể cần tham số width để tránh tiếp tục implicit default-width behavior.
- `read_session_snapshot` / `session_snapshot_get` phải dùng renderer mới; `StoredSessionSnapshot.content` trở thành rendered view, không còn persistence format.

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state/native_api.rs`
  - `crates/chatminal-runtime/src/state/runtime_bridge.rs`
  - `crates/chatminal-runtime/src/state.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`
- Create:
  - renderer helper nếu cần tách file.
- Delete:
  - chưa xóa legacy snapshot builder.

## Implementation Steps
1. Thêm snapshot query mới từ canonical records.
2. Thêm render function nhận width cols.
3. Đổi desktop restore path để truyền width thật hiện tại.
4. Đổi preview path nếu cần width-aware preview.
5. Test reopen ở width khác, resize sau reopen, joined layout restore.

## Todo List
- [ ] Snapshot builder width-aware.
- [ ] Restore path truyền width hiện tại.
- [ ] Preview path thống nhất semantics.
- [ ] Manual scenario cho resize/reopen.

## Success Criteria
- Session reopen ở width khác render history đúng width mới.
- Resize sau reopen không còn “mảng history width cũ”.

## Risk Assessment
- Nếu preview API chưa có width input, có thể phải chọn default width và vẫn còn mismatch ở sidebar preview.
- Giảm thiểu bằng tách rõ: full restore width-aware trước, preview có thể follow-up nhỏ nếu cần.

## Security Considerations
- Rendered snapshot không được tái sinh các volatile sequences đã strip.

## Next Steps
- Cắt schema legacy trong Phase 04 khi path mới ổn định.

## Unresolved Questions
- Preview snapshot có cần width riêng hay chấp nhận continue dùng default preview formatting?
