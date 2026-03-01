# Project Changelog

All notable project-level changes are tracked here.

## 2026-03-01 (terminal fidelity hardening round 2)

### Changed
- Improved canvas text placement to remove manual baseline offset drift:
  - render text from cell top with consistent vertical centering.
  - adjusted terminal cell metrics (`CELL_WIDTH_RATIO`, `CELL_HEIGHT_RATIO`) for tighter cursor/text alignment.
- Expanded keyboard encoding coverage in input mapper:
  - added `Shift+Tab`, `Insert`, `F1..F12`.
  - improved `Ctrl` symbol combos (`Ctrl+@`, `Ctrl+[`, `Ctrl+\\`, `Ctrl+]`, `Ctrl+^`, `Ctrl+_`, `Ctrl+?`).
  - added `Alt` prefix handling for non-empty key sequences.
- Upgraded terminal cell payload from single `char` to `String`:
  - preserves grapheme clusters better (emoji/combining sequences) when snapshotting from wezterm.
- Reduced hot-path allocations for blank cells:
  - default/continuation cells now use empty string sentinel instead of allocating `" "`.
- Improved style fidelity:
  - underline is now rendered based on cell attributes even when glyph content is blank/continuation.
- Color fidelity now resolves through wezterm active palette for non-default colors.

### Added
- New input-handler tests for:
  - `Shift+Tab`
  - `Alt+Arrow`
  - `F-key` sequence mapping
- Test baseline increased to 23 passing tests.

## 2026-03-01 (wezterm cursor-map hardening)

### Changed
- Completed terminal snapshot path on top of `wezterm-term` state:
  - PTY bytes -> `wezterm-term` -> screen snapshot -> `TerminalGrid` cells.
- Replaced hardcoded cursor rendering mode with mapped cursor metadata from wezterm:
  - `CursorShape` + `CursorVisibility` -> `CursorStyle::{Block, Underline, Bar, Hidden}`.
- Added dependency pin for `wezterm-surface` with the same git `rev` as `wezterm-term`.
- Hardened shutdown event flow to avoid reader-thread deadlock on queue backpressure:
  - reader no longer blocks on same thread when emitting `SessionEvent::Exited`.
  - `Exited` delivery is delegated to a short-lived async sender thread.
- Optimized snapshot extraction to avoid re-reading full physical history on each flush:
  - only reads `scrollback window + visible window` from wezterm screen range.

### Added
- PTY worker regression tests for cursor behavior:
  - cursor shape sequences (`CSI Ps SP q`) map to underline/bar.
  - cursor visibility (`CSI ? 25 l/h`) maps to hidden/visible styles.
  - cursor row/col stays aligned with visible viewport after scrollback growth.
  - exited event is delivered when queue becomes available.
- Test baseline increased to 20 passing tests.

### Documented
- Updated `docs/system-architecture.md` to reflect wezterm-based parsing/snapshot flow and cursor metadata mapping.

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
