# Phase 03 - Restore Preview And Resize Reflow

## Context Links
- [plan.md](./plan.md)
- [phase-02-introduce-runtime-canonical-buffer.md](./phase-02-introduce-runtime-canonical-buffer.md)
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`
- `crates/chatminal-store/src/lib.rs`

## Overview
- Priority: P1
- Status: completed
- Brief: build unified snapshot reader merge legacy+canonical, rồi expose restore/preview APIs trên logical snapshot; width wrap để terminal engine tự xử lý khi hydrate.

## Key Insights
- `execution_bridge.spawn_handle()` là path restore chính, nên phải đổi sang restore API riêng đọc unified logical snapshot.
- Sidebar preview và full restore có nhu cầu khác nhau, nên không được tiếp tục ép chung một API.

## Requirements
- Functional requirements:
  1. Có unified snapshot builder merge legacy rows và canonical rows theo `seq`.
  2. Có restore API riêng trả full logical snapshot đã materialize text chuẩn cho terminal hydrate.
  3. Có preview API riêng trả last `N` logical lines + optional trailing fragment.
  4. Session mixed-source không mất dòng, không duplicate dòng khi đọc.
  5. Resize sau restore không còn lộ ranh giới “live content reflow, history đứng yên”.
- Non-functional requirements:
  1. Không đưa width logic xuống UI shell.
  2. Không duplicate snapshot-building logic giữa runtime và desktop.

## Architecture
- Thêm unified reader ở runtime/store boundary:
  - đọc legacy chunks read-only
  - đọc canonical records
  - merge theo `seq`
  - nếu canonical source có records cho `seq = N` thì bỏ legacy chunk `seq = N`
  - canonical records luôn được sort theo `(seq, ord)`
- Thêm renderer logical snapshot:
  - input: logical snapshot
  - output: canonical text không hard-wrap; terminal engine sẽ wrap theo width runtime hiện tại
- API split đề xuất:
  - `session_snapshot_get(session_id, preview_lines)`
  - `session_restore_snapshot_get(session_id)`

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state/native_api.rs`
  - `crates/chatminal-runtime/src/state/runtime_bridge.rs`
  - `crates/chatminal-runtime/src/state.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
- Create:
  - renderer helper nếu cần tách file.
- Delete:
  - chưa xóa legacy snapshot builder ở phase này.

## Implementation Steps
1. Thêm store/runtime query cho unified logical snapshot.
2. Implement merge semantics legacy+canonical theo `seq`.
3. Implement logical snapshot renderer không hard-wrap.
4. Đổi desktop restore path sang restore API riêng.
5. Đổi preview path sang preview API riêng.
6. Test reopen ở width khác, resize sau reopen, mixed-source session, joined layout restore.
7. Test chunk có nhiều canonical records vẫn render đúng thứ tự.

## Todo List
- [x] Unified logical snapshot builder.
- [x] Mixed-source merge logic.
- [x] Restore API riêng.
- [x] Preview API riêng.
- [x] Logical snapshot renderer.
- [ ] Manual scenario cho reopen/resize/mixed-source.

## Success Criteria
- Session reopen ở width khác render history đúng width mới.
- Session chứa cả legacy+canonical history vẫn hiển thị đầy đủ đúng thứ tự.
- Preview không làm regress semantics hiện tại theo logical lines.

## Risk Assessment
- Merge theo `seq` sai sẽ duplicate hoặc mất dòng.
- Bỏ qua `ord` sẽ làm reorder sai trong cùng một chunk output.
- Preview width-aware có thể tạo mismatch với UX hiện tại nếu caller chưa truyền width đúng.
- Giảm thiểu bằng API split rõ ràng và tests mixed-source.

## Security Considerations
- Rendered snapshot không được tái sinh volatile sequences đã strip.

## Next Steps
- Sau khi path mới ổn định mới cleanup legacy trong Phase 04.

## Unresolved Questions
- Không có.
