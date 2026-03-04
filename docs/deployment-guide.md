# Deployment Guide

Last updated: 2026-03-04

## Build targets
- `apps/chatminald`
- `apps/chatminal-app`

## Prerequisites
- Rust stable
- Linux/macOS

## Build
```bash
cargo build --release --manifest-path apps/chatminald/Cargo.toml
cargo build --release --manifest-path apps/chatminal-app/Cargo.toml
```

## Run
Terminal 1:
```bash
CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminald/Cargo.toml
```

Terminal 2:
```bash
CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminal-app/Cargo.toml -- dashboard-tui-wezterm 120 200 120 32 20
```

## Environment
- `CHATMINAL_DAEMON_ENDPOINT`
- `CHATMINAL_PREVIEW_LINES`
- `CHATMINAL_MAX_LINES_PER_SESSION`
- `CHATMINAL_DEFAULT_COLS`
- `CHATMINAL_DEFAULT_ROWS`
- `CHATMINAL_HEALTH_INTERVAL_MS`
