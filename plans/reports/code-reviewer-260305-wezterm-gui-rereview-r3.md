# code-reviewer-260305-wezterm-gui-rereview-r3

Date: 2026-03-05
Reviewer: code-reviewer
Work context: `/home/khoa2807/working-sources/chatminal`
Scope:
- `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs`
- `scripts/smoke/window-wezterm-gui-smoke.sh`

Scout notes (edge-case focused):
- Re-checked proxy event/input flow + IPC backlog behavior.
- Re-checked smoke timeout fallback path and shell semantics (`set -euo pipefail`).

## Remaining findings

### High
- None.

### Medium
1. Exit-tail drain vẫn có thể dừng quá sớm, làm thiếu output cuối.
- Evidence: `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:323` break ngay khi `recv_event(timeout)` trả `None`, dù budget tổng đang là `EXIT_DRAIN_MAX_DURATION` (`:312`, `:318`).
- Impact: nếu tail output đến muộn hơn poll đầu (~5ms) sau `PtyExited`, phần output cuối vẫn có thể bị cắt.
- Suggestion: khi `None`, tiếp tục poll tới deadline (thay vì `break` ngay), hoặc yêu cầu tối thiểu N lần poll rỗng liên tiếp trước khi thoát.

### Low
- None.

## Validation run
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml terminal_wezterm_gui_proxy -- --nocapture` (pass)
- `bash -n scripts/smoke/window-wezterm-gui-smoke.sh` (pass)
- `bash scripts/smoke/window-wezterm-gui-smoke.sh` (pass)

## Unresolved questions
- None.
