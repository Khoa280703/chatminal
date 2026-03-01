# Deployment Guide

Last updated: 2026-03-01

## Environment
- OS: Unix-like (uses `/etc/shells` and `libc` signal APIs).
- Rust: `1.93+`.
- Cargo: bundled with Rust toolchain.
- Network access required for initial dependency fetch (includes git deps: `wezterm-term`, `wezterm-surface`).

## Local Build and Run
```bash
cargo build
cargo run
```

## Test Verification
```bash
cargo test
```
Expected current baseline (2026-03-01): `23 passed; 0 failed`.

## Release Build
```bash
cargo build --release
```
Binary output:
- `target/release/chatminal`

Release profile in `Cargo.toml`:
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

Runtime normalization:
- `scrollback_lines`: `100..=200_000`
- `font_size`: `8.0..=48.0`
- `sidebar_width`: `160.0..=640.0`

## Smoke Checklist
1. App starts and creates first session automatically.
2. Session shortcuts work (`Alt+N`, `Alt+W`).
3. Scroll controls work (mouse wheel, `Shift+PageUp/PageDown`).
4. ANSI rendering works (`ls --color=auto`, simple SGR tests).
5. Cursor visibility/style changes are reflected (block/underline/bar/hidden).

## Troubleshooting
| Symptom | Likely Cause | Action |
| --- | --- | --- |
| Session cannot start | Invalid shell config | Check `shell` value and `/etc/shells` allowlist. |
| Build fails on clean machine | Missing toolchain or network for git deps | Install Rust 1.93+, ensure outbound access for Cargo git dependencies. |
| Session exits immediately | Shell process exits early | Run app with logs: `RUST_LOG=info cargo run`. |
| UI not updating under heavy output | Event queue pressure | Check warnings about full update queue; rerun stress scenario. |

## Packaging Status
No repository-managed packaging scripts are present yet.
Packaging remains part of Phase 8 roadmap scope.
