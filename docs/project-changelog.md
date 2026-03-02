# Project Changelog

All notable implementation and documentation changes are tracked here.

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
