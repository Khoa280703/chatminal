# Phase 01 - Freeze Final Boundary And Rename Target Vocabulary

## Context Links
- `plans/20260311-1728-chatminal-session-execution-core-final-cutover/plan.md`
- `plans/20260311-2235-chatminal-session-runtime-direction-b/plan.md`
- `plans/20260311-2358-chatminal-session-layout-vscode-model/plan.md`

## Overview
- Priority: P0
- Status: completed
- Brief: khóa definition of done cuối cùng, chốt vocabulary first-party để các phase sau không tiếp tục reintroduce `mux/tab/pane/leaf`.

## Key Insights
- Các plan cũ dừng ở boundary “render compatibility được phép tồn tại”.
- Muốn sạch thật thì phải khóa luôn compile graph, naming, và public API.

## Requirements
- Chốt glossary mới: `session`, `session_view`, `workspace_layout`, `terminal_instance`, `render_node`.
- Chốt product rule kiểu VSCode:
  - `session` là runtime độc lập, không split nội bộ.
  - layout ngang/dọc chỉ là container ghép nhiều session cùng hiển thị.
  - clone tạo session mới; không clone view để nhiều view cùng cưỡi một session runtime sống.
- Inventory toàn bộ `mux/tab/pane/leaf/surface` còn active trong desktop + session-runtime.
- Gắn từng cluster callsite vào phase xử lý cụ thể phía sau.

## Architecture
- Tạo một boundary doc ngay trong plan: app layer, runtime layer, render layer, overlay layer.
- Định nghĩa rõ cái gì được phép tồn tại sau final cut:
  - `terminal_instance_id` thay cho `LeafId` ở desktop/public path.
  - `render_node_id` thay cho `TabId`/`PaneId` trong render tree.
- `workspace_layout` là source of truth duy nhất cho việc “gộp nhiều session cùng hiển thị”.
- `session-runtime` chỉ quản lý lifecycle/input/output của từng session độc lập.

## Related Code Files
- Modify: `plans/20260312-1358-chatminal-desktop-mux-final-removal/*`
- Inspect only: `apps/chatminal-desktop/src/**/*`
- Inspect only: `crates/chatminal-session-runtime/src/**/*`

## Implementation Steps
1. Freeze glossary và boundary.
2. Lập inventory theo bucket: runtime core, render, termwindow, overlay, frontend/main, commands.
3. Viết grep gates không mơ hồ.
4. Chỉ cho phép phase sau merge khi giảm dần inventory về zero.

## Todo List
- [ ] Freeze naming first-party
- [ ] Freeze done criteria
- [ ] Bucketize residual `mux` usage
- [ ] Attach ownership theo phase

## Success Criteria
- Không còn tranh luận “xong product nhưng còn shim”.
- Mỗi residual cluster đều có phase owner rõ ràng.
- Product rule VSCode được ghi rõ để phase sau không tái đưa split nội bộ của session quay lại.

## Risk Assessment
- Risk: naming freeze không đủ chặt, phase sau lại tạo alias mới.
- Mitigation: grep gate fail nếu có `mux/tab/pane/leaf/surface` tái xuất hiện ngoài allowlist.

## Security Considerations
- Không có thay đổi auth/data path.

## Next Steps
- Sang Phase 02 để cắt `chatminal-session-runtime` khỏi `mux` types trước.
