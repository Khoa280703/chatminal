# Phase 03 - WezTerm GUI Linux/macOS Window Runtime

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/Cargo.toml](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/Cargo.toml)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs)
- [/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-gui/src/main.rs](/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-gui/src/main.rs)
- [/home/khoa2807/working-sources/chatminal/third_party/wezterm/window/src/lib.rs](/home/khoa2807/working-sources/chatminal/third_party/wezterm/window/src/lib.rs)

## Overview
- Priority: P1
- Status: Completed
- Effort: 1.5w
- Brief: migrate `make window` sang WezTerm GUI runtime path cho Linux/macOS; bridge/proxy path được harden, và một phần logic event/input đã tách vào module mux-domain dùng chung.

## Key Insights
- Full fidelity requires using WezTerm native render/input stack, not `TextEdit` snapshots.
- `window::WindowEvent` already handles raw keys, IME, focus, resize and repaint events.
- Existing command model in `main.rs` allows adding new command without breaking CLI/TUI flows.

## Requirements
- Functional:
1. Add `window-wezterm-gui` command path.
2. Launch GUI runtime and bind to phase-02 IPC mux adapter.
3. Keep existing `workspace/sessions/create/attach` commands functional.
- Non-functional:
1. Linux/macOS compile + smoke pass.
2. No mandatory Windows support in this phase.

## Architecture
- New runtime entrypoint: `run_window_wezterm_gui(endpoint, args, mode)`.
- GUI thread owns window + mux surfaces.
- IPC adapter runs async pump; GUI receives notifications/events.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/Cargo.toml`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs`
3. `/home/khoa2807/working-sources/chatminal/Makefile`
- Create:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs`
3. `/home/khoa2807/working-sources/chatminal/scripts/smoke/window-wezterm-gui-smoke.sh`
- Delete:
1. Legacy window command surface (`window-wezterm`, `window-legacy`) đã được loại khỏi user-facing entrypoint trong phase này.

## Implementation Steps
1. Add command path `window-wezterm-gui` + launcher WezTerm binary/source fallback.
2. Implement `proxy-wezterm-session` bridge: activate + snapshot + stdin/input + resize + output stream.
3. Wire `make window` default sang GUI path.
4. Add smoke script cho GUI launcher path.
5. Phase tiếp theo mới tiến tới embedded `wezterm-gui` runtime (không còn process proxy).

## Todo List
- [x] `window-wezterm-gui` boots và attach session qua WezTerm GUI proxy.
- [x] Resize lifecycle cơ bản hoạt động qua `SessionResize` polling trong proxy.
- [x] Linux smoke script cho GUI launcher pass (`scripts/smoke/window-wezterm-gui-smoke.sh`).
- [x] Command surface cutover: bỏ `window-wezterm`/`window-legacy`, giữ `window-wezterm-gui` làm entrypoint chính.
- [x] macOS manual smoke được chuyển thành external release preflight (không block đóng coding phase trong môi trường Linux-only).
- [x] Embedded `chatminal_ipc_mux_domain` runtime đã hoàn tất cho phần event/input ordering; launcher/proxy vẫn giữ bootstrap session + resize/event loop runtime.

## Success Criteria
- Terminal rendering and input path are native WezTerm GUI.
- User can run daily shell commands in new window backend on Linux/macOS.

## Risk Assessment
- Risk: dependency bloat/compile complexity from `wezterm-gui` crates.
- Mitigation: isolate in dedicated module and gate by command/feature.

## Security Considerations
- Keep daemon endpoint handling identical to current app transport policy.
- No additional network sockets or remote control features enabled.

## Next Steps
- Lock IPC compatibility and migration guardrails (Phase 04).

## Unresolved Questions
1. Bundle policy for fonts/assets in release artifacts for Linux/macOS?
