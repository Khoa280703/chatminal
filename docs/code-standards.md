# Code Standards

Last updated: 2026-04-06

Scope: `apps/desktop` + `crates/*`.

## Principles

1. Keep desktop as single source of truth for session/profile/history state (single-process).
2. Keep PTY hot path non-blocking; DB writes go through async persist worker (background thread, zero lock contention).
3. Keep code straightforward, avoid feature creep.
4. Follow YAGNI (You Aren't Gonna Need It) and KISS (Keep It Simple, Stupid).

## Boundaries

- `apps/desktop`: Desktop app, session manager, PTY runtime, GUI shell, native API.
- `crates/runtime`: Embedded orchestrator, session state, startup recipes, persist worker, store facade.
- `crates/store`: SQLite schema and CRUD operations.
- `crates/terminal-emulator`: Terminal parser, state, and input core.

## Naming

- Rust: Use `snake_case` for functions and fields; use PascalCase for types.
- Protocol fields: Keep `snake_case` for consistent serde payloads.

## Runtime Rules

1. Desktop app does not spawn shells outside the process; PTY is created through LeafRuntime.
2. Session lifecycle is entirely within the desktop process (no IPC).
3. History retention must go through store policy (`max lines`, output history cap 512KB).
4. Persist writes must go through background worker thread (zero lock contention on hot path).
5. Startup recipes:
   - Per-session registry, persisted in SQLite.
   - Executed before first prompt (run/type/enter/wait/wait-for sequence).
   - Available for inspection via native API.
6. Desktop invariants:
   - Single Window, single Tab, multiple Panes mapped to sessions.
   - Session owns Pane via `session_id → Arc<SessionPane>` lookup.
   - No public Tab/Pane surface (private adapter zone only).

## Validation Commands

```bash
cargo check --workspace
cargo check -p desktop
cargo test -p runtime
cargo test --manifest-path crates/store/Cargo.toml
```
