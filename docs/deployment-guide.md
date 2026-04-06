# Deployment Guide

Last updated: 2026-04-06

## Build Target

- `apps/desktop` (single-process desktop app)

## Prerequisites

- Rust stable (>= 1.93)
- macOS or Linux
- Native dependencies: xcb-util (Linux), Cocoa (macOS)

## Build

```bash
cargo build --release -p desktop
```

## Run

```bash
make window
```

Or directly:

```bash
cargo run --manifest-path apps/desktop/Cargo.toml
```

## Vendor Dependencies (First-Time Setup)

```bash
make bootstrap-terminal-deps
```

## Desktop-Embedded Runtime

- **No daemon/IPC**: Runtime is fully embedded in the desktop process
- **Session lifecycle**: Create, resume, close all in-process
- **Database**: SQLite opened once at startup, shared connection via `Arc<Mutex<Connection>>`
- **Persist worker**: Background thread coalesces database writes, zero lock contention on hot path
- **Scrollback**: 3K lines per session (~22MB RAM), output history 512KB per session

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CHATMINAL_DATA_DIR` | SQLite database location | Platform-specific |
| `CHATMINAL_DEFAULT_SHELL` | Shell to spawn | `$SHELL` |
| `CHATMINAL_DEFAULT_COLS` | Default terminal width | `120` |
| `CHATMINAL_DEFAULT_ROWS` | Default terminal height | `40` |
| `CHATMINAL_HEALTH_INTERVAL_MS` | Health check interval | `5000` |
| `CHATMINAL_WINDOW_BACKEND` | Window backend | `wezterm-gui` |

## Performance Tuning

| Variable | Description | Default |
|----------|-------------|---------|
| `CHATMINAL_MAX_LINES_PER_SESSION` | Scrollback limit per session | `3000` |
| `CHATMINAL_ENFORCE_LIMIT_THROTTLE` | Lines between SQLite enforceLimit calls | `50` |
| `CHATMINAL_OUTPUT_HISTORY_SIZE` | Max output history per session | `512KB` |
| `CHATMINAL_LIVE_OUTPUT_SIZE` | Max live output buffer | `256KB` |

## Build Commands

```bash
# Run the desktop app
make window

# Check the build
make check

# Check just desktop
make check-desktop

# Run tests
make test

# Clean build artifacts
make clean

# Show all commands
make help
```
