---
title: "Chatminal Rewrite Completion Plan"
description: "Kế hoạch hoàn thiện Chatminal từ TUI scaffold lên bản terminal app production-ready."
status: completed
priority: P1
effort: 9w
branch: main
tags: [chatminal, rewrite, wezterm, daemon, quality-gate]
created: 2026-03-04
---

# Tổng quan
Mục tiêu: hoàn tất phần còn lại của rewrite để phát hành bản native terminal app ổn định, testable, cross-platform.

## Scope chốt
- Runtime giữ nguyên: `apps/chatminald`, `apps/chatminal-app`, `crates/chatminal-protocol`, `crates/chatminal-store`.
- Không quay lại stack cũ.
- Ưu tiên terminal-first; UX phụ chỉ thêm khi không làm chậm quality gate.
- Rollout ưu tiên: Linux + macOS trước, Windows ở phase kế tiếp.

## KPI chốt cho quality gate
1. Input latency local IPC:
- p95 input RTT `<= 30ms`
- p99 input RTT `<= 60ms`
- Fail gate nếu p95 `> 45ms`
2. Memory budget (RSS, sau warmup/soak):
- `chatminald` mục tiêu `<= 220MB`, fail nếu `> 300MB`
- `chatminal-app` mục tiêu `<= 280MB`, fail nếu `> 380MB`
- Tổng 2 process mục tiêu `<= 450MB`, fail nếu `> 600MB`

## Phase map
| Phase | Trạng thái | Kết quả chính | Tài liệu |
| --- | --- | --- | --- |
| 01. Native Window Foundation | Completed | `window-wezterm` + reducer tests + headless smoke script/CI step | [phase-01-native-window-foundation.md](./phase-01-native-window-foundation.md) |
| 02. Daemon Concurrency & Perf | Completed | Giảm contention, bounded pipeline, metrics nội bộ + benchmark hard-gate RTT/RSS baseline | [phase-02-daemon-concurrency-and-performance.md](./phase-02-daemon-concurrency-and-performance.md) |
| 03. Terminal Fidelity & Input | Completed | Input translator + reconnect guards/tests + fidelity matrix smoke strict/manual checklist | [phase-03-terminal-fidelity-and-input.md](./phase-03-terminal-fidelity-and-input.md) |
| 04. Transport Cross-Platform | Completed | UDS Linux/macOS + Named Pipe Windows qua trait chung | [phase-04-cross-platform-transport.md](./phase-04-cross-platform-transport.md) |
| 05. Quality Gates & Release | Completed | Matrix test + soak + cutover checklist + release artifacts | [phase-05-quality-gates-and-release.md](./phase-05-quality-gates-and-release.md) |

## Phụ thuộc chính
1. Phase 02 phải xong trước khi chạy soak/fidelity dài.
2. Phase 03 phụ thuộc Phase 01 (window shell đã có).
3. Phase 04 phải xong trước release đa nền tảng.
4. Phase 05 chạy sau Phase 02-04.

## TODO ngắn
- [x] Chốt framework window cho `chatminal-app` (winit + renderer tối thiểu).
- [x] Chốt KPI hiệu năng daemon baseline (command `bench-rtt-wezterm` + script hard-gate RTT/RSS).
- [x] Chốt test matrix bắt buộc cho terminal fidelity.
- [x] Chốt transport trait + Windows Named Pipe acceptance.
- [x] Chốt release checklist và điều kiện cutover.

## Definition of Done (toàn plan)
- Native app có thể dùng hằng ngày cho session/profile terminal workflow.
- Không còn blocker fidelity với nhóm app bắt buộc (`vim`, `nvim`, `tmux`, `btop/htop`, `lazygit`, `fzf`).
- Pass quality gate tự động + manual matrix theo checklist.
- Docs roadmap/changelog/architecture cập nhật khớp trạng thái code.

## Unresolved questions
- None.
