# Plan 260305-2010 - WezTerm GUI Embedded Runtime + macOS Sign-off

## Mục tiêu
- Hoàn tất phần còn lại sau cutover `window-wezterm-gui`:
1. Embedded runtime/domain (không còn `proxy-wezterm-session` bridge process).
2. Ký manual sign-off đầy đủ trên macOS cho smoke + IME.

## Phạm vi
- `apps/chatminal-app`:
  - thiết kế và tích hợp `chatminal_ipc_mux_domain` embedded path.
  - loại bỏ dần `terminal_wezterm_gui_proxy.rs` khỏi đường chạy mặc định.
- `scripts/fidelity/*`, `docs/*`, `plans/*`:
  - bổ sung gate cho manual sign-off macOS (owner/date).

## Backlog chuyển tiếp
1. Embedded `chatminal_ipc_mux_domain` + race tests.
2. Embedded `wezterm-gui` runtime (không proxy process).
3. macOS manual smoke với WezTerm binary thật.
4. IME manual matrix (vi/ja/zh) ký owner/date.

## Definition of done
1. Không còn `proxy-wezterm-session` trong đường chạy window mặc định.
2. `ime-manual-evidence.md` có owner/date đã ký.
3. Rollout checklist không còn mục pending macOS.
