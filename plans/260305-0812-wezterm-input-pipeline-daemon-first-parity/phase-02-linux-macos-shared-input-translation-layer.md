# Phase 02 - Linux/macOS Shared Input Translation Layer

## Context Links
- [plan.md](./plan.md)
- [phase-01-baseline-wezterm-input-gap-map.md](./phase-01-baseline-wezterm-input-gap-map.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/pty_key_translator.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/pty_key_translator.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_input_mapper.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_input_mapper.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_attach_tui.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_attach_tui.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_actions.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_actions.rs)

## Overview
- Priority: P1
- Status: Completed
- Effort: 8d
- Brief: hợp nhất translator cho attach + window để Ctrl+C/modifiers ổn định và giống nhau trên Linux/macOS.

## Key Insights
- Hai mapper khác nhau tạo drift theo thời gian, khó test và khó rollback có kiểm soát.
- Ctrl/meta handling trên macOS cần tách rõ shortcut-level và terminal-level.
- Nên chuẩn hóa qua semantic input event trước khi encode PTY bytes.

## Requirements
- Functional:
1. Một input pipeline chung cho attach (`crossterm`) và window (`egui`).
2. Cover đầy đủ Ctrl/Alt/Shift/Meta + function/navigation keys theo contract.
3. Ctrl+C luôn gửi đúng interrupt (`0x03`) vào PTY khi terminal có focus.
- Non-functional:
1. Không tăng p95 RTT quá target.
2. Có feature flag cho legacy fallback.

## Architecture
- Introduce `TerminalInputEvent` (internal):
1. `KeyChord { key, modifiers, repeat, source }`
2. `TextCommit { text, source }`
3. `Paste { text, bracketed_hint }`
4. `ImeCommit { text, locale_hint }`
- Introduce shared encoder:
1. `input/terminal_input_encoder.rs` encode theo xterm-compatible baseline, có nhánh `kitty CSI-u` opt-in.
2. unify control mapping table (Ctrl+A..Z, Ctrl+Space, Backtab, Fn keys).
3. app shortcut filter chạy trước encode (ví dụ `F10` quit attach).
- Fallback model:
1. `CHATMINAL_INPUT_PIPELINE_MODE=legacy|wezterm`.
2. default `legacy` trong wave đầu; `wezterm` cho canary.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/mod.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/pty_key_translator.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_input_mapper.rs`
4. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_actions.rs`
5. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_attach_tui.rs`
6. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/Cargo.toml`
- Create:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/terminal-input-event.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/terminal-input-encoder.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/terminal-input-shortcut-filter.rs`
- Delete:
1. Logic mapping trùng lặp sau khi hợp nhất (chỉ xóa khi test parity pass).

## Implementation Steps
1. Trích common mapping table từ 2 mapper hiện tại vào encoder chung.
2. Thêm semantic event layer để input source khác nhau vẫn ra cùng result.
3. Áp dụng pipeline mới vào attach path (Linux/macOS), giữ fallback env.
4. Áp dụng pipeline mới vào window path (Linux/macOS), giữ fallback env.
5. Viết unit tests table-driven cho 2 source event (`crossterm`, `egui`) với expected bytes giống nhau.

## Todo List
- [x] Hợp nhất mapping table, bỏ drift attach/window.
- [x] Thêm regression tests cho Ctrl+C, Ctrl+Z, Alt+key, Cmd/Ctrl split trên macOS.
- [x] Bổ sung integration test đơn giản qua command `input` + `snapshot`.
- [x] Cập nhật docs code standards cho invariant input pipeline.

## Success Criteria
- Attach và window cho ra cùng bytes với cùng key/modifier.
- Ctrl+C hoạt động ổn định trong `bash/vim/tmux`.
- Canary mode `wezterm` chạy pass matrix P0/P1 trên Linux/macOS.

## Risk Assessment
- Risk: shortcut conflict (Cmd+C copy vs Ctrl+C interrupt trên macOS).
- Mitigation: shortcut filter explicit theo focus + platform rules.
- Risk: encode sai với non-US layout.
- Mitigation: ưu tiên semantic text commit cho layout-sensitive chars.

## Security Considerations
- Giới hạn payload size vẫn bám `MAX_INPUT_BYTES` daemon side.
- Không cho bypass daemon bằng direct PTY write từ app.

## Next Steps
- Phase 03 xử lý IME composition đầy đủ.

## Decisions Locked
1. Wave đầu giữ encoder nội bộ tối giản (YAGNI), chưa thêm `wezterm-input-types` trực tiếp; đánh giá lại sau stabilization.
