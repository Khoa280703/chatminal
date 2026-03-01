# Project Changelog

All notable project-level changes are tracked here.

## 2026-03-01 (Latest docs sync for recent runtime hardening)

### Changed
- Updated docs to reflect config clamp behavior in `src/config.rs`:
  - `scrollback_lines` -> `100..=200_000`
  - `font_size` -> `8.0..=48.0`
  - `sidebar_width` -> `160.0..=640.0`
- Updated runtime architecture docs for terminal metrics derived from `font_size` via `metrics_for_font`.
- Updated PTY flow docs for queue-full behavior:
  - `SessionEvent::Update` retries latest dirty snapshot when queue is full.
  - `SessionEvent::Exited` uses blocking send on EOF/read error.
- Updated parser docs for reverse index (`ESC M`) handling semantics.
- Updated unit-test baseline from 7 to 13 passing tests.

### Added
- Documented parser-level regression tests in `src/session/pty_worker.rs`:
  - `reverse_index_esc_m_scrolls_down_from_top_row`
  - `flush_update_retries_after_queue_full`
- Added docs impact report for this update in `plans/reports/`.

### Documented
- Runtime/backpressure semantics for PTY -> UI event propagation.
- Runtime cell-metric calculation path and resize implications.
- Expanded test coverage map across config, session grid, PTY worker, and UI modules.

## 2026-03-01

### Changed
- Finalized implementation-plan sync for `plans/260301-1521-chatminal-messenger-terminal`.
- Updated plan status from `pending` to `in-progress`.
- Updated phase checklists with current implementation evidence (`cargo check/test/build/clippy` run).
- Marked roadmap phases 1-7 as `In progress` until remaining checklist items close.

### Added
- Established full `docs/` baseline:
  - `index.md`
  - `project-overview-pdr.md`
  - `system-architecture.md`
  - `code-standards.md`
  - `codebase-summary.md`
  - `deployment-guide.md`
  - `design-guidelines.md`
  - `project-roadmap.md`
  - `development-roadmap.md`
  - `project-changelog.md`

### Documented
- Current Rust architecture: app state machine, session manager, PTY workers, terminal grid, canvas renderer.
- Security controls already implemented: SIGPIPE ignore, shell allowlist validation, bounded input size.
- Current test baseline: 7 passing unit tests.

### Notes
- `docs/` directory did not exist before this date.
- `codebase-summary.md` is derived from `repomix-output.xml` generated on 2026-03-01.
