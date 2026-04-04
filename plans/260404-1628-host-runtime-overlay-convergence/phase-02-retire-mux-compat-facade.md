---
phase: 02
status: completed
priority: high
effort: medium
risk: medium
---

# Phase 02: Retire Mux Compat Facade

## Context Links
- [plan.md](./plan.md)
- [host-runtime lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)
- [spawn_target.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/spawn_target.rs)
- [pty_io.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/pty_io.rs)
- [localpane_hooks.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/localpane_hooks.rs)

## Overview
- Priority: P1
- Current status: completed
- Mục tiêu: bỏ `Mux` khỏi active ownership/default vocabulary để source không còn 2 mental model `HostRuntimeRoot` và `Mux` cùng sống song song.

## Key Insights
- `HostRuntimeRoot` đã là owner thật, nhưng init API vẫn trả `Arc<MuxHandle>` và nhiều hook vẫn giữ tên `mux_default()`.
- Nếu không cắt, dev mới vẫn bị dẫn sai sang mô hình `Mux-owned runtime` dù product path không còn như vậy.
- Phase này chỉ có giá trị sau khi phase 01 đã dọn bớt desktop fallback cũ.

## Requirements
- Product default path phải dùng vocabulary host/root-native.
- `Mux` nếu còn giữ phải bị hạ xuống explicit internal adapter hoặc test-only shim.
- Không đổi terminal behavior thực tế ở spawn/pty/localpane lifecycle.

## Architecture
- Public/default entrypoints chuyển sang `HostRuntimeRoot` / host-native naming.
- `MuxHandle` không còn là type alias chính mà desktop product code cầm nắm suốt lifecycle.
- `LocalSpawnHooks`, `PtyIoHooks`, `LocalPaneHooks` chỉ giữ `host_default()` làm default contract.

## Related Code Files
- Modify: `crates/chatminal-host-runtime/src/lib.rs`
- Modify: `crates/chatminal-host-runtime/src/spawn_target.rs`
- Modify: `crates/chatminal-host-runtime/src/pty_io.rs`
- Modify: `crates/chatminal-host-runtime/src/localpane_hooks.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/*`
- Update docs/tests affected by naming and contract changes

## Implementation Steps
1. Inventory toàn bộ active callsites còn phụ thuộc `MuxHandle` hoặc `mux_default()`.
2. Thiết kế canonical naming mới cho init/shutdown/subscribe/client/workspace path.
3. Đổi default hooks và spawn boundaries sang host-native types/names.
4. Hạ `MuxHandle` xuống private adapter hoặc explicit compat/test module nhỏ; không để product code import mặc định.
5. Xóa/deprecate `mux_default()` ở active path; nếu còn giữ thì phải nằm trong explicit compat namespace không được dùng mặc định.
6. Verify startup, spawn local shell, child-exit cleanup, pane output, alert/title updates.

## Todo List
- [x] Map toàn bộ `MuxHandle` import còn active
- [x] Định nghĩa root-native init/handle contract canonical
- [x] Cắt `mux_default()` khỏi default path
- [x] Thu hẹp `MuxHandle` visibility/usage
- [x] Sửa tests active scope; docs active scope chuyển sang phase 04
- [x] Chạy compile + runtime tests

## Success Criteria
- Product code không còn coi `MuxHandle` là runtime owner/default handle.
- `mux_default()` không còn là default path ở spawn/pty/localpane seams.
- `chatminal-host-runtime` đọc như một host-root architecture, không còn `Mux` là mental model song song.

## Risk Assessment
- Rename/cutover API ở crate thấp có thể làm nổ compile rộng.
- Nếu cắt quá sớm test seam, khó debug regression ở startup/cleanup.

## Security Considerations
- Giữ cleanup path deterministic để không leak pane/PTY resources.
- Không nới lỏng callback ownership hoặc cross-thread notify semantics.

## Next Steps
- Phase 03 chỉ bắt đầu khi host-runtime contract mới đã compile sạch và desktop smoke ổn định.
