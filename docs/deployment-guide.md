# Deployment Guide

Last updated: 2026-03-31 (single-process desktop model)

## Build target
- `apps/chatminal-desktop` (single-process desktop app)

## Prerequisites
- Rust stable (recommended >= 1.93)
- Linux/macOS (validated local)
- Native deps: xcb-util (Linux), Cocoa (macOS)

## Build
```bash
cargo build --release -p chatminal-desktop
```

## Run
```bash
make window
```

Or directly:
```bash
CHATMINAL_DESKTOP_SESSIONS_SIDEBAR=1 \
cargo run --manifest-path apps/chatminal-desktop/Cargo.toml -- start -- chatminal-runtime proxy-desktop-session
```

## Vendor deps (first-time setup)
```bash
make bootstrap-terminal-deps
```

## Desktop-embedded runtime
- No daemon/IPC: runtime fully embedded in desktop process
- Session lifecycle: create, resume, close all in-process
- Database: SQLite opened once at startup, shared connection via Arc<Mutex<Connection>>
- Persist worker: background thread, coalesces database writes, zero lock contention on hot path
- Scrollback: 3K lines per session (~22MB RAM), output history 512KB per session

## Environment
- `CHATMINAL_DATA_DIR` — SQLite database location (default: ~/.local/share/chatminal on Linux, ~/Library/Application Support/Chatminal on macOS)
- `CHATMINAL_DEFAULT_SHELL` — Shell to spawn (default: $SHELL)
- `CHATMINAL_DEFAULT_COLS` — Default terminal width (default: 120)
- `CHATMINAL_DEFAULT_ROWS` — Default terminal height (default: 40)
- `CHATMINAL_HEALTH_INTERVAL_MS` — Health check interval (default: 5000)
- `CHATMINAL_DESKTOP_SESSIONS_SIDEBAR` — Enable sessions sidebar (default: 1)
- `CHATMINAL_WINDOW_BACKEND` — Window backend: `wezterm-gui` (default) or `legacy`

## Performance tuning
- `CHATMINAL_MAX_LINES_PER_SESSION` — Scrollback limit per session (default: 3000, ~22MB per session)
- `CHATMINAL_ENFORCE_LIMIT_THROTTLE` — Lines between SQLite enforceLimit calls (default: 50)
- `CHATMINAL_OUTPUT_HISTORY_SIZE` — Max output history per session (default: 512KB)
- `CHATMINAL_LIVE_OUTPUT_SIZE` — Max live output buffer (default: 256KB)
