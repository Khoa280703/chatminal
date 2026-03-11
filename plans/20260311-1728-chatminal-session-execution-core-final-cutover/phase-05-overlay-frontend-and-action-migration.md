# Phase 05 - Overlay Frontend And Action Migration

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/frontend.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/quickselect.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/confirm_close_pane.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/copy.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/launcher.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/scripting/guiwin.rs

## Overview
- Priority: P1
- Current status: pending
- Brief: dọn các action/UI bridge còn đi thẳng vào `Mux`, chuyển hết active flow sang session host/context và leaf identity.

## Key Insights
- Đây là nhóm callsite dễ bị bỏ sót vì không nằm ở command core nhưng vẫn có thể kéo `mux` quay lại active path
- Một số overlay chỉ cần đổi source lookup; không cần đổi giao diện hay semantics
- `frontend.rs` là đầu mối action routing; nếu không dọn sạch ở đây thì session-native core vẫn bị bypass

## Requirements
- Functional:
  - copy/select/quickselect/confirm-close/action dispatch dùng `leaf_id` hoặc session context
  - launcher/session switch route bằng session host
  - frontend spawn/focus path không gọi `Mux::spawn_tab_or_window`
- Non-functional: giữ nguyên UX hiện tại và behavior phím tắt

## Architecture
- Tạo session-aware action helpers dùng chung cho frontend/overlay
- Tập trung resolve identity một chỗ thay vì để mỗi overlay tự hỏi `Mux`
- Mọi callback công khai tiếp tục dùng vocabulary `session/surface/leaf`

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/frontend.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/launcher.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/quickselect.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/confirm_close_pane.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/overlay/copy.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/scripting/guiwin.rs

## Implementation Steps
1. Tạo helper resolve leaf/session context cho overlay/frontend
2. Thay các callsite `Mux::get()` trong overlay bằng session host/context
3. Chuyển frontend dispatch cho focus/spawn/close/activate sang session engine APIs
4. Kiểm tra scripting/gui callbacks không còn phải suy ngược từ host tab/pane
5. Chạy regression các thao tác copy/select/close/popup/launcher

## Todo List
- [ ] Overlay active path không còn trực tiếp dùng `Mux`
- [ ] Frontend dispatch không còn spawn/focus bằng tab/pane APIs
- [ ] Session/leaf context dùng chung giữa overlay và termwindow
- [ ] Scripting bridge không cần host tab để resolve active target

## Success Criteria
- `frontend.rs` và overlay active files không còn `Mux::get()` cho session flow
- Quickselect/copy/close vẫn hoạt động đúng trên session-native panes
- Không còn split-brain action path giữa frontend và session engine

## Risk Assessment
- Risk: một số overlay phụ thuộc trait/object của pane cũ
- Mitigation: thay lookup layer trước, giữ interface consumer tương thích tối đa

## Security Considerations
- Các thao tác clipboard/selection giữ nguyên local-only semantics

## Next Steps
- Sau phase này có thể bypass adapter hoàn toàn ở active path
