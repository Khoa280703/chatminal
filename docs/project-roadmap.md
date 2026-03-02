# Project Roadmap

Last updated: 2026-03-02  
Current phase: Release hardening for Tauri runtime  
Overall progress: 90%

## Milestones
| Milestone | Status | Notes |
| --- | --- | --- |
| M1 - Runtime migration to Tauri v2 host | Completed | `src-tauri/` is the active desktop host. |
| M2 - Frontend migration to Svelte 5 + xterm.js | Completed | `frontend/` is active UI runtime. |
| M3 - Session command/event bridge | Completed | invoke contracts and `pty/*` events stable. |
| M4 - Persistence v1 (workspace + history) | Completed | SQLite profiles/sessions/scrollback/app_state delivered. |
| M5 - Lazy reconnect and restore flows | Completed | reconnect centered on `activate_session`. |
| M6 - Profile lifecycle UX + backend contracts | Completed | list/create/switch/rename/delete profile shipped. |
| M7 - Docs realignment to active runtime | Completed | core docs and deployment/design docs synchronized. |
| M8 - CI, integration tests, release packaging hardening | In Progress | release readiness track. |
| M9 - Tray lifecycle + keep-alive on close | Completed | close-to-tray, start-in-tray preference, graceful quit hooks shipped. |

## Current Priorities
1. Add integration tests for profile-switch + reconnect + retention edge cases.
2. Add CI pipeline for `src-tauri` tests and frontend build on Linux/macOS.
3. Finalize release checklist and artifact verification under `src-tauri/target/release/bundle/`.
4. Define legacy deprecation policy and timeline for `src/` runtime code.

## Release Gate for v0.2.x
1. `npx --prefix frontend tauri build` passes in release environment.
2. Profile/session lifecycle contracts verified by automated tests.
3. Persistence restore/reconnect scenarios covered with regression tests.
4. Core docs + changelog remain aligned with shipped behavior.

## Legacy Track
- `src/` (Iced) remains read-only legacy reference unless a legacy maintenance task is explicitly requested.
- No new feature work should target legacy runtime by default.

## Related Docs
- [Development Roadmap](./development-roadmap.md)
- [Deployment Guide](./deployment-guide.md)
- [Project Changelog](./project-changelog.md)
