# Development Roadmap

Last updated: 2026-03-01

## Phase Status
| Phase | Scope | Status |
| --- | --- | --- |
| Phase 1 | Project bootstrap and app shell | Completed |
| Phase 2 | PTY session manager and worker threading | Completed |
| Phase 3 | Iced UI layout and app message loop | Completed |
| Phase 4 | Terminal rendering and cursor style mapping | Completed |
| Phase 5 | Input mapping and resize integration | Completed |
| Phase 6 | Scrollback viewport and stable offset behavior | Completed |
| Phase 7 | Runtime hardening + regression tests | Completed |
| Phase 8 | Integration tests, CI, packaging | In progress |

## Recently Completed
1. Runtime parser/state path migrated to `wezterm-term` (+ `wezterm-surface`).
2. Snapshot extraction constrained to `scrollback window + visible window`.
3. `lines_added` derived from top stable row delta to preserve scroll offset.
4. Exited-event dispatch moved to spawned sender thread with `blocking_send` to avoid reader-thread blocking.
5. Keyboard mapper expanded (`Shift+Tab`, function keys `F1..F12`, Alt prefix, Ctrl symbols).
6. Grid cell payload moved from `char` to `String` for grapheme-friendly snapshots.
7. Test baseline increased to 23 passing tests.

## Active Backlog (Phase 8)
1. Add integration tests for full session lifecycle under sustained output.
2. Add load/stress scenarios for update queue pressure.
3. Add CI workflow for `cargo test` and release build.
4. Define packaging strategy for at least one target platform.

## Dependency Snapshot
- `iced 0.14.0`
- `portable-pty 0.9.0`
- `wezterm-term` (git rev `05343b3...`)
- `wezterm-surface` (git rev `05343b3...`)

## Documentation Sync Checklist
For each runtime change:
1. Update `docs/system-architecture.md`.
2. Update `docs/codebase-summary.md`.
3. Update `docs/project-overview-pdr.md` if requirement impact exists.
4. Update `docs/project-roadmap.md` and changelog entries.
5. Re-run docs validation script.
