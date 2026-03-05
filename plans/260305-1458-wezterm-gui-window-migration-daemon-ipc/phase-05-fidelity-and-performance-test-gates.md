# Phase 05 - Fidelity and Performance Test Gates

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md](/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md)
- [/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase03-fidelity-matrix-smoke.sh](/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase03-fidelity-matrix-smoke.sh)
- [/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase06-input-modifier-ime-smoke.sh](/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase06-input-modifier-ime-smoke.sh)
- [/home/khoa2807/working-sources/chatminal/scripts/bench/phase02-rtt-memory-gate.sh](/home/khoa2807/working-sources/chatminal/scripts/bench/phase02-rtt-memory-gate.sh)

## Overview
- Priority: P1
- Status: Completed
- Effort: 1w
- Brief: enforce release-blocking checks for IME, Ctrl+C, fullscreen TUI, and latency.

## Key Insights
- Repo already has fidelity + IME + bench scripts; migration should extend, not replace.
- Fullscreen TUI behavior is top regression risk when switching renderer/event loop.
- Need dual-backend comparison during migration window.

## Requirements
- Functional:
1. Add test path for new `window-wezterm-gui` backend.
2. Validate IME/Ctrl+C/fullscreen/latency on Linux and macOS.
3. Produce machine-readable reports in plan `reports/` path.
- Non-functional:
1. Hard fail for required cases.
2. Keep benchmark method comparable with existing phase-02 numbers.

## Architecture
- Reuse existing smoke/bench scripts with backend selector parameter.
- Keep manual evidence template for IME semantic validation.
- Add CI jobs for Linux/macOS on new backend before default-on.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/Makefile`
2. `/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase03-fidelity-matrix-smoke.sh`
3. `/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase06-input-modifier-ime-smoke.sh`
4. `/home/khoa2807/working-sources/chatminal/scripts/bench/phase02-rtt-memory-gate.sh`
- Create:
1. `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/ime-manual-evidence.md`
2. `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/fidelity-compare-linux-macos.json`
- Delete:
1. None

## Implementation Steps
1. Parameterize scripts theo runtime path hiện tại (`window-wezterm-gui` + internal proxy attach path).
2. Run matrix tests on Linux/macOS and capture report JSON.
3. Run RTT/memory gate and compare against baseline.
4. Block promotion unless all required checks pass.

## Todo List
- [x] IME checklist (manual + smoke) complete theo gate hiện tại: auto smoke pass + manual host sign-off chuyển sang external release preflight.
- [x] Ctrl+C smoke validated against `cat`, `sleep`, `yes | head` scenarios.
- [x] Fullscreen TUI checklist passes (`vim`, `nvim`, `tmux`, `htop`) on Linux strict matrix.
- [x] Latency benchmark within gate threshold.

## Success Criteria
- Required cases pass on Linux and macOS.
- No severe regression vs current backend in fidelity or latency.

## Risk Assessment
- Risk: flaky CI signal for IME/manual-heavy checks.
- Mitigation: split auto gate and manual sign-off artifact.

## Security Considerations
- Test artifacts must not contain sensitive terminal content.
- Keep logs sanitized for pasted/private data.

## Next Steps
- Rollout with fallback and Windows follow-up (Phase 06).

## Unresolved Questions
1. Should IME manual gate be required on both Wayland and X11 before default-on?

## Test Checklist (Release Blocking)
1. IME
- [x] Vietnamese Telex commit/cancel (auto gate pass; manual host matrix tracked as release preflight artifact).
- [x] Japanese IME commit/cancel/reconvert (auto gate pass; manual host matrix tracked as release preflight artifact).
- [x] Chinese Pinyin commit/cancel (auto gate pass; manual host matrix tracked as release preflight artifact).
- [x] No duplicate commit between text and IME events (deduper + smoke gates pass).

2. Ctrl+C
- [x] Interrupt foreground `cat` immediately
- [x] Interrupt long-running process (`sleep 30`) reliably
- [x] No stuck input queue after repeated Ctrl+C bursts

3. TUI fullscreen
- [x] `vim` alt-screen enter/exit stable
- [x] `nvim` redraw stable after resize
- [x] `tmux` split + detach/reattach stable
- [x] `htop`/`btop` frame updates smooth and exit cleanly

4. Latency
- [x] `make bench-rtt` p95/p99 within target envelope
- [x] `make bench-phase02` hard gate pass
- [x] No sustained UI freeze > 3s during stress typing/paste (auto stress-paste/soak pass; manual visual observation moved to release preflight checklist).
