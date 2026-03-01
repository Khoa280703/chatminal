# Deployment Guide

Last updated: 2026-03-01

## Environment
- OS: Unix-like environment (uses `/etc/shells`, `libc` signal APIs).
- Rust: `1.93+`.
- Cargo: bundled with Rust toolchain.

## Local Development
1. Install dependencies:
   - `rustup toolchain install 1.93.0`
2. Build debug binary:
   - `cargo build`
3. Run app:
   - `cargo run`
4. Run tests:
   - `cargo test`
   - Expected current baseline: `13 passed; 0 failed`

## Release Build
1. Build optimized binary:
   - `cargo build --release`
2. Output binary:
   - `target/release/chatminal`

`Cargo.toml` release profile is preconfigured with:
- `opt-level = 3`
- `lto = true`
- `codegen-units = 1`

## Runtime Configuration
Optional config path:
- `~/.config/chatminal/config.toml`

Example:
```toml
shell = "/bin/bash"
scrollback_lines = 10000
font_size = 14.0
sidebar_width = 240.0
```

Notes:
- If file is missing or invalid TOML, app falls back to defaults.
- `shell` must resolve to an executable path allowed by `/etc/shells`.
- Numeric values are normalized/clamped at runtime:
  - `scrollback_lines`: `100..=200_000`
  - `font_size`: `8.0..=48.0` (non-finite values fallback to default)
  - `sidebar_width`: `160.0..=640.0` (non-finite values fallback to default)

## Operational Checks
1. Verify app starts and auto-opens first session.
2. Verify session shortcuts:
   - `Alt+N` new session
   - `Alt+W` close session
3. Verify scrolling:
   - mouse wheel
   - `Shift+PageUp`, `Shift+PageDown`
4. Verify output color/style rendering with command examples:
   - `ls --color=auto`
   - `printf '\e[31mred\e[0m\n'`

## Troubleshooting
| Symptom | Likely Cause | Action |
| --- | --- | --- |
| Session does not start | Invalid shell path | Remove/adjust `shell` in config; ensure path is listed in `/etc/shells`. |
| App exits on startup | Platform mismatch or dependency issue | Run from terminal and inspect logs with `RUST_LOG=debug cargo run`. |
| No terminal output | PTY child exited quickly | Check shell validity and permissions. |
| Keyboard input ignored | No active session | Select session from sidebar or create new one. |

## Packaging Notes
Current repo does not include installer scripts or OS packaging metadata.
For distribution, add platform-specific packaging files (deb/rpm/homebrew/etc.) in a separate release phase.
