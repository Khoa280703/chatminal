# Code Standards

Last updated: 2026-03-03  
Scope: active runtime code in `src-tauri/` and `frontend/`.

## Runtime Scope Rule
- Treat `src-tauri/` + `frontend/` as the default runtime.
- Treat `src/` (Iced) as legacy; only touch it for explicit legacy tasks.

## Core Principles
1. Keep Rust/TypeScript contracts stable and explicitly versioned in docs.
2. Keep PTY paths bounded and fail-fast on invalid input/state.
3. Keep persistence off PTY hot paths (queue + batch writer only).
4. Keep UI session-safe during async operations (guard by active IDs).
5. Keep docs synchronized whenever command/event/runtime behavior changes.

## Architecture Boundaries
| Layer | Primary Files | Rules |
| --- | --- | --- |
| Tauri bridge | `src-tauri/src/main.rs` | Register/invoke commands only; no PTY business logic. |
| PTY service | `src-tauri/src/service.rs` | Own session/profile orchestration, runtime spawn, events, workers. |
| Persistence | `src-tauri/src/persistence.rs` | Own SQLite schema, state keys, retention, migrations. |
| Contracts | `src-tauri/src/models.rs`, `frontend/src/lib/types.ts` | Keep request/response/event fields aligned (snake_case). |
| UI runtime | `frontend/src/App.svelte` | Own xterm lifecycle, workspace hydration, activation and event handling. |

## Runtime Owner Rules
1. Runtime owner observability must be explicit (`requested_mode` vs `runtime_owner`).
2. Until daemon cutover is complete, runtime owner must fail-closed to `in_process`.
3. Daemon probe failures must never block app startup.

## API and Naming Rules
1. Rust naming: `snake_case` functions/fields, UpperCamelCase types.
2. Tauri payload field names stay `snake_case` for Rust model parity.
3. Command names are API contracts; avoid rename without migration notes.
4. Event names are API contracts; `pty/output`, `pty/exited`, `pty/error` must remain stable.
5. Profile command set must stay coherent: `list_profiles`, `create_profile`, `switch_profile`, `rename_profile`, `delete_profile`.

## Session and Reconnect Rules
1. `activate_session` is the reconnect boundary for disconnected sessions.
2. `write_input` and `resize_session` must reject disconnected sessions.
3. Session `cwd` updates come from CWD sync worker, not frontend assumptions.
4. New session default `cwd`: explicit payload -> home directory -> `/` fallback.
5. Session status contract values remain `running` or `disconnected`.

## Persistence Rules
1. Preserve state keys:
- `active_profile_id`
- `active_session_id:{profile_id}`
2. Keep legacy key migration support for `active_session_id` until explicit removal.
3. Append history using batch pipeline only; no direct DB writes in PTY read loop.
4. Enforce retention in batch writes (line cap + TTL).
5. Profile deletion must prevent deleting the last profile.

## Security and Reliability Rules
1. Validate shell path with `/etc/shells`, canonicalization, executable-bit checks.
2. Enforce the backend input-size guard constant before enqueueing writes.
3. Keep the input queue bounded and surface backpressure errors.
4. Cap snapshot size with the backend snapshot-size guard.
5. Emit deterministic `pty/exited` and `pty/error` signals for UI consistency.

## Frontend Rules
1. Guard async actions with current `activeSessionId` where race conditions are possible.
2. Hydrate with `get_session_snapshot` before connect/resize paths.
3. Use `ensureSessionConnected`/`activate_session` before sending input to disconnected sessions.
4. Keep xterm addon loading fault-tolerant (WebGL fallback to canvas renderer).
5. Re-run `resize_session` only for running sessions.
6. Treat command-level DB sync behavior as opt-in; default behavior must match normal terminal semantics.

## Documentation Sync Rules
When runtime contracts change, update:
1. `README.md`
2. `docs/system-architecture.md`
3. `docs/codebase-summary.md`
4. `docs/project-overview-pdr.md`
5. `docs/project-roadmap.md`
6. `docs/development-roadmap.md`
7. `docs/deployment-guide.md`
8. `docs/design-guidelines.md` (if UI behavior changed)
9. `docs/project-changelog.md`

## Validation Commands
```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm --prefix frontend run build
node $HOME/.claude/scripts/validate-docs.cjs docs/
```

## Compatibility Regression Commands (Linux)
```bash
vim /tmp/chatminal-vim.txt
btop
printf '%s\n' alpha beta gamma | fzf
seq 1 300 | less
nano /tmp/chatminal-unicode.txt
printf 'e\u0301 | 你 | 😀\n'
stty size
```
