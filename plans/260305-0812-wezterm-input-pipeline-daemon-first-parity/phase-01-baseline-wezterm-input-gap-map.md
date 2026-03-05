# Phase 01 - Baseline WezTerm Input Gap Map

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/README.md](/home/khoa2807/working-sources/chatminal/README.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/pty_key_translator.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/pty_key_translator.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_input_mapper.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_input_mapper.rs)
- [/home/khoa2807/working-sources/chatminal/third_party/wezterm/window/src/lib.rs](/home/khoa2807/working-sources/chatminal/third_party/wezterm/window/src/lib.rs)
- [/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-input-types/src/lib.rs](/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-input-types/src/lib.rs)

## Overview
- Priority: P1
- Status: Completed
- Effort: 4d
- Brief: đóng gap map giữa pipeline hiện tại và WezTerm-style input architecture trước khi sửa code.

## Key Insights
- Hiện có 2 mapper độc lập (`crossterm` attach và `egui` window) nên behavior lệch nhau.
- Mapping đang rời rạc, chưa có semantic event layer (`raw key`, `composed text`, `paste`, `ime commit`).
- IME path mới handle commit thô, chưa có quy tắc chống double-send giữa `Text` và `ImeCommit`.

## Requirements
- Functional:
1. Có inventory đầy đủ key/modifier/IME cases cần support.
2. Có bảng so sánh behavior hiện tại vs expected terminal behavior.
3. Có baseline benchmark trước thay đổi để tránh false regression.
- Non-functional:
1. Không đụng behavior runtime production ở phase này.
2. Tài liệu hóa đủ để implement team làm song song không lệch spec.

## Architecture
- Tạo `input behavior contract` theo 3 lane:
1. Shortcut lane (app-level commands, ví dụ quit/focus).
2. Terminal key lane (Ctrl/Alt/Meta/Fn/navigation -> PTY bytes).
3. Text/IME lane (unicode text commit, composition-safe).
- Ánh xạ theo mô hình WezTerm:
1. `RawKeyEvent` để quyết định shortcut và modifiers.
2. `KeyEvent`/composed text cho dữ liệu terminal.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
2. `/home/khoa2807/working-sources/chatminal/docs/code-standards.md`
3. `/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md`
- Create:
1. `/home/khoa2807/working-sources/chatminal/plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/baseline-input-gap-map.md`
2. `/home/khoa2807/working-sources/chatminal/plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/baseline-input-kpi.json`
- Delete:
1. None

## Implementation Steps
1. Thu thập baseline commands (`bench-phase02`, fidelity scripts, attach/window manual key checks).
2. Lập matrix key/modifier: Ctrl combos, Alt/meta combos, Fn/navigation, backtab, delete/insert.
3. Lập matrix IME: Vietnamese Telex/VNI, Japanese Hiragana/Katakana, Chinese Pinyin (commit/cancel/reconvert).
4. So khớp matrix với WezTerm reference logic (raw/composed, xterm baseline + kitty opt-in semantics).
5. Chốt acceptance contract cho Phase 02-03.

## Todo List
- [x] Baseline RTT/RSS report lưu vào `reports/`.
- [x] Gap matrix có priority P0/P1/P2 cho từng case.
- [x] Document rõ case nào chỉ Linux/macOS, case nào defer Windows.
- [x] Chốt test hosts tối thiểu: Linux representative stack + macOS (full X11/Wayland matrix defer wave sau).

## Success Criteria
- Có 1 tài liệu gap map dùng làm source-of-truth cho implement.
- Có baseline số liệu trước thay đổi để so sánh hậu triển khai.
- Không có ambiguity về expected behavior cho Ctrl+C/modifiers/IME.

## Risk Assessment
- Risk: baseline thiếu host variation (X11/Wayland/macOS input stack khác nhau).
- Mitigation: bắt buộc thu mẫu ít nhất 2 host profile trước phase code (Linux representative + macOS).
- Risk: over-scope do cover quá nhiều combo hiếm.
- Mitigation: gán priority, ship P0/P1 trước, P2 backlog.

## Security Considerations
- Không log raw user input đầy đủ vào file; chỉ log metadata hoặc sampled escaped payload.
- Reports không chứa dữ liệu nhạy cảm từ terminal session thật.

## Next Steps
- Bắt đầu Phase 02 với input contract đã đóng băng.

## Decisions Locked
1. Dead-key specific matrix (fr/de layouts) không nằm trong wave đầu; đưa vào backlog P2 sau khi P0/P1 ổn định.
