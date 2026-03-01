# Project Changelog

All notable implementation and documentation changes are tracked here.

## 2026-03-01 (docs synchronization refresh)

### Changed
- Added root `README.md` as repository entrypoint.
- Updated core docs to match current runtime implementation:
  - parser/runtime based on `wezterm-term` + `wezterm-surface`
  - snapshot extraction limited to `scrollback window + visible window`
  - `lines_added` derived from stable-row delta
  - EOF/read-error `SessionEvent::Exited` dispatch via spawned sender thread + `blocking_send`
- Aligned all references to current test baseline: `23 passed`.
- Updated roadmap state and cross-links between roadmap/changelog docs.

## 2026-03-01 (terminal fidelity hardening round 2)

### Changed
- Expanded keyboard mapping (`Shift+Tab`, function keys `F1..F12`, extended Ctrl symbols, Alt prefix).
- Moved grid cell payload from `char` to `String` for better grapheme handling.
- Improved underline rendering for empty/continuation cells.
- Updated color fidelity path to use resolved wezterm palette for non-default colors.

### Added
- Input-handler tests for key mapping expansions.
- Test baseline increased to 23 passing tests.

## 2026-03-01 (wezterm cursor-map hardening)

### Changed
- Completed PTY output path using wezterm terminal state snapshots.
- Added cursor style mapping from wezterm cursor shape/visibility.
- Optimized snapshot extraction to avoid scanning full physical history.
- Hardened exited-event delivery via dedicated sender thread to avoid reader-thread stalls.

### Added
- Regression tests for cursor shape/visibility mapping and exited-event delivery behavior.
- Test baseline increased to 20 passing tests.

## 2026-03-01 (runtime hardening baseline)

### Changed
- Config clamp hardening for `scrollback_lines`, `font_size`, `sidebar_width`.
- Runtime cell-metric path tied to `font_size` via `metrics_for_font`.
- Queue-full update behavior changed to retry latest dirty snapshot.
- Reverse-index parser behavior covered (`ESC M`).

### Added
- Regression tests for reverse-index and queue-full retry.
- Test baseline increased to 13 passing tests.

## 2026-03-01 (initial docs baseline)

### Added
- Established `docs/` structure and initial architecture/PDR/standards/roadmap docs.

### Notes
- This was the initial documentation bootstrap point for the repository.
