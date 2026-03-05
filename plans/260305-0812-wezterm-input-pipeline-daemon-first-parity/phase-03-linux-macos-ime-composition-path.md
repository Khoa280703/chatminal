# Phase 03 - Linux/macOS IME Composition Path

## Context Links
- [plan.md](./plan.md)
- [phase-02-linux-macos-shared-input-translation-layer.md](./phase-02-linux-macos-shared-input-translation-layer.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_actions.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_actions.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs)
- [/home/khoa2807/working-sources/chatminal/third_party/wezterm/window/src/lib.rs](/home/khoa2807/working-sources/chatminal/third_party/wezterm/window/src/lib.rs)

## Overview
- Priority: P1
- Status: Completed
- Effort: 7d
- Brief: làm sạch luồng IME để tiếng Việt/Japanese/Chinese commit đúng, không drop/duplicate.

## Key Insights
- `egui` có thể phát cả `Text` và `ImeCommit` cho cùng input, dễ double-send.
- Candidate/preedit UI cần cursor rect/focus chính xác để tránh cảm giác lag hoặc sai vị trí.
- IME composition không nên đi chung lane với shortcut hoặc raw control keys.

## Requirements
- Functional:
1. IME commit cho Vietnamese, Japanese, Chinese gửi đúng text cuối cùng vào PTY.
2. Không duplicate ký tự khi cả `Text` và `ImeCommit` cùng xuất hiện.
3. Ctrl-based terminal shortcuts vẫn hoạt động khi IME bật.
- Non-functional:
1. Không tăng input RTT vượt gate.
2. Không phá flow không-IME hiện tại.

## Architecture
- Composition state machine (window path):
1. `Idle`
2. `Composing { preedit_hash, started_at }`
3. `Committed { text, ts }`
- Event handling order:
1. Raw key shortcut filter.
2. IME commit dedupe layer.
3. Text commit forward layer.
- Cursor/IME sync:
1. giữ `IMEOutput` cập nhật theo terminal cursor rect.
2. fallback khi mất focus: flush composition hoặc discard theo policy rõ.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_actions.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/mod.rs`
4. `/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md`
- Create:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/ime-composition-state.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/ime-commit-deduper.rs`
3. `/home/khoa2807/working-sources/chatminal/plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/ime-manual-matrix.md`
- Delete:
1. None

## Implementation Steps
1. Tách IME handling khỏi `handle_terminal_input_events` thành module riêng.
2. Thêm dedupe rule giữa `EguiEvent::Text` và `EguiEvent::Ime(Commit)`.
3. Thêm manual matrix script/checklist cho 3 ngôn ngữ IME.
4. Chạy thử nghiệm Linux (IBus/Fcitx) + macOS Input Sources, ghi report.
5. Chốt default policy khi focus đổi trong lúc composing.

## Todo List
- [x] IME dedupe tests cho các pattern double-event phổ biến.
- [x] Manual checklist cho Vietnamese Telex/VNI.
- [x] Manual checklist cho Japanese Hiragana/Katakana conversion.
- [x] Manual checklist cho Chinese Pinyin candidate selection.

## Success Criteria
- Không còn lỗi duplicate/drop char trong IME matrix required cases.
- Candidate window/cursor behavior chấp nhận được trên Linux/macOS.
- Ctrl+C, Ctrl+L, Alt+Backspace vẫn đúng khi IME đang bật.

## Risk Assessment
- Risk: backend GUI event order khác nhau theo platform.
- Mitigation: state machine + timestamp dedupe thay vì assumption order cố định.
- Risk: IME preedit handling chưa có API đầy đủ trong eframe/egui.
- Mitigation: target commit-correctness trước; preedit rendering best-effort.

## Security Considerations
- Không persist preedit text vào store/log.
- Error logs IME phải redact nội dung text người dùng khi không bật debug mode.

## Next Steps
- Phase 04 tối ưu daemon/runtime path để chịu tải input tốt hơn.

## Decisions Locked
1. Không expose IME debug overlay ở wave đầu; dùng structured debug logs + replay markers trong môi trường dev.
