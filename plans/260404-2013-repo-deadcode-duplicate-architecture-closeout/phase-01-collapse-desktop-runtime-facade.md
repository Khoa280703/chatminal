---
phase: 01
status: completed
priority: high
effort: medium
risk: medium
---

# Phase 01: Collapse Desktop Runtime Facade

## Overview
Cắt `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` khỏi vai trò facade one-hop cho desktop host/runtime path, để desktop app chỉ còn một internal boundary thật: `desktop_host_runtime` cộng với runtime DTOs mà UI cần.

## Findings Covered
- Finding 1: duplicate desktop facade layer trong `chatminal_runtime/mod.rs`

## Scope
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/main.rs`
- toàn bộ caller trong `apps/chatminal-desktop/src/*` còn gọi helper one-hop qua `chatminal_runtime`

## Requirements
- Caller desktop product path không được cần thêm một module trung gian chỉ để chuyển tiếp sang `desktop_host_runtime`
- Runtime DTO/re-export nào còn thực sự hữu ích cho UI có thể giữ, nhưng phải là boundary data thật, không phải forwarder behavior
- Không tạo module mới song song; cập nhật trực tiếp module hiện có

## Architecture
- `desktop_host_runtime` là owner của desktop host/runtime behavior
- `chatminal_runtime` chỉ nên còn data-facing helpers tối thiểu hoặc bị rút về thin namespace không chứa one-hop actions nữa
- `main.rs` và UI callers gọi thẳng canonical helper layer tương ứng

## Related Code Files
- Modify:
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - `apps/chatminal-desktop/src/main.rs`
  - các caller desktop liên quan được grep ra trong phase
- Delete:
  - các one-hop wrappers không còn caller sau cutover

## Implementation Steps
1. Inventory toàn bộ exported functions trong `chatminal_runtime/mod.rs`, phân loại thành: DTO/re-export cần giữ, one-hop wrappers phải cắt, helper local còn dùng nội bộ.
2. Chuyển caller product path ở `main.rs` và các module desktop khác sang `desktop_host_runtime` hoặc runtime crate tương ứng.
3. Xóa block wrappers one-hop không còn lý do tồn tại.
4. Thu hẹp `chatminal_runtime/mod.rs` về data/query boundary tối thiểu hoặc internal namespace hẹp.
5. Chạy grep guard để chắc không còn caller mới vô tình bám facade cũ.

## Todo List
- [x] Lập inventory exports của `chatminal_runtime/mod.rs`
- [x] Chuyển `main.rs` khỏi `chatminal_runtime::*` wrappers hành vi
- [x] Chuyển các caller desktop khác khỏi facade one-hop
- [x] Xóa wrappers dư
- [x] Giữ lại đúng DTO/re-export còn có semantic value

## Success Criteria
- `main.rs` không còn gọi startup/shutdown host runtime qua `chatminal_runtime` facade
- `chatminal_runtime/mod.rs` không còn block lớn wrappers one-hop sang `desktop_host_runtime`
- Desktop product path chỉ còn một behavioral boundary rõ ràng cho host runtime

## Risk Assessment
- Risk: cắt nhầm helper mà UI/test vẫn cần
- Mitigation: inventory theo nhóm caller trước, chỉ xóa sau khi grep caller về 0

## Security Considerations
- Không có auth/data risk trực tiếp; focus là kiến trúc và maintainability

## Next Steps
- Phase 02 có thể bắt đầu sau khi behavioral entrypoints desktop không còn lệ thuộc facade trung gian
