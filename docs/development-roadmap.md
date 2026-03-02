# Development Roadmap

Last updated: 2026-03-02

## Phase Status
| Phase | Scope | Status |
| --- | --- | --- |
| Phase 1 | Tauri host + frontend migration foundation | Completed |
| Phase 2 | PTY session orchestration and command bridge | Completed |
| Phase 3 | Workspace persistence (sessions + history + state) | Completed |
| Phase 4 | Lazy reconnect and snapshot hydration | Completed |
| Phase 5 | Profile lifecycle management | Completed |
| Phase 6 | Runtime hardening and docs realignment | Completed |
| Phase 7 | CI + integration tests + packaging hardening | In Progress |

## Recently Completed
1. Workspace/profile contracts stabilized (`load_workspace`, profile CRUD/switch).
2. Persistence keys normalized to profile-scoped active session key pattern.
3. Reconnect workflow centered on `activate_session` for disconnected sessions.
4. CWD sync worker integrated for runtime path continuity after `cd`.
5. Core docs were rewritten to remove active-runtime ambiguity.

## Active Backlog
1. Integration tests for profile switch + reconnect + retention edge cases.
2. CI matrix for Linux/macOS build + test + bundle verification.
3. Release checklist and artifact validation by target format.
4. Legacy runtime deprecation strategy (`src/`) and cleanup criteria.

## Dependency Snapshot (Active Runtime)
- `tauri 2.0.0`
- `tauri-plugin-store 2.4.1`
- `portable-pty 0.9.0`
- `rusqlite 0.32.1` (bundled SQLite)
- `svelte ^5.0.0`
- `xterm ^5.3.0`

## Documentation Sync Checklist
For runtime contract changes:
1. Update `README.md`.
2. Update `docs/system-architecture.md`.
3. Update `docs/codebase-summary.md`.
4. Update `docs/project-overview-pdr.md` if requirements changed.
5. Update `docs/project-roadmap.md` and `docs/development-roadmap.md`.
6. Update `docs/deployment-guide.md` and `docs/design-guidelines.md` if behavior/UI changed.
7. Append entry in `docs/project-changelog.md`.
8. Run docs validation script.
