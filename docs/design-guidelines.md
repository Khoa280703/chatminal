# Design Guidelines

Last updated: 2026-03-01

## Design Intent
Chatminal prioritizes a terminal-first workflow: content density, fast switching, low visual noise, keyboard-first controls.

## Current Layout Contract
- Two-pane layout:
  - Left sidebar for session list/actions.
  - Right terminal canvas for active session output.
- Sidebar width default: `240.0`.
- Terminal background: black.

## Typography and Cell Metrics
- Default font: Iced monospace.
- Default font size: `14.0`.
- Default cell metrics:
  - width `8.4`
  - height `18.0`

## Session List Behavior
1. Active session is indicated with `●`; inactive uses `○`.
2. Session labels follow index order from `SessionManager`.
3. Footer exposes quick actions and shortcuts.

## Terminal Rendering Rules
1. Draw non-default background colors per cell block.
2. Draw cursor highlight only when viewport is at live bottom (`scroll_offset == 0`).
3. Support text attributes:
   - bold
   - italic (state retained, currently only bold has explicit font weight change)
   - underline
4. Keep draw loop clipped to current canvas bounds.

## Color Behavior
- Default foreground: white.
- Default background: black.
- Indexed colors follow terminal base16 + 256-color cube + grayscale mapping.
- Truecolor mapping is direct to Iced `Color::from_rgb8`.

## Input/Interaction Guidelines
1. Keyboard mapping should preserve expected terminal semantics.
2. Alt combinations should remain reserved for app-level session controls.
3. Scroll behavior should be line-based and deterministic across wheel modes (line/pixel).

## Accessibility and Usability Gaps
1. No configurable color theme presets yet.
2. No configurable keybindings yet.
3. No explicit screen-reader support path.
4. No zoom-in/zoom-out shortcuts; font size is config-driven only.

## Future UX Priorities
1. Add in-app settings panel for font/spacing/theme.
2. Add visible scrollback position indicator.
3. Add session rename command.
4. Add command palette for keyboard-driven actions.
