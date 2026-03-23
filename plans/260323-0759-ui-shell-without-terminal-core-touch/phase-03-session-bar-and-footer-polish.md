# Phase 03 - Session Bar and Footer Polish

## Context Links
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`
- `apps/chatminal-desktop/src/termwindow/render/fancy_tab_bar.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`

## Overview
- Priority: P2 | Status: done | Effort: 1d
- Polish session bar + footer chrome: consistent spacing, clear hierarchy, no terminal content conflict

## No-Touch
- Tab/session switching semantics, runtime commands, telemetry source contracts
- `desktop_commands.rs` — command definitions, not visual shell; if hit-target mapping needed, do it in tabbar.rs render layer only

## Objective
- Session bar: proper spacing, truncation with ellipsis, clear active state, adequate close/new affordances
- Footer: label/value rhythm, density, stays within bounds during resize

## Files Likely Touched
- Modify: `tabbar.rs`, `termwindow/render/tab_bar.rs`, `termwindow/render/fancy_tab_bar.rs`
- Modify: `termwindow/render/chatminal_sidebar.rs` (footer section)
- Modify: `desktop_termwindow_mouseevent.rs`

## Implementation Steps
1. Audit session bar item types and slot priorities
2. Extract shared chrome tokens (padding, divider, hover/active bg, muted colors) for bar + footer
3. Fix session bar: truncation, hit area sizing (>= 24px), close/new button spacing
4. Fix footer: grouping, ellipsis rules, resize stability

## Success Criteria
- Tab titles truncate with ellipsis, no text overflow or visual bleed
- Footer stays within bounds during window resize (no overlap with content)
- Hover/click areas >= 24px hit target on session bar items
- Top/bottom session bar modes both render correctly
- `cargo check -p chatminal-desktop` passes
- No files changed in `crates/chatminal-terminal-core/**`

## Risk Assessment
- Visual refactor may break old config behaviors for session bar position
- Mitigation: separate visual-only changes from assignment routing; verify config matrix

## Dependencies
- Phase 01 geometry contract (footer/header bounds)
