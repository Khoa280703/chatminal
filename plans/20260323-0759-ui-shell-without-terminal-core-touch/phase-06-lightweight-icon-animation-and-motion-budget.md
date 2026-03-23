## Context Links
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/paint.rs`

## Overview
- Priority: P2
- Current status: pending
- Brief description: add subtle motion and icon-state polish only where shell feedback benefits from it, using existing animation scheduler.
- Explicit boundary: không đụng `crates/chatminal-terminal-core/**`; không introduce new terminal render loop, parser hooks, or per-cell animation semantics.

## Objective
- Give shell chrome a small amount of life without tax on terminal performance.
- Reuse existing invalidation/animation plumbing.

## Scope
- Icon emphasis, progress-state transitions, hover fades, maybe sidebar/session selection accents.
- Strictly no terminal glyph animation or scrollback animation.

## Key Insights
- Desktop render path already tracks `has_animation`; shell motion can piggyback on that.
- `tabbar.rs` already maps progress to glyphs, so shell polish can stay presentation-only.
- Performance risk rises fast if motion enters line rendering or pane content.

## Requirements
- Functional: motion must communicate state change, not decorate everything.
- Non-functional: small frame budget, no continuous idle animation, respect reduced-motion config if available or add shell-local guard.

## Architecture
- Schedule shell animations through existing next-frame timing.
- Keep motion attached to chrome elements only.
- Prefer state interpolation in shell render modules, not terminal pane content code.

## Files Likely Touched
- Modify:
  - `apps/chatminal-desktop/src/tabbar.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
  - `apps/chatminal-desktop/src/termwindow/render/paint.rs`
  - `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Create: none preferred
- Delete: none

## Implementation Steps
1. Define shell motion budget and eligible components.
2. Reuse existing animation invalidation path for shell-only transitions.
3. Add subtle transition states to session bar/sidebar/overlay icons where useful.
4. Verify no persistent animation when shell is idle.

## Todo List
- [ ] Motion budget documented
- [ ] Shell-only animation hooks implemented
- [ ] Idle state stays still
- [ ] Performance regression check done

## Success Criteria
- Motion is noticeable but quiet.
- No measurable degradation during normal typing/scrolling.
- Terminal content render path remains behaviorally identical.

## Risk Assessment
- Risk: animation leaks into pane rendering and causes cache churn.
- Mitigation: isolate motion to chrome layers and invalidate minimally.

## Security Considerations
- No security surface change.
- Avoid motion patterns that obscure confirmations or status states.

## Next Steps
- Lock behavior with verification gates and docs sync.
- Unresolved questions: none.
