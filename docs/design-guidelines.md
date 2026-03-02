# Design Guidelines

Last updated: 2026-03-02

## Design Intent
Chatminal UI is terminal-first:
- fast session switching
- low-friction keyboard usage
- high-contrast terminal readability
- clear profile/session state visibility

## Visual Language (Current Frontend)
Source: `frontend/src/styles.css`

- Primary UI font: `Space Grotesk`
- Monospace terminal/support text: `Space Mono` plus xterm fallback stack
- Layout: two-column grid (`320px` sidebar + terminal pane)
- Sidebar: dark layered gradient with session cards and profile footer
- Terminal pane: dark canvas with xterm rendering

## Color Tokens
`frontend/src/styles.css` defines root tokens:
- `--bg-deep`, `--bg-surface`, `--bg-surface-soft`, `--bg-terminal`
- `--text-main`, `--text-dim`
- `--line`, `--primary`, `--danger`

Session tone badges use semantic classes:
- `tone-indigo`
- `tone-emerald`
- `tone-rose`
- `tone-amber`
- `tone-slate`

## Layout and Interaction Contracts
1. Sidebar contains:
- brand header
- new-session CTA
- session search
- filtered session list
- profile menu/footer actions
2. Terminal pane contains:
- active session meta bar
- session-scoped actions (rename/persist/clear history)
- xterm host area
3. Session cards show:
- glyph/avatar
- name and cwd
- status (`running` or `disconnected`)
- inline controls

## Session UX Rules
1. Search filters by session name + cwd.
2. Session status labels must reflect backend status values (`running`, `disconnected`).
3. Active session hydration uses snapshot before live reconnect.
4. Disconnected sessions should be visually distinct and reconnect on activation.

## Input and Shortcut Rules
1. App-level shortcuts:
- `Alt+N`: create new session
- `Alt+W`: close active session
2. Other terminal input should flow to PTY via `write_input`.
3. Reconnect should happen before input if session is disconnected.
4. Rename/profile inputs must support Enter confirm and Escape cancel.

## Terminal Behavior Rules
1. Keep xterm `scrollback` aligned with app preview strategy.
2. Preserve output ordering using per-session sequence checks.
3. If WebGL addon load fails, continue with renderer fallback.
4. Keep terminal resize responsive with the fit addon plus `resize_session` for running sessions.

## Accessibility and Usability Backlog
1. Add keyboard-first focus ring improvements for session controls.
2. Add explicit high-contrast theme toggle in settings.
3. Add explicit reconnect affordance text for disconnected sessions.
4. Add screen-reader semantics audit for profile/session menus.
