# Deployment Guide

Last updated: 2026-04-09

## Quick Install

```bash
curl -fsSL https://chatminal.com/install | bash
```

## Homebrew Install

Use this repo itself as the tap:

```bash
brew tap Khoa280703/chatminal https://github.com/Khoa280703/chatminal
brew install --cask chatminal
```

Upgrade later with:

```bash
brew update
brew upgrade --cask chatminal
```

Remove the tap with:

```bash
brew uninstall --cask chatminal
brew untap Khoa280703/chatminal
```

Default behavior installs the latest stable GitHub Release. For prerelease tags, pin `CHATMINAL_VERSION` explicitly.

Current curl|bash targets:

- macOS `aarch64`
- macOS `x86_64`
- Linux `x86_64`

Override release version:

```bash
curl -fsSL https://chatminal.com/install | CHATMINAL_VERSION=v0.1.4 bash
```

Override install locations:

```bash
curl -fsSL https://chatminal.com/install | \
CHATMINAL_BIN_DIR="$HOME/.local/bin" \
CHATMINAL_INSTALL_DIR="$HOME/.local/share/chatminal" \
bash
```

## Build Target

- `apps/desktop` (single-process desktop app)

## Installer Release Assets

- `Chatminal-<version>-macos-<arch>.tar.gz`
- `Chatminal-<version>-macos-<arch>.dmg`
- `Chatminal-<version>-linux-<arch>.tar.gz`
- `SHA256SUMS`
- `install.sh`

Windows `.zip` artifacts remain part of the full GitHub Release, but they are not used by the curl|bash installer path.

## Homebrew Cask Asset

- `Casks/chatminal.rb`
- targets macOS `aarch64` and `x86_64`
- pulls `.dmg` assets from the matching GitHub Release tag

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
