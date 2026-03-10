# Phase 02 - GUI Runtime Integration

## Context Links
- [plan.md](./plan.md)
- [chatminal_sidebar/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-chatminal-desktop/src/chatminal_sidebar/mod.rs)
- [chatminal_sidebar/ipc.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-chatminal-desktop/src/chatminal_sidebar/ipc.rs)
- [termwindow/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-chatminal-desktop/src/termwindow/mod.rs)

## Overview
- Priority: P0
- Status: completed
- Brief: đưa `apps/chatminal-chatminal-desktop` sang dùng runtime in-process thay vì sidebar IPC client.

## Key Insights
- Sidebar hiện có thread sync loop + reconnect + polling; đó là overhead thừa trên desktop path.
- GUI cần một runtime handle/snapshot subscription ổn định để invalidate window theo event.

## Requirements
- GUI bootstrap được runtime singleton trong process.
- Sidebar snapshot lấy trực tiếp từ runtime state.
- Create/switch profile/session gọi trực tiếp runtime API.

## Architecture
- Tạo module mới trong GUI:
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/mod.rs`
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/handle.rs`
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/subscription.rs`
- `chatminal_sidebar/mod.rs` đổi từ IPC client sang runtime adapter nội bộ.
- `main.rs` khởi tạo runtime tại startup và inject handle vào `TermWindow`.

## Related Code Files
- Modify:
  - `apps/chatminal-chatminal-desktop/src/main.rs`
  - `apps/chatminal-chatminal-desktop/src/chatminal_sidebar/mod.rs`
  - `apps/chatminal-chatminal-desktop/src/termwindow/mod.rs`
  - `apps/chatminal-chatminal-desktop/src/termwindow/mouseevent.rs`
- Create:
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/mod.rs`
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/handle.rs`
  - `apps/chatminal-chatminal-desktop/src/chatminal_runtime/subscription.rs`
- Delete:
  - `apps/chatminal-chatminal-desktop/src/chatminal_sidebar/ipc.rs`

## Implementation Steps
1. Thêm runtime bootstrap trong GUI app.
2. Đổi `ChatminalSidebar` sang giữ `RuntimeHandle` thay vì endpoint/app_bin.
3. Đổi background sync từ socket reconnect loop sang event subscription in-process.
4. Chuyển `request/apply_workspace` thành direct runtime API calls.

## Todo List
- [ ] Bootstrap runtime singleton trong GUI
- [ ] Thay sidebar IPC client bằng runtime adapter
- [ ] Đưa invalidate/event refresh về subscription in-process
- [ ] Giữ UX sidebar hiện tại không đổi

## Success Criteria
- GUI không còn mở local stream chỉ để render sidebar.
- Không còn polling reconnect loop trong sidebar.
- Click profile/session/new profile/new session không đi qua IPC.

## Risk Assessment
- Risk: lifetime/runtime handle khó cắm vào window lifecycle.
- Mitigation: dùng `Arc<ChatminalRuntime>` + subscription channel riêng cho mỗi window.

## Security Considerations
- Runtime handle không expose raw store/path APIs trực tiếp cho UI code.
- UI chỉ gọi typed methods cho workspace/session actions.

## Next Steps
- Sau phase này, sidebar đã native thật; còn terminal pane path vẫn đi qua proxy.
