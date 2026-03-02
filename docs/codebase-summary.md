# Codebase Summary

Last updated: 2026-03-02

## Source of Truth
This summary is based on:
1. `repomix-output.xml` regenerated with:
```bash
repomix -o repomix-output.xml --style xml
```
2. Direct verification in active runtime files under `src-tauri/src/` and `frontend/src/`.

Repomix snapshot (2026-03-02):
- packed files: `83`
- total tokens: `184,938`
- output: `repomix-output.xml`

## Runtime Status
- Active runtime: `Tauri v2 + Rust + Svelte 5 + xterm.js`.
- Active execution path: `src-tauri/` + `frontend/`.
- Legacy path: root `Cargo.toml` + `src/` (Iced runtime), retained for legacy maintenance only.

## Repository Map (High Signal)
| Path | Role |
| --- | --- |
| `src-tauri/src/main.rs` | Tauri command registration and app initialization |
| `src-tauri/src/service.rs` | PTY lifecycle, runtime workers, shell/cwd handling, event emission |
| `src-tauri/src/persistence.rs` | SQLite schema/migrations, workspace restore, history retention |
| `src-tauri/src/models.rs` | Rust request/response/event payload contracts |
| `src-tauri/src/config.rs` | `settings.json` load/normalize + legacy `config.toml` fallback |
| `frontend/src/App.svelte` | Profile/session UX, xterm setup, invoke/listen bridge |
| `frontend/src/lib/types.ts` | TypeScript contract mirror |
| `src/` | legacy Iced runtime |
| `docs/` | technical/product documentation |

## Filtered LOC Snapshot
(Workspace scan excluding `.git`, `node_modules`, `target`, test/cache artifacts)
- `src-tauri`: 66 files, 17,327 LOC
- `frontend`: 9 files, 3,339 LOC
- `src` (legacy): 15 files, 2,028 LOC
- `plans`: 33 files, 4,240 LOC
- `docs`: 10 files, 583 LOC

High-LOC code directories:
- `src-tauri/src`: 2,934 LOC
- `frontend/src`: 1,653 LOC
- `src/session` (legacy): 1,053 LOC
- `src/ui` (legacy): 461 LOC

## Verified Runtime Command Contracts
### Workspace/Profile
- `load_workspace`
- `list_profiles`
- `create_profile`
- `switch_profile`
- `rename_profile`
- `delete_profile`

### Session/Terminal
- `list_sessions`
- `create_session`
- `activate_session`
- `write_input`
- `resize_session`
- `rename_session`
- `set_session_persist`
- `close_session`
- `clear_session_history`
- `clear_all_history`
- `get_session_snapshot`

### Runtime Events
- `pty/output`
- `pty/exited`
- `pty/error`

## Persistence Model (Verified)
SQLite tables:
- `profiles`
- `sessions`
- `scrollback`
- `app_state`

State keys:
- `active_profile_id`
- `active_session_id:{profile_id}`
- legacy migration key: `active_session_id`

Retention behavior:
- line cap trim (`max_lines_per_session`)
- TTL trim (`auto_delete_after_days`)

## Runtime Limits and Workers
Limits in `src-tauri/src/service.rs`:
- `MAX_INPUT_BYTES = 65_536`
- `INPUT_QUEUE_SIZE = 128`
- `MAX_SNAPSHOT_BYTES = 512 * 1024`
- `HISTORY_FLUSH_INTERVAL = 50ms`
- `HISTORY_BATCH_SIZE = 128`
- `CWD_SYNC_INTERVAL = 500ms`

Background workers:
- cleanup worker: marks exited sessions disconnected and finalizes runtime state
- history writer worker: batches scrollback writes and retention
- CWD sync worker: tracks and persists live process working directory

## Runtime Behavior Notes
- New session `cwd` defaults to user home (`~`) when available, then `/` fallback.
- Lazy reconnect is performed through `activate_session`; UI calls it on activation and before input when session is disconnected.
- Workspace restore hydrates sessions as disconnected previews first; runtime is spawned only when activated.

## Current Dev/Build Commands
```bash
npm --prefix frontend install
npx --prefix frontend tauri dev
npm --prefix frontend run build
npx --prefix frontend tauri build
```
