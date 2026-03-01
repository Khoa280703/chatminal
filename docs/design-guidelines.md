# Design Guidelines

Last updated: 2026-03-01

## Design Intent
Chatminal is terminal-first: dense information, low chrome, keyboard-first flow, predictable viewport behavior.

## Layout Contract
- Two-pane layout:
  - Left: session sidebar.
  - Right: terminal canvas.
- Default sidebar width: `240.0`.
- Terminal background: black (`TERMINAL_BG`).

## Typography and Cell Metrics
- Font family: Iced monospace.
- Default font size: `14.0`.
- `metrics_for_font(14.0)` yields approximately:
  - cell width: `8.68`
  - cell height: `16.8`

## Session List Behavior
1. Active session indicator uses `●`; inactive uses `○`.
2. Session order follows insertion order from `SessionManager`.
3. Footer keeps primary shortcuts visible (Alt+N, Alt+W).

## Terminal Rendering Rules
1. Draw non-default background color per cell rectangle.
2. Draw cursor only when viewport is at live bottom (`scroll_offset == 0`).
3. Cursor styles must map exactly to runtime state:
   - `CursorStyle::Block`
   - `CursorStyle::Underline`
   - `CursorStyle::Bar`
   - `CursorStyle::Hidden`
4. Underline is attribute-driven and should render even for empty/continuation cells.
5. Draw loop must stay clipped to canvas bounds.

## Color Behavior
- Default fg: white.
- Default bg: black.
- Non-default colors are produced from wezterm palette resolution in runtime snapshot stage.
- UI maps `CellColor::Rgb` directly to Iced `Color::from_rgb8`.

## Input and Interaction
1. App-level shortcuts handled in app state layer.
2. Terminal input mapper handles terminal sequences (Shift+Tab, Insert, F1..F12, Alt prefix, control symbols).
3. Scroll behavior is line-based and deterministic for both line and pixel wheel deltas.

## Usability Gaps
1. No in-app settings panel yet.
2. No keybinding customization UI.
3. No explicit accessibility/screen-reader mode.
4. No visible scrollback position indicator.

## Next UX Priorities
1. Add settings panel (font/theme/sidebar width).
2. Add scrollback position indicator.
3. Add session rename flow.
4. Add command palette for keyboard actions.
