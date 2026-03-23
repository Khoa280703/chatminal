## Context Links
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`

## Overview
- Priority: P1
- Current status: pending
- Brief description: make session bar and footer feel like one product shell, with better density, hierarchy, truncation, and status readability.
- Explicit boundary: không đụng `crates/chatminal-terminal-core/**`; không đổi terminal progress/state generation, only presentation.

## Objective
- Rebalance top/bottom chrome so session switching and ambient status are useful but not noisy.
- Align footer with sidebar/session bar spacing and typography.

## Scope
- Session title formatting, active/hover states, button grouping, truncation rules, footer metric layout, separator rules.
- Keep all data sources unchanged.

## Key Insights
- `tabbar.rs` already owns progress/icon/title rendering.
- Footer is currently built in sidebar render file and can share shell tokens without runtime changes.
- Progress icons already exist via `pct_to_glyph`; presentation can improve without touching data semantics.

## Requirements
- Functional: active session stands out; inactive sessions remain scannable; footer metrics stay legible on narrow widths.
- Non-functional: no overflow into title buttons or sidebar; no extra runtime fetches.

## Architecture
- Session bar stays in `tabbar.rs`.
- Footer stays shell-owned in `termwindow/render/chatminal_sidebar.rs`.
- Shared shell tokens from Phase 02 drive both.

## Files Likely Touched
- Modify:
  - `apps/chatminal-desktop/src/tabbar.rs`
  - `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
  - `apps/chatminal-desktop/src/termwindow/mod.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- Create: none preferred
- Delete: none

## Implementation Steps
1. Rework session bar density, padding, and truncation rules.
2. Refine progress/icon placement and inactive contrast.
3. Recompose footer items for narrow, medium, and wide window widths.
4. Align footer spacing with sidebar width and content bounds.

## Todo List
- [ ] Session titles truncate predictably
- [ ] Active/inactive contrast tuned
- [ ] Footer metrics collapse gracefully on small widths
- [ ] Top/bottom chrome feels visually related

## Success Criteria
- Session bar remains readable with many sessions.
- Footer does not crowd terminal area.
- No overlap with integrated title buttons or sidebar.

## Risk Assessment
- Risk: too much chrome reduces terminal usable area.
- Mitigation: define max chrome budget and validate minimum content viewport.

## Security Considerations
- Presentation-only changes.
- No new command execution or runtime state writes.

## Next Steps
- Bring same hierarchy rules into overlays.
- Unresolved questions: none.
