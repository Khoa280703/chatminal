# Development Roadmap

Last updated: 2026-03-01

## Phase Status
| Phase | Scope | Status |
| --- | --- | --- |
| Phase 1 | Project setup and app shell | Completed |
| Phase 2 | PTY session manager and worker threads | Completed |
| Phase 3 | Iced UI base layout | Completed |
| Phase 4 | Terminal canvas rendering | Completed |
| Phase 5 | Input handling + resize integration | Completed |
| Phase 6 | Virtual scrolling and scrollback behavior | Completed |
| Phase 7 | Integration polish and baseline tests | In progress (latest hardening patch applied) |
| Phase 8 | Hardening, CI, packaging | Planned |

## Recently Completed (Latest Edit Batch)
1. Config clamp hardening for `scrollback_lines`, `font_size`, and `sidebar_width`.
2. Runtime terminal cell metrics derived from `font_size` and used in resize calculations.
3. Parser handling fix for reverse index (`ESC M`).
4. PTY queue-full behavior improvement for update snapshot retry.
5. Regression test additions for `ESC M` and queue-full retry; total unit tests now 13.

## Upcoming Engineering Backlog
1. Add integration tests under `tests/` for end-to-end session open/close behavior.
2. Add load test script for high-output PTY scenarios.
3. Add CI pipeline (`cargo test`, `cargo build --release`) for Linux target.
4. Add platform compatibility checks for shell path strategy.
5. Add optional telemetry hooks for redraw and update frequency.
6. Evaluate final-snapshot flush behavior when EOF happens immediately after queue-full update.

## Dependency Notes
- Runtime stack currently stable with:
  - `iced 0.14.0`
  - `portable-pty 0.9.0`
  - `vte 0.15.0`
- Before dependency upgrades, run parser + rendering regression checks.

## Documentation Synchronization Checklist
For each feature or fix:
1. Update `docs/project-changelog.md`.
2. Update architecture if runtime flow changes.
3. Update PDR if scope or acceptance criteria changes.
4. Re-run docs validator.
