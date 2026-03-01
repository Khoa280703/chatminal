# Project Roadmap

Last updated: 2026-03-01
Current phase: MVP Completed
Overall progress: 72%

## Milestones
| Milestone | Status | Notes |
| --- | --- | --- |
| M1 - Core app bootstrap and session creation | Completed | `main.rs`, `app.rs`, `session/manager.rs` in place. |
| M2 - PTY integration and ANSI parsing | Completed | `session/pty_worker.rs` + `session/grid.rs` implemented. |
| M3 - Terminal rendering and scrollback | Completed | `ui/terminal_pane.rs` and scroll controls implemented. |
| M4 - Keyboard/session UX controls | Completed | `ui/input_handler.rs`, sidebar actions, shortcuts implemented. |
| M5 - Baseline unit tests | Completed | 13 tests passing in `cargo test` (includes ESC M + queue-full retry regression tests). |
| M6 - Documentation baseline | Completed (this update) | `docs/` standards and architecture/PDR created. |
| M7 - Hardening and release prep | In Progress | Config clamp, runtime metrics, ESC M fix, queue-full handling documented/done; integration/load/packaging pending. |

## Next 3 Priority Tracks
1. Reliability hardening
- Add integration tests for session lifecycle and high-throughput output.
- Add benchmark for canvas redraw pressure.

2. Product polish
- Session rename.
- In-app settings for font/theme/sidebar width.
- Better state indicators (session exit, scrollback position).

3. Distribution readiness
- Add packaging strategy and release artifacts.
- Add CI workflow for build + test on target platforms.

## Exit Criteria for v0.2.0
1. Integration tests added for PTY lifecycle and resize behavior.
2. At least one packaging target scripted.
3. User-facing settings path available (config UI or robust config docs + validation warnings).
4. Changelog and architecture docs updated for release.
