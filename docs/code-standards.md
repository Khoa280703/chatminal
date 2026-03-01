# Code Standards

Last updated: 2026-03-01
Scope: Rust code in `src/`.

## Core Principles
1. Keep modules narrow and explicit; one clear responsibility per module.
2. Prefer bounded queues and explicit backpressure handling.
3. Preserve terminal correctness before visual polish shortcuts.
4. Use typed domain errors for session/runtime boundaries.
5. Keep behavior testable and add regression tests for stateful logic.

## Module Boundaries
| Layer | Files | Rule |
| --- | --- | --- |
| Bootstrap | `src/main.rs`, `src/config.rs` | Startup and config only; no terminal rendering logic. |
| App state machine | `src/app.rs`, `src/message.rs` | `Message` is the only mutation surface for app runtime state. |
| Session runtime | `src/session/*` | PTY lifecycle, parser state, grid snapshots. |
| Presentation/UI | `src/ui/*` | Rendering and input translation only; do not spawn/manage PTY here. |

## Naming and Data Types
1. Use snake_case for functions/fields and PascalCase for types/enums.
2. Keep public enums exhaustive (`SessionError`, `CursorStyle`, `Message`).
3. Avoid magic numbers when shared across code paths (use constants).
4. Use `String` cell payload (`Cell.c`) for grapheme-safe rendering snapshots.

## Error Handling Rules
1. PTY/session APIs should return `Result<_, SessionError>`.
2. Convert queue states to explicit domain errors (`SessionError::ChannelFull`, `SessionError::ChannelClosed`).
3. Log recoverable runtime issues and keep UI process alive.
4. Do not use unwrap calls in runtime IO/process paths.

## Concurrency Rules
1. Per session: one reader thread and one writer thread.
2. Reader thread must never deadlock on shutdown event delivery.
3. Use bounded `tokio::sync::mpsc` channels for session events and PTY input.
4. Share terminal snapshots with UI as immutable `Arc<TerminalGrid>`.

## Terminal Runtime Rules
1. ANSI/parser state must flow through `wezterm-term` in `src/session/pty_worker.rs`.
2. `SessionEvent::Update` must be non-blocking (`try_send`) with retry semantics when queue is full.
3. EOF/read error must dispatch `SessionEvent::Exited` without blocking reader teardown (spawn sender thread + `blocking_send`).
4. Snapshot extraction should stay bounded to `scrollback window + visible window`.
5. `lines_added` should track stable row delta for scroll offset continuity.

## Rendering Rules
1. `terminal_pane_view` consumes read-only grid snapshots.
2. Cursor rendering must respect style enum (`CursorStyle::Block`, `CursorStyle::Underline`, `CursorStyle::Bar`, `CursorStyle::Hidden`).
3. Underline draw is attribute-driven, including empty/continuation cells.
4. Non-default fg/bg colors must come from resolved palette values.
5. Canvas invalidation must stay generation-based.

## Input Mapping Rules
1. Keep terminal-compatible byte sequences in `src/ui/input_handler.rs`.
2. Maintain coverage for Shift+Tab, Insert, F1..F12, and Alt-prefix handling.
3. Preserve control-symbol combo mapping (`@`, `[`, `\\`, `]`, `^`, `_`, `?`).
4. Keep app-level shortcuts (Alt+N, Alt+W) handled in app layer, not mapper.

## Security Rules
1. Ignore broken-pipe signal at process startup.
2. Validate shell path via `/etc/shells`, canonicalization, executable checks.
3. Enforce PTY input size limit constant in `SessionManager::send_input` before enqueue.
4. Clamp PTY resize dimensions to valid `u16` bounds.
5. Clamp configurable numeric values before runtime use.

## Testing Standards
1. `cargo test` must pass before documenting major behavior updates.
2. Add regression tests for parser/runtime edge cases when fixing bugs.
3. Keep coverage across config, session runtime, input mapping, and renderer logic.
4. Current baseline reference: `23` passing unit tests (2026-03-01).

## Documentation Sync Rules
When session/runtime behavior changes, update at least:
1. `docs/system-architecture.md`
2. `docs/codebase-summary.md`
3. `docs/project-overview-pdr.md`
4. `docs/project-roadmap.md` and/or `docs/development-roadmap.md`
5. `docs/project-changelog.md`
