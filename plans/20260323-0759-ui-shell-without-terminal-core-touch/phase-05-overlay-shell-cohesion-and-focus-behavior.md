## Context Links
- `apps/chatminal-desktop/src/overlay/mod.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `apps/chatminal-desktop/src/overlay/prompt.rs`
- `apps/chatminal-desktop/src/overlay/confirm.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`

## Overview
- Priority: P2
- Current status: pending
- Brief description: unify overlay shell styling and focus behavior so launcher/prompt/confirm/selector feel consistent with sidebar/session chrome.
- Explicit boundary: không đụng `crates/chatminal-terminal-core/**`; không đổi overlay terminal allocation semantics or pane capability contracts.

## Objective
- Clean up overlay chrome, spacing, focus return, and close behavior.
- Keep overlays shell-only and visually consistent with surrounding UI.

## Scope
- Overlay frame styling, spacing, list row affordances, confirm CTA hierarchy, focus/close consistency.
- No changes to overlay business logic beyond shell behavior polish.

## Key Insights
- Overlays already sit behind `start_overlay` and shell helpers.
- Biggest risk is mixing visual polish with action-routing changes. Avoid.
- Focus/close behavior belongs in shell orchestration, not terminal core.

## Requirements
- Functional: overlay open/close/focus behavior remains deterministic.
- Non-functional: same shell tokens and hover states as sidebar/session bar; no new overlay lifecycle race.

## Architecture
- Keep `overlay/mod.rs` and individual overlay modules responsible for view composition.
- Keep routing in `desktop_termwindow_host_runtime_helpers.rs` and action helpers.
- Reuse Phase 02 tokens, not new overlay-specific constants scattered everywhere.

## Files Likely Touched
- Modify:
  - `apps/chatminal-desktop/src/overlay/mod.rs`
  - `apps/chatminal-desktop/src/overlay/launcher.rs`
  - `apps/chatminal-desktop/src/overlay/prompt.rs`
  - `apps/chatminal-desktop/src/overlay/confirm.rs`
  - `apps/chatminal-desktop/src/overlay/selector.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- Create: none preferred
- Delete: none

## Implementation Steps
1. Standardize overlay container padding, border, and title treatment.
2. Normalize list row hover/active styling across launcher/selector.
3. Audit close reasons and focus return path after cancel/confirm.
4. Validate overlay placement against sidebar and footer bounds.

## Todo List
- [ ] Shared overlay chrome rules applied
- [ ] Focus return after overlay close verified
- [ ] Overlay row states consistent
- [ ] No shell overlap regressions

## Success Criteria
- Overlays feel like part of same desktop shell.
- Cancel/confirm paths return focus to prior terminal/view reliably.
- No visual clipping against sidebar/footer.

## Risk Assessment
- Risk: overlay focus bugs feel like terminal bugs.
- Mitigation: keep action routing unchanged unless necessary for focus restore only.

## Security Considerations
- Do not widen overlay command surface.
- Keep existing confirmation semantics intact.

## Next Steps
- Add light motion only after shell states are visually consistent.
- Unresolved questions: none.
