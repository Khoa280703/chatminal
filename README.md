# Chatminal

Chatminal is a local desktop terminal workspace built with Rust + Iced.
It runs multiple PTY sessions in one window and renders terminal output through `wezterm-term` state snapshots.

Last updated: 2026-03-01

## What It Does
- Multi-session local shell workspace (create, switch, close sessions).
- PTY reader/writer threads per session with bounded channels.
- Terminal parsing/state via `wezterm-term` (+ `wezterm-surface` cursor metadata).
- Scrollback-aware rendering with virtual viewport offsets.
- Keyboard-first workflow with terminal key mapping and app shortcuts.

## Runtime Highlights
- PTY bytes are fed into `Terminal::advance_bytes`.
- UI snapshots are built from `scrollback window + visible window` (not full history scan each flush).
- `lines_added` is calculated from top stable row delta to keep scroll offset stable.
- EOF/read error sends `SessionEvent::Exited` from a short-lived sender thread using `blocking_send`.

## Requirements
- Unix-like OS (uses `/etc/shells` validation and `libc` signal APIs).
- Rust `1.93+`.
- Cargo.

## Quick Start
```bash
cargo build
cargo run
```

Run tests:
```bash
cargo test
```

Current baseline (2026-03-01): `23 passed; 0 failed`.

## Configuration
Optional config file:
`~/.config/chatminal/config.toml`

Example:
```toml
shell = "/bin/bash"
scrollback_lines = 10000
font_size = 14.0
sidebar_width = 240.0
```

Runtime normalization:
- `scrollback_lines`: `100..=200_000`
- `font_size`: `8.0..=48.0`
- `sidebar_width`: `160.0..=640.0`

Notes:
- Invalid/missing config falls back to defaults.
- `shell` must be executable and listed in `/etc/shells` (or canonicalized equivalent).

## Default Controls
- `Alt+N`: New session
- `Alt+W`: Close active session
- `Shift+PageUp`: Scroll up one viewport
- `Shift+PageDown`: Scroll down one viewport
- Mouse wheel: Scroll terminal viewport

Input mapper also supports terminal sequences for keys including `Shift+Tab`, `Insert`, `F1..F12`, `Alt+<key>` prefix, and `Ctrl` symbol combos.

## Troubleshooting
| Symptom | Likely Cause | Action |
| --- | --- | --- |
| Session fails to start | Invalid shell path | Check `shell` in config; ensure shell exists, executable, and present in `/etc/shells`. |
| No output / session exits fast | Shell exits immediately | Run `RUST_LOG=info cargo run` and verify shell command works standalone. |
| Input feels dropped | Input/event channel pressure | Retry after output settles; inspect warnings for queue pressure. |
| Scrollback not moving | Active buffer is alternate screen | Exit full-screen TUI app or return to primary screen. |

## Project Documentation
- [Docs index](./docs/index.md)
- [Project overview & PDR](./docs/project-overview-pdr.md)
- [System architecture](./docs/system-architecture.md)
- [Codebase summary](./docs/codebase-summary.md)
- [Code standards](./docs/code-standards.md)
- [Project roadmap](./docs/project-roadmap.md)
- [Development roadmap](./docs/development-roadmap.md)
- [Project changelog](./docs/project-changelog.md)
