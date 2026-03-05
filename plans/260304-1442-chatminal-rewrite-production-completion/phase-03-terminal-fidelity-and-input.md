# Phase 03 - Terminal Fidelity and Input

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_core.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_core.rs)
- [/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md](/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md)

## Overview
- Priority: P1
- Status: Completed
- Mục tiêu: nâng độ giống terminal thực tế cho input/output/alt-screen/reconnect.

## Key Insights
- `attach` đã cho interactive path, cần nâng đầy đủ key mapping + mode behavior.
- Fidelity fail thường nằm ở input translation + resize + cursor state restore.
- Runtime binding cần guard timestamp theo session + bootstrap watermark để chặn stale backlog sau rehydrate/reconnect.

## Requirements
- Functional:
1. Key mapping đủ cho shell/TUI phổ biến.
2. Reconnect không làm lệch prompt/cursor.
3. Scrollback/paste behavior nhất quán.
- Non-functional:
1. Không lag rõ rệt khi output burst.
2. IME path không làm mất ký tự.

## Architecture
- Tạo input translation layer riêng (raw key -> PTY bytes).
- Tạo snapshot/reconnect reconciler theo seq.
- Tăng addon/state handling ở client adapter.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_attach_tui.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_core.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_workspace_binding_runtime.rs`
- Create:
1. `apps/chatminal-app/src/input/*`
2. `apps/chatminal-app/src/terminal_wezterm_attach_frame_renderer.rs`
3. `apps/chatminal-app/src/reconnect/*`
- Delete:
1. Mapping cũ trùng sau khi tách input layer

## Implementation Steps
1. Chuẩn hóa keymap (ctrl/alt/meta/function keys/home/end/page).
2. Chuẩn hóa paste + bracketed paste behavior.
3. Hoàn thiện reconnect reconciliation theo seq/generation.
4. Tạo fidelity test scripts cho nhóm app bắt buộc.
5. Chạy matrix manual và fix regression.

## Todo List
- [x] Bổ sung key mapping mở rộng (function keys/backtab) + unit tests cho `attach`.
- [x] Tách input translator module riêng, có test table-driven đầy đủ.
- [x] Thêm baseline reconnect/runtime-event tests cho workspace binding.
- [x] Thêm guard chống giảm `seq` khi nhận `SessionUpdated` out-of-order + regression test.
- [x] Thêm reconnect state machine tests đầy đủ (multi-session + generation edge-cases, stale backlog watermark, same-ms timestamp cases).
- [x] Viết checklist manual cho `vim/nvim/tmux/btop/lazygit/fzf`.
- [x] Thêm script auto smoke cho fidelity matrix với JSON report + strict mode toggle. (`scripts/fidelity/phase03-fidelity-matrix-smoke.sh`)
  - strict mode mặc định để tránh false-pass gate.
  - strict mode enforce required-case coverage (`CHATMINAL_FIDELITY_REQUIRED_CASES`).
  - relaxed mode có chủ đích qua env.
  - report có fallback path để không mất artifact khi path report custom bị lỗi.
- [x] Fix toàn bộ P1 fidelity bugs trước phase 04 (trong scope matrix smoke + checklist hiện tại).

## Success Criteria
- Các app matrix chạy được không lỗi blocker.
- Prompt/cursor không chồng sau reconnect + clear.
- Input mapping ổn định qua SSH/local.

## Risk Assessment
- Risk: edge-case terminal mode khó tái hiện tự động.
- Mitigation: test scripts + manual matrix bắt buộc trong CI artifact.

## Security Considerations
- Không persist stdin raw theo default.
- Sanitization log khi bật debug input tracing.

## Next Steps
- Bàn giao Phase 04 để khóa transport đa nền tảng.

## Unresolved questions
- None.
