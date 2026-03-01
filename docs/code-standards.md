# Code Standards

Last updated: 2026-03-01
Scope: Current Rust implementation in `src/`.

## Principles
1. Keep modules focused and explicit (single responsibility).
2. Prefer bounded channels and explicit backpressure over unbounded queues.
3. Fail safely with typed errors (Result + SessionError) and structured logs.
4. Preserve terminal correctness over visual shortcuts.
5. Keep changes testable and add unit tests for stateful logic.

## Module Structure
| Layer | Files | Rule |
| --- | --- | --- |
| Entry/bootstrap | `src/main.rs`, `src/config.rs` | No business logic in `main`; config parsing isolated in config module. |
| State machine | `src/app.rs`, `src/message.rs` | `Message` is single mutation surface for app state. |
| Session runtime | `src/session/*` | PTY lifecycle + parsing logic isolated from UI widgets. |
| UI rendering | `src/ui/*` | UI code must not spawn processes or own PTY resources. |

## Naming and Types
1. Use `snake_case` for functions and fields.
2. Use PascalCase for structs/enums (`AppState`, `SessionManager`, `TerminalGrid`).
3. Keep message and error enums explicit and exhaustive.
4. Avoid implicit numeric magic; define constants (for example max input bytes and theme values).

## Error Handling Pattern
1. Return typed errors from session control paths:
   - `SessionManager::create_session -> Result<SessionId, SessionError>`
   - `SessionManager::send_input -> Result<(), SessionError>`
2. UI layer logs recoverable failures (`log::warn!`, `log::error!`) and keeps app alive.
3. Never use unwrap in runtime code paths that can fail from IO/process behavior.
4. Convert low-level IO errors into domain errors with clear context.

## Concurrency and Threading Rules
1. Reader/writer threads are per session and must be joined on close.
2. Inter-thread payloads must be bounded and validated before enqueue.
3. Do not hold global mutex locks across await points in UI runtime.
4. Snapshot sharing to UI must use immutable data (`Arc<TerminalGrid>`).

## Terminal Parsing Rules
1. Keep ANSI handling in `src/session/pty_worker.rs` only.
2. Preserve support for:
   - Cursor movement
   - Erase sequences
   - SGR styles and colors
   - Alternate screen toggling
3. Update tests whenever parser behavior changes.
4. Keep scrollback semantics consistent between parser and renderer.

## UI Rendering Rules
1. `terminal_pane_view` draws from read-only grid snapshots.
2. Cache invalidation is generation-based; increment generation for visible state changes.
3. Scroll events should map to logical lines and clamp bounds.
4. Sidebar must reflect current session ordering from `SessionManager`.

## Security Rules
1. Ignore broken-pipe signal at process start.
2. Validate configured shell path through `/etc/shells` and executable bit.
3. Enforce PTY input size limit before channel send.
4. Do not execute arbitrary shell strings; pass resolved shell path to `CommandBuilder`.

## Testing Standards
1. `cargo test` must pass before finalizing docs/feature changes.
2. Add unit tests for:
   - Grid mutation and scrollback invariants
   - Input key mapping conversions
   - Row/window calculations in renderer
3. When fixing bugs, add regression tests in module-local `#[cfg(test)]` blocks.

## Documentation Sync Rules
1. When session lifecycle or terminal rendering changes, update:
   - `docs/system-architecture.md`
   - `docs/codebase-summary.md`
   - `docs/project-changelog.md`
2. When requirements/scope changes, update `docs/project-overview-pdr.md` and roadmap files.
3. Keep each docs file under 800 LOC.
