---
title: "Chatminal WezTerm GUI Migration Plan"
description: "Migrate Chatminal window runtime from egui+wezterm-term to full wezterm-gui while preserving daemon-first session/profile/history IPC model."
status: completed
priority: P1
effort: 5w
branch: main
tags: [chatminal, wezterm-gui, ipc, linux, macos, terminal-fidelity]
created: 2026-03-05
---

# Overview
Goal: ship `window-wezterm-gui` with terminal fidelity equal to WezTerm GUI, while `chatminald` remains single source of truth for `session/profile/history` via existing IPC contracts.

## Scope Lock
- Linux/macOS first-class in first release wave.
- Keep daemon ownership of PTY/session/store logic; client never spawns shell directly.
- Keep current IPC (`chatminal-protocol`) backward compatible during migration.

## Architecture Mapping (Current -> Target)
| Current (`chatminal-app`) | Target (`chatminal-app`) | Migration note |
| --- | --- | --- |
| `window/native_window_wezterm.rs` (`eframe/egui`) | `window_wezterm_gui` runtime built on `wezterm-gui` + `window` | Replace UI loop, remove text-area pseudo terminal rendering path |
| `terminal_wezterm_core.rs` (`wezterm-term` snapshot renderer) | Native `wezterm-gui` pane rendering | Stop re-rendering terminal text manually |
| `native_window_wezterm_input_mapper.rs` (egui events -> bytes) | `window::WindowEvent::{RawKeyEvent,KeyEvent}` pipeline | Let WezTerm input/IME stack drive fidelity |
| `terminal_workspace_binding_runtime.rs` (event->adapter state) | `chatminal_ipc_mux_domain` (IPC -> mux/pane updates) | Preserve daemon event stream semantics |
| `ipc::ChatminalClient` + `chatminal-protocol` | Keep and reuse | Compatibility-critical boundary |

## Phase Map
| Phase | Status | Focus | File |
| --- | --- | --- | --- |
| 01 | Completed | Baseline + target architecture freeze | [phase-01-baseline-and-architecture-mapping.md](./phase-01-baseline-and-architecture-mapping.md) |
| 02 | Completed | Build IPC-backed mux/domain adapter | [phase-02-chatminal-ipc-mux-domain-adapter.md](./phase-02-chatminal-ipc-mux-domain-adapter.md) |
| 03 | Completed | Integrate wezterm-gui window runtime (Linux/macOS) | [phase-03-wezterm-gui-linux-macos-window-runtime.md](./phase-03-wezterm-gui-linux-macos-window-runtime.md) |
| 04 | Completed | Session/profile/history compatibility + guardrails | [phase-04-session-profile-history-compatibility-and-rollout-guard.md](./phase-04-session-profile-history-compatibility-and-rollout-guard.md) |
| 05 | Completed | Fidelity/perf test gates (IME, Ctrl+C, fullscreen, latency) | [phase-05-fidelity-and-performance-test-gates.md](./phase-05-fidelity-and-performance-test-gates.md) |
| 06 | Completed | Rollout, rollback, Windows follow-up | [phase-06-rollout-rollback-and-windows-followup.md](./phase-06-rollout-rollback-and-windows-followup.md) |

## Latest Progress (2026-03-05)
1. `window-wezterm-gui` + `proxy-wezterm-session` đã chạy ổn định theo mô hình bridge process.
2. `make window` đã dùng đường chạy WezTerm GUI mặc định.
3. Đã thêm smoke mới `scripts/smoke/window-wezterm-gui-smoke.sh` và nối vào `make smoke-window`.
4. Proxy hardening:
   - decode input theo pending buffer để tránh vỡ UTF-8 boundary.
   - fair-drain input/event + input batching để giảm starvation/backpressure.
   - auto-create session khi workspace trống để first-run không fail.
5. Đã cut command surface sang GUI path:
   - bỏ `window-wezterm` và `make window-legacy`.
   - chỉ giữ `window-wezterm-gui` làm entrypoint window chính.
6. Đã thêm guardrails phase 04:
   - `CHATMINAL_WINDOW_BACKEND=wezterm-gui|legacy`.
   - script verify `scripts/migration/phase08-wezterm-gui-killswitch-verify.sh`.
   - compatibility matrix report tại `reports/ipc-compatibility-matrix.md`.
7. Đã có evidence phase 05 Linux:
   - phase03 matrix smoke pass (strict required cases gồm `ctrl-c-burst`, `stress-paste`).
   - phase06 input/IME smoke pass theo auto-gate; manual host sign-off được tách thành external release preflight.
   - phase05 soak + release dry-run pass và lưu artifact trong `reports/`.
   - phase02 bench hard-gate pass (`p95=8.688ms`, `p99=13.225ms`, `pass_fail_gate=true`, fail-gate `p95<=50ms`).
8. Đã đóng coding scope của toàn plan; các bước host-specific manual preflight được ghi rõ ở checklist release thay vì giữ TODO mở trong plan implementation.
9. Batch closeout bổ sung:
   - module `window_wezterm_gui/chatminal_ipc_mux_domain` + race tests cho stale timestamp/seq.
   - proxy graceful-detach guard khi `session_input_write` gặp race `session not running`.
   - docs consistency fixes cho phase files/roadmap/changelog.

## Dependency Order
1. Phase 01 lock architecture and acceptance criteria.
2. Phase 02 complete IPC mux adapter before GUI wiring.
3. Phase 03 wire GUI runtime on top of phase-02 adapter.
4. Phase 04 lock compatibility/feature flags before default switch.
5. Phase 05 pass all gates on Linux/macOS.
6. Phase 06 switch default backend and start Windows parity track.

## Release Gate (must pass before default-on)
- IME commit behavior stable (no duplicate commit, no lost commit).
- Ctrl+C behavior equals native WezTerm in foreground job scenarios.
- TUI fullscreen (`vim`, `nvim`, `tmux`, `htop`) redraw/resize/reconnect stable.
- RTT budget: keep or improve existing phase-02 benchmark envelope.

## Decisions Locked
1. Session/profile UX giai đoạn đầu dùng control plane tách rời (dashboard/CLI), terminal pane để WezTerm GUI render thuần.
2. `make window` chuyển mặc định sang `window-wezterm-gui`.
