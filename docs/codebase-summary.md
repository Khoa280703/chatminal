# Codebase Summary

Last updated: 2026-03-04

## Runtime baseline
Chatminal hiện chỉ dùng runtime native Rust:
- `apps/chatminald`
- `apps/chatminal-app`
- `crates/chatminal-protocol`
- `crates/chatminal-store`

## High-signal files
- `apps/chatminald/src/main.rs`: daemon entrypoint
- `apps/chatminald/src/server.rs`: local IPC server loop
- `apps/chatminald/src/state.rs`: request handling + runtime state machine
- `apps/chatminald/src/session.rs`: PTY wrapper per session
- `apps/chatminald/src/config.rs`: daemon env/default config
- `apps/chatminal-app/src/main.rs`: CLI command router
- `apps/chatminal-app/src/terminal_wezterm_core.rs`: wezterm-term adapter
- `apps/chatminal-app/src/terminal_wezterm_dashboard_tui.rs`: interactive TUI dashboard
- `crates/chatminal-protocol/src/lib.rs`: protocol contracts
- `crates/chatminal-store/src/lib.rs`: SQLite persistence API

## Current risk
- `apps/chatminald/src/state.rs` vẫn có global mutex scope rộng; tải cao có thể contention.
