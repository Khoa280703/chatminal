---
phase: 01
status: completed
priority: high
effort: medium
risk: medium
---

# Phase 01: Collapse Desktop Host Legacy Surface

## Context Links
- [plan.md](./plan.md)
- [desktop_host_runtime/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/mod.rs)
- [session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs)

## Overview
- Priority: P1
- Current status: completed
- Mục tiêu: bỏ dual surface `desktop_session_host()` path vs `legacy_*` fallback path trong desktop host adapter để chỉ còn một control plane canonical.

## Key Insights
- `mod.rs` đang re-export rất nhiều `legacy_*` helper và dùng fallback ở hàng loạt function public trong desktop shell.
- `session_host.rs` đã có logic host-native thật, nhưng nhiều method instance vẫn quay lại gọi `legacy_*`, nên canonical surface chưa khóa được.
- Nếu chưa collapse phase này, các phase sau chỉ thay vocabulary chứ không giảm seam thực tế.

## Requirements
- Product flow startup, focus, spawn, activate, workspace, notifications, shutdown phải đi qua `DesktopSessionHost` hoặc helper host-native duy nhất.
- `legacy_*` nếu còn giữ chỉ được nằm trong test-only seam hoặc explicit migration seam không route từ product path.
- Không đổi behavior user-facing.

## Architecture
- Chốt `DesktopSessionHost` là owner duy nhất của desktop product host operations.
- `desktop_host_runtime/mod.rs` chỉ expose typed host-native functions; không còn fallback branching cho behavior chính.
- `session_host.rs` giữ helper nội bộ rõ ownership thay vì export `legacy_*` surface rộng.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- Modify: callsites trong `apps/chatminal-desktop/src/*` nếu còn gọi `legacy_*` indirectly
- Delete or shrink: wrapper/helpers one-hop không còn giá trị sau cutover

## Implementation Steps
1. Inventory toàn bộ `legacy_*` export và wrapper/fallback còn được gọi từ product path.
2. Phân loại: `replace now`, `move internal`, `test-only`, `delete`.
3. Chuyển các function public ở `mod.rs` sang gọi trực tiếp host-native surface; bỏ `if desktop_session_host() else legacy_*` cho product operations.
4. Inline hoặc đổi tên các `legacy_*` trong `session_host.rs` thành helper host-native nội bộ có ownership rõ.
5. Cập nhật tests đang bám vào lock/helper cũ để chỉ giữ seam test tối thiểu.
6. Verify startup, session activate, focus chuyển session, spawn shell, shutdown.

## Todo List
- [x] Lập bảng map `legacy_*` -> replacement/internal/delete
- [x] Cắt re-export `legacy_*` khỏi `mod.rs`
- [x] Bỏ fallback branching ở product-path APIs
- [x] Thu gọn `session_host.rs` instance methods không còn thin-wrap `legacy_*`
- [x] Sửa tests/seams cần thiết
- [x] Chạy compile + desktop tests liên quan

## Success Criteria
- `desktop_host_runtime/mod.rs` không còn export một dải `legacy_*` cho product path.
- Các API host product chính không còn fallback behavioral sang `legacy_*`.
- `session_host.rs` không còn instance methods chỉ thin-wrap `legacy_*` surface cũ.

## Risk Assessment
- Có thể làm gãy startup nếu một số path bootstrap vẫn ngầm phụ thuộc slot/client/workspace cũ.
- Dễ sót path focus/workspace ít dùng nếu chỉ grep tên hàm mà không trace runtime path.

## Security Considerations
- Không mở rộng capability mới.
- Giữ nguyên lifecycle cleanup để tránh leak session/pane handle hoặc dangling focus state.

## Next Steps
- Phase 02 chỉ bắt đầu sau khi desktop host surface canonical đã ổn định và test pass.
