# code-reviewer-260305-wezterm-gui-rereview-r2

Date: 2026-03-05
Reviewer: code-reviewer
Work context: `/home/khoa2807/working-sources/chatminal`
Scope:
- `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs`
- `apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs`
- `scripts/smoke/window-wezterm-gui-smoke.sh`

## Remaining findings

### High
- None.

### Medium
1. Proxy có risk mất tail output ở thời điểm process exit.
- Evidence: `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:269` (return `Ok(true)` ngay khi nhận `PtyExited`), được xử lý thoát vòng lặp tại `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:117`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:118`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:126`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:127`.
- Impact: nếu `PtyOutput` cuối đến sau `PtyExited` do race liên luồng/event ordering, UI có thể thiếu phần output cuối.

2. Input burst lớn vẫn có thể gây event starvation ngắn hạn.
- Evidence: proxy vẫn flush request đồng bộ ngay trong phase drain input tại `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:90`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:92`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:93`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:94`, rồi flush tiếp ở `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:97`; poll event chỉ diễn ra sau đó tại `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:117`.
- Impact: khi paste burst lớn, vẫn có cửa sổ thời gian output/event bị trễ (và trong tải cực đại có thể tăng xác suất drop event ở tầng IPC backlog).

### Low
1. Smoke timeout có thể thành không giới hạn trên host không có `timeout`/`gtimeout`.
- Evidence: fallback branch chạy trực tiếp command tại `scripts/smoke/window-wezterm-gui-smoke.sh:67`.
- Impact: nếu launcher bị treo bất thường, smoke job có thể treo dài.

## Unresolved questions
- None.
