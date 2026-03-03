# Project Changelog

All notable implementation and documentation changes are tracked here.

## 2026-03-03 (settings page for lifecycle preferences)

### Added
- New Settings pane in main content area (`terminal | explorer | settings`).
- Dedicated lifecycle section in Settings for:
  - `Keep running in tray when close`
  - `Start in tray`

### Changed
- Removed lifecycle preference controls from profile popup menu.
- Added mobile override when sidebar is collapsed to keep terminal shell at full height.

## 2026-03-03 (realtime explorer tracking + native preview scroll)

### Added
- Backend realtime explorer watcher worker (`notify`) for active session root.
- New frontend event contract:
  - `explorer/fs-changed` (`session_id`, `root_path`, `changed_paths`, `full_resync`, `revision`)
- Debounced fs-change batching in backend to reduce UI refresh bursts.

### Changed
- Explorer frontend now auto-refreshes visible tree/open preview when matching fs-change events arrive.
- File preview renderer switched to native `readonly textarea` for stable default touchpad/mouse scroll behavior on Linux WebKitGTK.
- Explorer header now supports toggling right file tree panel (`Hide tree` / `Show tree`).

## 2026-03-03 (session-scoped file explorer v1)

### Added
- File explorer backend contracts:
  - `get_session_explorer_state`
  - `set_session_explorer_root`
  - `update_session_explorer_state`
  - `list_session_explorer_entries`
  - `read_session_explorer_file`
- SQLite table `session_explorer_state` with per-session persisted fields:
  - `root_path`
  - `current_dir`
  - `selected_path`
  - `open_file_path`
- Frontend explorer panel with mandatory root-picker flow per session.

### Changed
- Explorer behavior now strictly session-scoped (not profile-scoped).
- Explorer root is user-driven and no longer tied to terminal `cwd`.
- Added native folder-picker integration via dialog plugin for root selection.

## 2026-03-03 (live-path fidelity + owner observability)

### Added
- Runtime UI setting contract:
  - `get_runtime_ui_settings`
  - `sync_clear_command_to_history` (default off)
- Runtime backend observability fields:
  - `requested_mode`
  - `runtime_owner`
- Live replay buffer path in frontend to reduce snapshot dependence for running sessions.
- Linux compatibility checklist for `vim/btop/fzf/less/nano/unicode/resize`.

### Changed
- Frontend hydrate flow now prioritizes live replay/cache for running sessions before snapshot fallback.
- `clear` command interception path is now opt-in via runtime UI settings.
- Daemon staging now uses local IPC endpoint naming with fail-closed in-process owner behavior.

## 2026-03-02 (native daemon staging baseline)

### Added
- Runtime backend staging modules:
  - `src-tauri/src/runtime_backend.rs`
  - `src-tauri/src/chatminald_client.rs`
- New Tauri commands for runtime backend introspection:
  - `get_runtime_backend_info`
  - `ping_runtime_backend`
- New runtime backend model set for mode/info/ping contracts.

### Changed
- `AppState` now includes runtime backend mode resolver alongside PTY service.
- Windows shell-path validation now resolves executable candidates instead of accepting non-empty strings.
- Daemon ping transport switched to local IPC only:
  - Unix Domain Socket on Linux/macOS
  - Named Pipe on Windows
  - TCP removed from production path
- README and architecture docs now describe daemon staging env vars and command contracts.

## 2026-03-02 (tray lifecycle + app keep-alive)

### Added
- Tray integration in Tauri runtime:
  - menu actions `Show Chatminal`, `New Session`, `Quit Completely`
  - frontend tray event bridge via `app/tray-new-session`
- New lifecycle commands:
  - `get_lifecycle_preferences`
  - `set_lifecycle_preferences`
  - `shutdown_app`
- Lifecycle preference models and persistence keys:
  - `keep_alive_on_close`
  - `start_in_tray`

### Changed
- Main window close flow now supports hide-to-tray instead of hard exit when enabled.
- Backend gained `shutdown_graceful` session teardown path for controlled app quit.
- Shell resolution logic now has explicit Windows candidate branch for common shell executables.
- Frontend profile menu now exposes lifecycle toggles (keep-alive on close, start in tray).

## 2026-03-02 (docs realignment from merged scouting + doc-read reports)

### Changed
- Re-synced docs to active runtime: `Tauri v2 + Rust + Svelte 5 + xterm.js`.
- Added complete profile command coverage in architecture/summary/PDR docs:
  - `list_profiles`, `create_profile`, `switch_profile`, `rename_profile`, `delete_profile`.
- Corrected default session `cwd` docs to match runtime behavior:
  - payload `cwd` -> home directory (`~`) -> `/` fallback.
- Added persistence state-key documentation:
  - `active_profile_id`
  - `active_session_id:{profile_id}`
- Expanded deployment docs with release artifact locations and `settings.json` normalization ranges.
- Updated design/development roadmap docs to remove legacy-Iced-as-active ambiguity.
- Regenerated `repomix-output.xml` and rebuilt `docs/codebase-summary.md` from the new compaction.

### Docs Updated
- `README.md`
- `docs/index.md`
- `docs/project-overview-pdr.md`
- `docs/codebase-summary.md`
- `docs/code-standards.md`
- `docs/system-architecture.md`
- `docs/project-roadmap.md`
- `docs/deployment-guide.md`
- `docs/design-guidelines.md`
- `docs/development-roadmap.md`
- `docs/project-changelog.md`

## 2026-03-02 (persistence v1 + lazy reconnect implementation)

### Added
- SQLite persistence layer at `src-tauri/src/persistence.rs`:
  - tables `sessions`, `scrollback`, `app_state`
  - batch writer path for scrollback chunks
  - retention controls: line-cap + TTL
- New workspace/runtime commands:
  - `load_workspace`
  - `activate_session`
  - `set_session_persist`
  - `clear_session_history`
  - `clear_all_history`
- Session model extensions:
  - session status contract now uses `running | disconnected`
  - metadata fields `cwd`, `persist_history`

### Changed
- Frontend startup flow now restores workspace via `load_workspace`.
- Lazy reconnect restores disconnected previews and respawns on activation/input.
- Added session-level actions in UI: rename, toggle persistence, clear history.

## 2026-03-02 (tauri + svelte runtime docs baseline)

### Changed
- Established core docs baseline for active Tauri runtime.
- Added legacy note clarifying `src/` as non-default runtime path.

## 2026-03-01 (historical baseline)

### Notes
- Earlier docs/workplans were centered on the Rust/Iced runtime before Tauri became the active runtime.
