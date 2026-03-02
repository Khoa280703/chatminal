# System Architecture

Last updated: 2026-03-02

## Runtime Topology
Chatminal runs as a desktop Tauri app with a Rust PTY backend and Svelte frontend.

```text
+-------------------------------+      invoke()       +-------------------------------+
| Frontend (Svelte + xterm.js)  |-------------------->| Tauri command layer (main.rs) |
| frontend/src/App.svelte       |<--------------------| load/profile/session commands  |
| - workspace/profile/session UX|      events         +---------------+---------------+
| - xterm IO + reconnect logic  |  pty/output|exited|error            |
+---------------+---------------+                                      |
                |                                                      v
                |                                   +------------------+------------------+
                |                                   | PtyService (service.rs)              |
                |                                   | - profile/session maps               |
                |                                   | - runtime spawn/activate/close       |
                |                                   | - shell validation + bounds          |
                |                                   | - history/cleanup/cwd workers        |
                |                                   +------------------+------------------+
                |                                                      |
                |                                                      v
                |                                   +------------------+------------------+
                +---------------------------------->| Persistence (persistence.rs)         |
                                                    | SQLite: profiles/sessions/scrollback |
                                                    | app_state keys + retention/migration |
                                                    +--------------------------------------+
```

## Active vs Legacy Runtime
- Active runtime: `src-tauri/` + `frontend/`.
- Legacy runtime: `src/` + root `Cargo.toml` (Iced). Keep as legacy reference only.

## Component Responsibilities
| Component | Files | Responsibilities |
| --- | --- | --- |
| App bootstrap and command exposure | `src-tauri/src/main.rs` | Register Tauri commands and app state. |
| PTY orchestration | `src-tauri/src/service.rs` | Session lifecycle, profile operations, event emit, worker startup. |
| Data contracts | `src-tauri/src/models.rs` | Request/response/event payload models. |
| Persistence | `src-tauri/src/persistence.rs` | Schema, migrations, state keys, history retention. |
| Runtime config | `src-tauri/src/config.rs` | `settings.json` normalization + legacy shell fallback. |
| Frontend shell | `frontend/src/App.svelte` | Workspace hydration, terminal rendering, profile/session actions. |

## Command Contracts
### Workspace and Profile
- `load_workspace`
- `list_profiles`
- `create_profile`
- `switch_profile`
- `rename_profile`
- `delete_profile`

### Session and Terminal
- `list_sessions`
- `create_session`
- `activate_session`
- `write_input`
- `resize_session`
- `rename_session`
- `set_session_persist`
- `get_lifecycle_preferences`
- `set_lifecycle_preferences`
- `shutdown_app`
- `close_session`
- `clear_session_history`
- `clear_all_history`
- `get_session_snapshot`

## Event Contracts
- `pty/output` -> `{ session_id, chunk, seq, ts }`
- `pty/exited` -> `{ session_id, exit_code, reason }`
- `pty/error` -> `{ session_id, message }`
- `app/tray-new-session` -> tray yêu cầu frontend tạo session mới
- `app/lifecycle-hidden` -> main window vừa được hide về tray

## Lifecycle and Data Flow
1. App boot initializes the PTY service, workers, and persistence restore.
2. Frontend calls `load_workspace` and applies profile/session state.
3. Frontend hydrates current terminal using `get_session_snapshot`.
4. Disconnected sessions remain preview-only until activation.
5. Frontend calls `activate_session` to reconnect/spawn runtime for disconnected sessions.
6. Frontend sends keyboard input via `write_input`.
7. Reader thread emits `pty/output`; frontend applies ordered chunks by `seq`.
8. On reader EOF/error, cleanup worker emits `pty/exited`, closes runtime, and sets status to disconnected.
9. Window close event có thể được intercept để hide-to-tray thay vì thoát process, dựa trên lifecycle preferences.

## Persistence Design
SQLite tables:
- `profiles`
- `sessions`
- `scrollback`
- `app_state`

State keys:
- `active_profile_id`
- `active_session_id:{profile_id}`
- `keep_alive_on_close`
- `start_in_tray`
- legacy key migration: `active_session_id`

History retention:
- line cap: `max_lines_per_session`
- TTL: `auto_delete_after_days`

## Background Workers
- `chatminal-cleanup`: finalizes exited sessions and disconnect state.
- `chatminal-history-writer`: buffers and batches history writes (`50ms` interval, batch `128`).
- `chatminal-cwd-sync`: polls process cwd every `500ms`, updates in-memory and DB state.

## Runtime Controls and Limits
- `MAX_INPUT_BYTES = 65_536`
- `INPUT_QUEUE_SIZE = 128`
- `MAX_SNAPSHOT_BYTES = 512 * 1024`
- `HISTORY_FLUSH_INTERVAL = 50ms`
- `HISTORY_BATCH_SIZE = 128`
- `CWD_SYNC_INTERVAL = 500ms`

## Security and Validation Controls
- Shell path allow-list from `/etc/shells`.
- Canonical path + executable-bit checks before spawn.
- Input-size guard and bounded queues for write path.
- Fallback shell order: configured shell -> `$SHELL` -> `/bin/zsh` -> `/bin/bash` -> `/bin/sh`.
