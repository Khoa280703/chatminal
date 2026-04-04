---
phase: 01
status: completed
priority: high
effort: small
risk: low
---

# Phase 01: Prune Dead WezTerm Code

## Overview
Xóa dead code từ WezTerm: SSH, TLS, WSL, auto-update config stubs + unused engine-* crates. ~5K+ LOC removal.

## Key Insights
- 30+ config fields tồn tại cho SSH/TLS/WSL/auto-update với ZERO implementation
- 10/18 engine-* crates không được import trực tiếp bởi desktop app
- Serial port: minimal CLI-only, giữ lại (có user)
- Mux Domains (remote routing): hoàn toàn absent — không cần xóa

## Steps

### 1. Xóa dead config files
- `crates/chatminal-config/src/ssh.rs` (162 LOC) — zero usage
- `crates/chatminal-config/src/tls.rs` (106 LOC) — zero usage
- `crates/chatminal-config/src/wsl.rs` (212 LOC) — zero usage
- Remove `mod ssh; mod tls; mod wsl;` from `crates/chatminal-config/src/lib.rs`
- Remove `pub use ssh::*; pub use tls::*; pub use wsl::*;` exports

### 2. Clean config.rs fields
Remove from `crates/chatminal-config/src/config.rs`:
- Line ~343: `pub wsl_targets: Option<Vec<WslTarget>>`
- Line ~349: `pub serial_ports: Vec<SerialTarget>` — **KEEP** (CLI integration exists)
- Line ~353: `pub unix_targets: Vec<UnixTargetConfig>` — **KEEP** (local socket multiplexer)
- Line ~356: `pub ssh_targets: Option<Vec<SshTarget>>`
- Line ~359: `pub ssh_backend: SshBackend`
- Line ~364: `pub tls_servers: Vec<TlsTargetServer>`
- Line ~368: `pub tls_clients: Vec<TlsTargetClient>`
- Line ~386: `pub mux_enable_ssh_agent: bool`
- Line ~389: `pub default_ssh_auth_sock: Option<String>`
- Line ~720: `pub check_for_updates: bool`
- Line ~725: `pub show_update_window: bool`
- Line ~728: `pub check_for_updates_interval_seconds: u64`

### 3. Remove default functions
- `default_check_for_updates()` (config.rs ~1597)
- `default_update_interval()` (config.rs)
- Any SSH/TLS/WSL related default functions

### 4. Audit unused engine-* crates
Check transitive deps before removing. For each:
```
cargo check --workspace 2>&1
```
After removing from workspace members + Cargo.toml deps.

**Candidates (0 direct imports):**
- chatminal-gui-subcommands — check if CLI uses it
- chatminal-uds — unix domain socket, check if portable-pty uses
- chatminal-toast-notification — check if desktop uses

**DO NOT remove (confirmed active usage):**
- engine-version — `config::engine_version()` used in `leaf_runtime.rs:136`
- engine-cell, engine-char-props, engine-color-types, engine-escape-parser, engine-surface — transitive deps of engine-term
- engine-config-derive, engine-dynamic, engine-dynamic-derive — used by config

### 5. Remove libssh dependency
- Remove `libssh-rs` from workspace Cargo.toml (if SSH code deleted)
- Remove `libssh2` if only used by SSH
- Run `cargo check --workspace` to verify

## Verification
```bash
cargo check --workspace
cargo test --workspace --lib --bins --tests
grep -r "SshTarget\|TlsTarget\|WslTarget\|check_for_updates" crates/ apps/ --include="*.rs"
```

## Completed
- Deleted dead config modules `tls.rs` and `wsl.rs`
- Replaced former `ssh.rs` config surface with minimal compile-path helpers only:
  - `SshParameters`
  - `username_from_env()`
- Removed dead config fields from `Config`:
  - `wsl_targets`
  - `ssh_targets`
  - `ssh_backend`
  - `tls_servers`
  - `tls_clients`
  - `mux_enable_ssh_agent`
  - `default_ssh_auth_sock`
  - `check_for_updates`
  - `show_update_window`
  - `check_for_updates_interval_seconds`
- Removed Lua API `default_wsl_targets`
- Removed WSL branch from `chatminal-host-runtime/src/spawn_target.rs`
- Removed `libssh-rs` from root workspace dependencies
- Audited engine crate candidates:
  - `engine-gui-subcommands`: active, kept
  - `engine-toast-notification`: active, kept
  - `engine-uds`: no direct product reference found in this phase, left untouched pending wider rename/config work

## Success Criteria
- [x] Zero references to removed SSH/TLS/WSL config types in codebase
- [x] Zero auto-update config fields
- [x] `cargo check --workspace` clean
- [ ] Binary size reduction measurable
