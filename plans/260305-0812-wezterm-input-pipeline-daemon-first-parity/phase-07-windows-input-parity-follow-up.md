# Phase 07 - Windows Input Parity Follow-up

## Context Links
- [plan.md](./plan.md)
- [phase-02-linux-macos-shared-input-translation-layer.md](./phase-02-linux-macos-shared-input-translation-layer.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/ipc/transport/windows.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/ipc/transport/windows.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminald/src/transport/windows.rs](/home/khoa2807/working-sources/chatminal/apps/chatminald/src/transport/windows.rs)
- [/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-input-types/src/lib.rs](/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-input-types/src/lib.rs)

## Overview
- Priority: P2
- Status: Completed
- Effort: 5d
- Brief: đã mang input pipeline mới sang Windows ở mức baseline parity sau khi Linux/macOS stable.

## Key Insights
- Windows có khác biệt lớn ở key semantics và IME stack.
- WezTerm có path `encode_win32_input_mode`, có thể dùng làm reference cho parity.
- Không block ship Linux/macOS; phase này chạy follow-up.

## Requirements
- Functional:
1. Giữ daemon model bắt buộc với Named Pipe transport.
2. Mapping modifiers/ctrl keys parity với Linux/macOS ở mức user-visible behavior.
3. IME commit cơ bản hoạt động không duplicate/drop.
- Non-functional:
1. Không làm regress transport stability trên Windows.
2. CI Windows phải pass compile + tests tối thiểu.

## Architecture
- Reuse shared semantic input layer từ phase 02.
- Add platform adapter Windows cho raw modifier mapping.
- Evaluate optional support cho win32 input mode encoding (phù hợp ConPTY apps) sau khi parity cơ bản pass.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/*`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm_actions.rs`
3. `/home/khoa2807/working-sources/chatminal/.github/workflows/rewrite-quality-gates.yml`
4. `/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md`
5. `/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md`
- Create:
1. `/home/khoa2807/working-sources/chatminal/plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/windows-input-parity-report.md`
- Delete:
1. None

## Implementation Steps
1. Port adapter layer cho Windows modifier semantics.
2. Thêm test cases Windows-specific key combos (AltGr, RightAlt/RightCtrl distinctions).
3. Chạy CI windows (compile + tests) và checklist parity.
4. Chốt decision về win32 input mode (defer/enable).

## Todo List
- [x] Windows mapping parity checklist được define.
- [x] CI windows có compile/test regression coverage tối thiểu.
- [x] Docs cập nhật trạng thái hỗ trợ Windows input fidelity.

## Success Criteria
- Hành vi Ctrl/modifier/IME cơ bản trên Windows không khác biệt lớn so Linux/macOS.
- Không có regression ở Named Pipe transport và daemon request loop.

## Risk Assessment
- Risk: khác biệt ConPTY behavior khiến expected output lệch.
- Mitigation: định nghĩa acceptance theo user-visible behavior, không ép byte-identical mọi case.
- Risk: thiếu host test thực tế ngoài CI.
- Mitigation: giữ manual smoke checklist trên máy Windows thật trong release hardening checklist.

## Security Considerations
- Named Pipe endpoint validation giữ nguyên strict policy.
- Không mở network transport để giải quyết input issues.

## Next Steps
- Đánh giá bỏ `legacy` mode sau khi cả Linux/macOS/Windows ổn định qua nhiều release.

## Progress Sync (2026-03-05)
- Rereview latest batch xác nhận docs requirement/evidence cho Windows CI đã đồng bộ.
- Open severity for this phase scope: `Critical=0`, `High=0`.

## Decisions Locked
1. Wave đầu không bắt buộc self-hosted Windows runner cho IME; dùng `windows-latest`, manual smoke trên máy thật đặt trong release hardening checklist.
