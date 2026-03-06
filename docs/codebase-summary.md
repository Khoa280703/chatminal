# Codebase Summary

Last updated: 2026-03-05

## Runtime baseline
Chatminal hiện chỉ dùng runtime native Rust:
- `apps/chatminald`
- `apps/chatminal-app`
- `crates/chatminal-terminal-core`
- `crates/chatminal-protocol`
- `crates/chatminal-store`

## High-signal files
- `apps/chatminald/src/main.rs`: daemon entrypoint
- `apps/chatminald/src/server.rs`: local IPC server loop
- `apps/chatminald/src/state.rs`: request handling + runtime state machine
- `apps/chatminald/src/state/request_handler.rs`: daemon request dispatch logic (request frame -> response frame)
- `apps/chatminald/src/state/explorer_utils.rs`: session-explorer path normalization and root-boundary guards
- `apps/chatminald/src/state/session_explorer.rs`: session explorer request handlers (state/list/read/update)
- `apps/chatminald/src/state/runtime_lifecycle.rs`: active-runtime ensure/publish workspace/session updates + broadcast helpers
- `apps/chatminald/src/state/session_event_processor.rs`: PTY output/exited/error event processing path
- `apps/chatminald/src/state/tests.rs`: daemon state tests moved out from runtime file
- `apps/chatminald/src/session.rs`: PTY wrapper per session
- `apps/chatminald/src/config.rs`: daemon env/default config
- `apps/chatminald/src/transport/unix.rs`: UDS transport backend
- `apps/chatminald/src/transport/windows.rs`: Named Pipe transport backend
- `apps/chatminal-app/src/main.rs`: CLI command router
- `apps/chatminal-app/src/ipc/transport/unix.rs`: UDS client connector
- `apps/chatminal-app/src/ipc/transport/windows.rs`: Named Pipe client connector
- `apps/chatminal-app/src/input/pty_key_translator.rs`: key event -> PTY byte translation
- `apps/chatminal-app/src/terminal_wezterm_attach_frame_renderer.rs`: attach TUI frame rendering utilities
- `apps/chatminal-app/src/terminal_quality_benchmark/runner.rs`: RTT benchmark runner (`bench-rtt-wezterm`)
- `apps/chatminal-app/src/terminal_quality_benchmark/stats.rs`: percentile/statistics + benchmark report
- `apps/chatminal-app/src/terminal_wezterm_core.rs`: terminal pane adapter (dùng terminal core nội bộ)
- `crates/chatminal-terminal-core/src/lib.rs`: internal terminal parser/state wrapper
- `apps/chatminal-app/src/terminal_wezterm_dashboard_tui.rs`: interactive TUI dashboard
- `apps/chatminal-app/src/window/native_window_wezterm.rs`: eframe window shell
- `apps/chatminal-app/src/window/native_window_wezterm_controller.rs`: window state hydration/event sync
- `apps/chatminal-app/src/window/native_window_wezterm_actions.rs`: window session actions/input/resize
- `apps/chatminal-app/src/window/native_window_wezterm_reducer.rs`: pure reducer logic + unit tests cho window flow
- `crates/chatminal-protocol/src/lib.rs`: protocol contracts
- `crates/chatminal-store/src/lib.rs`: SQLite persistence API
- `apps/chatminald/src/metrics.rs`: daemon runtime counters (request/event/broadcast/drop + input backpressure)
- `scripts/bench/phase02-rtt-memory-gate.sh`: phase-02 RTT/RSS hard gate script
- `scripts/fidelity/phase03-fidelity-matrix-smoke.sh`: phase-03 fidelity smoke (required cases + JSON report)
- `scripts/soak/phase05-soak-smoke.sh`: phase-05 soak smoke (`pr|nightly`) + JSON envelope

## Current risk
- `apps/chatminald/src/state.rs` vẫn có global mutex scope rộng; tải cao có thể contention.
