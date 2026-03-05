# Terminal Fidelity Matrix

Last updated: 2026-03-05

Mục tiêu: checklist manual chuẩn cho phase 03 (fidelity + input + reconnect).

Windows note:
- Wave hiện tại ưu tiên Linux/macOS.
- Windows input parity baseline đã completed ở phase 07 (CI compile/test lane + mapping checklist); manual matrix trên máy Windows thật vẫn nằm trong release hardening checklist.

## Setup
1. Terminal A: `make daemon-reset`
2. Terminal B: `make attach`
3. Tạo session test riêng (khuyến nghị): `make create NAME='phase03-fidelity'`
4. Attach vào session test: `make attach SESSION_ID='<session_id>'`

## Auto Smoke (phase 03)
Script smoke tự động để bắt regression nhanh trước khi chạy matrix manual:

```bash
make fidelity-matrix-smoke
```

Script chạy strict mặc định (fail cứng khi có bất kỳ case fail).
Script tự detect `timeout` (Linux) hoặc `gtimeout` (macOS coreutils).
Trong strict mode, nếu case thuộc `CHATMINAL_FIDELITY_REQUIRED_CASES` bị skip thì script cũng fail.
Default required cases: `bash-prompt,ctrl-c,ctrl-z,alt-backspace,meta-shortcuts-macos,unicode,resize,reconnect`.

Mode relaxed (không fail cứng, ghi warning vào report):

```bash
make fidelity-matrix-smoke-relaxed
```

Override danh sách required cases (CSV):
```bash
CHATMINAL_FIDELITY_REQUIRED_CASES="bash-prompt,vim,nvim,tmux,unicode,resize,reconnect" make fidelity-matrix-smoke
```

Phase-06 wrapper (strict matrix + IME manual-gate metadata):
```bash
make fidelity-input-ime-smoke
```

Report JSON mặc định:
- `/tmp/chatminal-phase03-fidelity-matrix-report-$$.json`

## Matrix
| Tool/Case | Command | Expected |
| --- | --- | --- |
| bash prompt | `echo ready` | Input/echo không trễ bất thường, prompt không chồng |
| ctrl-c | `cat` rồi gửi `Ctrl+C` | Dừng process foreground ngay, shell nhận lại prompt |
| ctrl-z | `sleep 30` rồi gửi `Ctrl+Z` | Foreground process bị stop, shell trả prompt, `jobs` thấy trạng thái Stopped |
| alt-backspace | gửi `Alt+Backspace` ở prompt | Auto smoke xác nhận input pipeline không treo; semantic xóa từ xác nhận manual |
| meta-shortcuts-macos | gửi `Option+Left/Right` hoặc `Meta-b/f` | Auto smoke xác nhận không drop input; semantic di chuyển từ xác nhận manual |
| vim | `vim` | Enter/exit alt-screen đúng, cursor không lệch |
| nvim | `nvim` | Navigation hjkl/arrow ổn định, redraw không rách |
| tmux | `tmux` | Prefix + split pane hoạt động, detach/attach không crash |
| htop | `htop` | Refresh mượt, phím `q` thoát sạch |
| btop | `btop` | Render màu + box đúng, không flood lỗi |
| lazygit | `lazygit` | UI redraw đúng, phím tắt cơ bản hoạt động |
| fzf | `fzf` | Filter realtime không giật mạnh, enter/esc đúng |
| unicode | `printf 'Tiếng Việt: ă â ê ô ơ ư đ\n'` | Ký tự không vỡ cột |
| resize | đổi kích thước terminal host | Nội dung reflow hợp lý, không crash |
| reconnect | `Ctrl+F10` thoát attach rồi `make attach SESSION_ID='<session_id>'` | Prompt/cursor không chồng; input tiếp tục bình thường |

## Required Pass Criteria
1. Không crash daemon/app trong toàn bộ matrix.
2. Không có lỗi blocker: cursor lệch, prompt chồng, input drop, freeze > 3s.
3. Reconnect giữ được khả năng thao tác terminal bình thường.
4. Với `alt-backspace` và `meta-shortcuts-macos`, auto smoke chỉ là signal ban đầu; release gate phải có manual evidence cho semantic line-edit.

## Report Template
```text
Run date:
OS:
Shell:
Session ID:

[PASS/FAIL] vim:
[PASS/FAIL] nvim:
[PASS/FAIL] tmux:
[PASS/FAIL] htop:
[PASS/FAIL] btop:
[PASS/FAIL] lazygit:
[PASS/FAIL] fzf:
[PASS/FAIL] unicode:
[PASS/FAIL] resize:
[PASS/FAIL] reconnect:

Notes:
```
