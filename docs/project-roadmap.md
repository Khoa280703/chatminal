# Project Roadmap

Last updated: 2026-03-01
Current phase: Phase 7 - Hardening and Release Prep
Overall progress: 78%

## Milestones
| Milestone | Status | Evidence |
| --- | --- | --- |
| M1 - App bootstrap + initial session flow | Completed | `main.rs`, `app.rs` boot and initial session create path. |
| M2 - PTY session management | Completed | `session/manager.rs` create/close/resize/send input. |
| M3 - Terminal runtime migration to wezterm | Completed | `wezterm-term` + `wezterm-surface` in `Cargo.toml`; runtime in `pty_worker.rs`. |
| M4 - Rendering and scrollback viewport | Completed | `ui/terminal_pane.rs` + scroll offset handling in `app.rs`. |
| M5 - Keyboard mapping expansion | Completed | `ui/input_handler.rs` covers Shift+Tab, Insert, F1..F12, Alt/Ctrl combos. |
| M6 - Unit test hardening baseline | Completed | `cargo test`: 23 passed, 0 failed (2026-03-01). |
| M7 - Docs consistency refresh | Completed | README + core docs synced to wezterm runtime and latest test baseline. |
| M8 - Integration, CI, packaging | In Progress | Missing integration tests, CI workflow, and release packaging scripts. |

## Next Priority Tracks
1. Reliability
- Add integration tests for session lifecycle and resize behavior.
- Add sustained-output test scenario for event queue pressure.

2. Product UX
- Session rename.
- In-app settings surface for font/theme/sidebar width.
- Visible scrollback position indicator.

3. Release Engineering
- Add CI pipeline for build + test.
- Add at least one packaging/distribution target.
- Publish versioned release checklist.

## Exit Criteria for v0.2.0
1. Integration tests cover create/switch/close/resize/exited flows.
2. CI pipeline runs `cargo test` and release build.
3. One packaging path is scripted and documented.
4. Architecture + changelog + roadmap are updated for release cut.

## Related Docs
- [Development Roadmap](./development-roadmap.md)
- [Project Changelog](./project-changelog.md)
