# Phase 01 - Project Setup

## Context Links
- Plan: [plan.md](plan.md)
- Research: [PTY Libraries](../reports/researcher-260301-0820-rust-pty-terminal-libraries.md)
- Research: [UI Frameworks](../reports/researcher-260301-0820-rust-ui-frameworks-evaluation.md)

## Overview
- **Priority:** P1 (blocker for all phases)
- **Status:** in-progress
- **Effort:** 2h
- **Goal:** Compilable Rust workspace with all deps, correct feature flags, and directory scaffold

## Key Insights
- Iced 0.14 requires explicit `wgpu` feature flag for GPU backend
- `portable-pty` 0.9 exposes only `default` and `serde_support` features (no `ssh` feature to disable)
- Must enable `tokio` multi-thread runtime (PTY I/O is concurrent)
- Keep `main.rs` thin — all logic in modules

## Requirements
- Rust edition 2024 + `rust-version` pinned to stable toolchain, binary crate (not workspace yet)
- All MVP crate deps declared with pinned versions (latest stable snapshot on 2026-03-01)
- Build compiles without warnings on Linux
- Directory structure matches plan

## Architecture

```
chatminal/
├── Cargo.toml
├── Cargo.lock          # commit this (binary app)
├── assets/
│   └── JetBrainsMono.ttf   # embedded via include_bytes! in main.rs
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── message.rs
│   ├── config.rs
│   ├── session/
│   │   ├── mod.rs
│   │   ├── manager.rs
│   │   ├── pty_worker.rs
│   │   └── grid.rs
│   └── ui/
│       ├── mod.rs
│       ├── sidebar.rs
│       ├── terminal_pane.rs
│       ├── input_handler.rs
│       ├── color_palette.rs
│       └── theme.rs
```

## Related Code Files
- **Create:** `/home/khoa2807/working-sources/chatminal/Cargo.toml`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/main.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/app.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/message.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/config.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/session/mod.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/session/manager.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/session/pty_worker.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/session/grid.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/ui/mod.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/ui/sidebar.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/ui/terminal_pane.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/ui/input_handler.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/ui/color_palette.rs`
- **Create:** `/home/khoa2807/working-sources/chatminal/src/ui/theme.rs`

## Implementation Steps

1. **Init Cargo project**
   ```bash
   cargo init chatminal --name chatminal
   ```

2. **Write Cargo.toml** with dependencies:
   ```toml
   [package]
   name = "chatminal"
   version = "0.1.0"
   edition = "2024"
   rust-version = "1.93"

   [dependencies]
   iced = { version = "0.14.0", features = ["wgpu", "tokio"] }
   portable-pty = "0.9.0"
   vte = "0.15.0"
   tokio = { version = "1.49.0", features = ["full"] }
   uuid = { version = "1.21.0", features = ["v4"] }
   indexmap = "2.13.0"
   log = "0.4.29"
   env_logger = "0.11.9"
   libc = "0.2.182"
   # 256-color mapping: use static XTERM_PALETTE table in color_palette.rs — no external dep needed
   # ansi_colours crate REMOVED: dead dependency (color_palette.rs uses static array directly)

   [profile.release]
   opt-level = 3
   lto = true
   codegen-units = 1
   ```
   > Note: versions above validated from crates.io on 2026-03-01

3. **Download JetBrains Mono font**
   ```bash
   mkdir -p assets
   curl -L "https://github.com/JetBrains/JetBrainsMono/releases/latest/download/JetBrainsMono-2.304.zip" -o /tmp/jbmono.zip
   unzip -j /tmp/jbmono.zip "fonts/ttf/JetBrainsMono-Regular.ttf" -d assets/
   mv assets/JetBrainsMono-Regular.ttf assets/JetBrainsMono.ttf
   rm /tmp/jbmono.zip
   ```
   — Or manually place any monospace TTF at `assets/JetBrainsMono.ttf`; file name must match `include_bytes!("../assets/JetBrainsMono.ttf")` in `main.rs`

4. **Create stub `main.rs`** — just `iced::application(...)` call, compiles, shows empty window

5. **Create stub modules** — each file with `// TODO` placeholder, correct `pub mod` declarations

6. **Verify compile:** `cargo check` must pass with 0 errors

7. **Add `.gitignore`** — exclude `target/`, keep `Cargo.lock`

## Todo List
- [x] `cargo init` project
- [x] Write `Cargo.toml` with all deps + feature flags
- [x] Verify `portable-pty` exact version on crates.io
- [x] Verify `iced` 0.14 feature flag names (`wgpu` vs `wgpu_backend`)
- [ ] Download JetBrains Mono TTF → `assets/JetBrainsMono.ttf`
- [x] Create all stub source files with correct module declarations
- [x] `cargo check` passes
- [x] `cargo clippy` no errors (warnings OK at this stage)

## Success Criteria
- `cargo check` exits 0
- Module tree matches planned structure
- All imports resolvable (no missing crate errors)

## Risk Assessment
- **Iced 0.14 API may differ** from 0.13 docs → check CHANGELOG before coding
- **Upstream API drift** — re-check crates.io before implementation if plan date is stale
- **wgpu backend** may need system Vulkan drivers on Linux → document in README

## Security Considerations
- Run `cargo-audit` (`cargo install cargo-audit --version 0.22.1`) before first commit — terminal multiplexer (PTY access) là high-value supply chain target
- Add `cargo-deny` (`cargo install cargo-deny --version 0.19.0`) với allowlist để block unlicensed/compromised crates
- Phase 01 security NOT N/A: lock crate versions, verify hashes
