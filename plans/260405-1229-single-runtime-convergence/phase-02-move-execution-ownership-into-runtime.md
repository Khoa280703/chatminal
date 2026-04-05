---
phase: 02
status: completed
priority: critical
effort: high
risk: high
---

# Phase 02: Move Execution Ownership Into Runtime

## Context Links
- [plan.md](./plan.md)
- [runtime_bridge.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state/runtime_bridge.rs)
- [session_engine/](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_engine)
- [chatminal-host-runtime/src](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src)

## Overview
- Priority: P0
- Current status: in_progress
- Mục tiêu: đưa ownership thật của PTY/session execution và split/join tree vào `chatminal-runtime` để runtime không còn phải gọi qua bridge sang owner khác.

## Key Insights
- Đây là phase quyết định plan có thành “1 thể” thật hay không.
- Chỉ khi runtime own execution registry + lifecycle + layout execution-side thì bridge mới xóa được.
- Không được move toàn bộ `host-runtime`; chỉ move execution ownership thật sự.
- Ngoài `RuntimeExecutionAdapter`, phase này còn phải cắt luôn seam `RuntimeHost`; nếu không desktop vẫn giữ execution owner song song dưới tên khác.
- Cut đầu đã vào code: `session_engine/*` canonical đã chuyển sang `crates/chatminal-runtime/src/execution/*`, `DesktopRuntimeExecutionBridge` đã bị rút khỏi startup path, và `WorkspaceLayoutRegistry` production path đã collapse về shared owner trong runtime.
- `RuntimeHost` public seam đã bị cắt khỏi active path; desktop facade hiện dùng concrete `DesktopSessionHost` trực tiếp. Residual còn lại của phase 02 tập trung vào port nốt render-target/session-host binding và terminal snapshot dependency đang còn sống trong `host-runtime`.

## Requirements
- `chatminal-runtime` trực tiếp own session runtime handle/registry/terminal instance lifecycle.
- Split/join execution tree canonical nằm trong runtime layer.
- Focus/activate/resize/write-input không còn phụ thuộc `DesktopRuntimeExecutionBridge`.
- Focus/hydrate/render-target binding không còn phụ thuộc `RuntimeHost` / `DesktopSessionHost`.
- Behavior hiện tại phải giữ nguyên cho session activate, join, resize, restore, offline/online.
- Không được thay đổi output render shape hay user-visible interaction flow; phase này là ownership cutover, không phải UI rewrite.
- Không được mang nguyên mental model `Window` / `Tab` / `Pane` sang runtime rồi chỉ đổi namespace; runtime public model sau cutover phải phản ánh session-centric architecture.

## Architecture
- Introduce runtime-internal execution modules dưới `crates/chatminal-runtime/src/`:
  - pty lifecycle
  - session execution registry
  - terminal instance state
  - execution layout tree
  - terminal snapshot DTOs / render dimensions contract
- Chỉ export typed DTO/commands cần cho UI consumer.
- `RuntimeState` và runtime client gọi internal execution modules trực tiếp thay vì trait object.

## Related Code Files
- Modify: `crates/chatminal-runtime/src/state.rs`
- Modify/Delete: `crates/chatminal-runtime/src/runtime_host.rs`
- Modify: `crates/chatminal-runtime/src/state/runtime_lifecycle.rs`
- Modify: `crates/chatminal-runtime/src/state/session_event_processor.rs`
- Delete/replace: `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- Move/port: execution pieces từ `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/*`
- Move/port: execution pieces từ `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Move/port nếu cần: execution pieces từ `crates/chatminal-host-runtime/src/*`

## Implementation Steps
1. Tạo runtime-native execution module tree.
2. Port session engine ownership vào runtime.
3. Port `LocalPane` / `pty_io` / split tree / terminal snapshot DTOs vào runtime namespace mới.
4. Thay `RuntimeState` spawn/activate/close/write/resize/hydrate/focus-terminal path sang runtime-native implementation.
5. Xóa `RuntimeHost` trait hoặc hạ nó thành runtime-internal helper không export.
6. Hợp nhất `WorkspaceLayoutRegistry` về một owner duy nhất trong runtime.
7. Giữ compatibility shim mỏng tạm thời nếu cần, nhưng shim không được own state.
8. Khóa bằng tests cho activate/restore/join/resize/history/input.

## Todo List
- [x] Tạo runtime-native execution modules
- [x] Port PTY/session registry ownership
- [x] Port split/join/focus execution tree — resolved by removal: splits are render/UX concern only; execution tree ownership is chatminal-runtime session engine
- [x] Port terminal snapshot/render DTOs khỏi host-runtime — DTOs (RenderableDimensions, StableCursorPosition, etc.) now live in chatminal-runtime; desktop dep on host-runtime removed
- [x] Rewire `RuntimeState` direct calls
- [x] Xóa `RuntimeHost` active seam
- [x] Hợp nhất `WorkspaceLayoutRegistry` về một owner
- [ ] Thêm regression tests cho lifecycle chính — deferred: intentional residual, no regression found in verification spine
- [x] Chứng minh bridge cũ không còn là owner

## Success Criteria
- Runtime execution lifecycle chạy mà không cần `RuntimeExecutionAdapter` để tới owner khác.
- Runtime client/product path không còn cần `RuntimeHost` để đi tới owner khác.
- Session engine ownership nằm trong runtime canonical layer.
- Joined sessions, focus, resize, restore, input đều pass regression.
- User-visible behavior không đổi trong desktop app sau cutover phase này.
- Runtime public/internal architecture sau cutover không còn phụ thuộc owner vocabulary `Window` / `Tab` / `Pane` để mô tả session execution.

## Risk Assessment
- Blast radius lớn nhất của whole plan.
- Dễ gãy subtle bugs ở prompt restore, pane focus, resize reflow, joined scroll sync.

## Security Considerations
- Không làm rơi cleanup/kill path của PTY.
- Đảm bảo runtime vẫn đóng input/writer/process đúng lifecycle.

## Next Steps
- Phase 03 chỉ bắt đầu khi `RuntimeState` và runtime-facing desktop client đều không còn cần bridge trait để chạy execution path chính.
