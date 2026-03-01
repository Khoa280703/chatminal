---
title: "Chatminal - Messenger-Style Terminal Multiplexer"
description: "Rust/Iced terminal multiplexer with messenger UI: session list sidebar + terminal pane"
status: in-progress
priority: P1
effort: 28h
branch: main
tags: [feature, experimental, rust, terminal, iced]
created: 2026-03-01
---

# Chatminal Implementation Plan

## Overview
- Rust desktop app with messenger-style shell UX: sidebar sessions + active terminal pane
- PTY per session via `portable-pty`, ANSI parse via `vte`, render via `iced` Canvas
- MVP scope: multi-session shell, resize, shortcuts, scrollback, graceful session lifecycle

## Architecture Summary
```text
portable-pty (per session)
  -> std::thread PTY reader/writer (blocking I/O)
  -> vte::Parser -> TerminalGrid (primary + alternate buffers)
  -> tokio::sync::mpsc bounded(4) -> SessionEvent::{Update, Exited}
  -> Iced Subscription -> Message::{TerminalUpdated {.., lines_added}, SessionExited}
  -> app.update() applies snapshots + lifecycle cleanup
  -> input via iced::event::listen() -> SessionManager::send_input()
```

## Phases
| # | Phase | Est. | Status |
|---|-------|------|--------|
| 01 | [Project Setup](phase-01-project-setup.md) | 2h | in-progress |
| 02 | [PTY Session Manager](phase-02-pty-session-manager.md) | 6h | in-progress |
| 03 | [Iced UI Layout](phase-03-iced-ui-layout.md) | 5h | in-progress |
| 04 | [Terminal Rendering](phase-04-terminal-rendering.md) | 6h | in-progress |
| 05 | [Input Handling](phase-05-input-handling.md) | 3h | in-progress |
| 06 | [Virtual Scrolling](phase-06-virtual-scrolling.md) | 3h | in-progress |
| 07 | [Integration & Polish](phase-07-integration-polish.md) | 3h | in-progress |

## Key Dependencies
- Rust toolchain `1.93.1`, crate `edition = "2024"`, `rust-version = "1.93"`
- `iced 0.14.0` (`wgpu`, `tokio`), `portable-pty 0.9.0`, `vte 0.15.0`
- `tokio 1.49.0`, `uuid 1.21.0`, `indexmap 2.13.0`
- `log 0.4.29`, `env_logger 0.11.9`, `libc 0.2.182`
- Config phase deps: `serde 1.0.228`, `toml 1.0.3`, `dirs 6.0.0`

## Codebase Structure
```text
src/
  main.rs
  app.rs
  message.rs
  config.rs
  session/{mod.rs,manager.rs,pty_worker.rs,grid.rs}
  ui/{mod.rs,sidebar.rs,terminal_pane.rs,input_handler.rs,color_palette.rs,theme.rs}
assets/
  JetBrainsMono.ttf
```

## Review Status
- Post-fix sync (2026-03-01 19:24 +07): fixed hardcoded runtime metrics, `ESC M` reverse index, and config numeric bounds clamp.
- PTY queue-full risk mitigated: `Exited` path uses `blocking_send`; `Update` keeps dirty snapshot and retries when queue becomes available.
- New regression tests pass: `reverse_index_esc_m_scrolls_down_from_top_row`, `flush_update_retries_after_queue_full`.
- Latest gate snapshot: `cargo test` 13/13 pass, `cargo clippy -- -D warnings` pass, `cargo build --release` pass, code-review **9.6/10** (`0 critical`, `0 high`).
- Plan remains `in-progress` until remaining manual/integration checklist + medium edge case (flush pending dirty snapshot before `Exited`) are completed. Finish full plan before marking `completed`.
