---
title: "Chatminal WezTerm Input Pipeline Daemon-First Plan"
description: "Kế hoạch hardening input pipeline theo kiến trúc WezTerm để đạt terminal behavior truyền thống mà không phá runtime hiện tại."
status: completed
priority: P1
effort: 6w
branch: main
tags: [chatminal, wezterm, input-pipeline, ime, daemon, linux, macos, windows]
created: 2026-03-05
---

# Tổng quan
Mục tiêu: đưa hành vi terminal về gần terminal truyền thống (Ctrl+C, key modifiers, IME tiếng Việt/Japanese/Chinese), giữ daemon model bắt buộc, bám runtime hiện tại `apps/chatminal-app + apps/chatminald + crates/chatminal-protocol + crates/chatminal-store`.

## Scope chốt
- Không cho client spawn shell trực tiếp; mọi input đi qua daemon (`SessionInputWrite`).
- Ưu tiên Linux + macOS; Windows triển khai sau khi Linux/macOS ổn định.
- Không rewrite toàn bộ runtime; làm incremental, có fallback/rollback rõ.

## WezTerm reference principles
- Tách `RawKeyEvent` và `KeyEvent`/composed text path.
- Chuẩn hóa encode cho modifiers + control keys theo `xterm` baseline; `kitty CSI-u` chỉ bật opt-in khi cần.
- IME commit/preedit đi theo text pipeline riêng, không trộn với key shortcut pipeline.

## Quality gates (release-blocking)
| Gate | Target | Hard fail |
| --- | --- | --- |
| Input RTT | p95 <= 30ms, p99 <= 60ms | p95 > 45ms |
| Memory RSS | daemon <= 120MB, app <= 180MB, total <= 300MB | daemon > 160MB, app > 220MB, total > 350MB |
| Fidelity matrix | required cases pass, required skip = 0 | bất kỳ required case fail/skip |
| Soak test | 2h run, không crash/deadlock/input loss | crash, freeze > 3s, event drop tăng liên tục |

## Phase map
| Phase | Status | Trọng tâm | Tài liệu |
| --- | --- | --- | --- |
| 01 | Completed | Baseline/gap map theo WezTerm input architecture | [phase-01-baseline-wezterm-input-gap-map.md](./phase-01-baseline-wezterm-input-gap-map.md) |
| 02 | Completed | Shared input translation layer Linux/macOS | [phase-02-linux-macos-shared-input-translation-layer.md](./phase-02-linux-macos-shared-input-translation-layer.md) |
| 03 | Completed | IME composition + multilingual text pipeline | [phase-03-linux-macos-ime-composition-path.md](./phase-03-linux-macos-ime-composition-path.md) |
| 04 | Completed | Daemon contract/backpressure/runtime safety | [phase-04-daemon-contract-and-runtime-stability.md](./phase-04-daemon-contract-and-runtime-stability.md) |
| 05 | Completed | Quality gates: RTT/memory/fidelity/soak | [phase-05-quality-gates-fidelity-matrix-soak.md](./phase-05-quality-gates-fidelity-matrix-soak.md) |
| 06 | Completed | Migration rollout + rollback + test checklist | [phase-06-migration-rollout-rollback-and-test-checklist.md](./phase-06-migration-rollout-rollback-and-test-checklist.md) |
| 07 | Completed | Windows parity follow-up (non-blocking initial ship, CI lane dùng làm early-warning) | [phase-07-windows-input-parity-follow-up.md](./phase-07-windows-input-parity-follow-up.md) |

## Progress Sync (2026-03-05)
- Latest rereview + tester final verification: `Critical=0`, `High=0` for current HEAD scope.
- No open severity trong scope plan này: `Critical=0`, `High=0`, `Medium=0`.

## Dependency order
1. Phase 01 xong mới chốt design chi tiết cho Phase 02/03.
2. Phase 02 + 03 xong mới khóa daemon/runtime adjustments (Phase 04).
3. Phase 04 xong mới bật hard quality gates mới (Phase 05).
4. Phase 05 pass liên tiếp mới cho migration default-on (Phase 06).
5. Windows phase chạy sau Linux/macOS stable (Phase 07).

## Definition of Done
- Ctrl+C/modifiers/IME behavior khớp checklist terminal truyền thống trên Linux/macOS.
- Daemon-first invariant vẫn giữ, protocol/store không vỡ backward compatibility.
- Pass full gate suite 2 vòng liên tiếp + có rollback đã diễn tập.

## Decisions Locked
1. Không bật `kitty keyboard protocol` mặc định ở Linux/macOS trong wave đầu.
2. Không bắt buộc cover full IME matrix trên cả Wayland + X11 ngay wave đầu; triển khai theo wave.
