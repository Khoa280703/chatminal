## Context Links
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `plans/260319-1643-chatminal-sidebar-scroll-fix/plan.md`

## Overview
- Priority: P1
- Current status: pending
- Brief description: upgrade sidebar tree density, hierarchy readability, scroll affordance, and large-list behavior while staying inside shell render/input code.
- Explicit boundary: không đụng `crates/chatminal-terminal-core/**`; không đổi session persistence, profile schema, or explorer backend.

## Objective
- Make profile/session tree easier to scan and navigate at scale.
- Improve scroll behavior for long lists and dense nested states.

## Scope
- Tree row hierarchy, spacing, empty states, hover/active treatment, scrollbar affordance.
- Wheel handling, clipping, hit-testing, and local scroll-state robustness.

## Key Insights
- Sidebar already has local scroll state and clipping, so polish can stay local.
- Tree rows are built in one render module, making visual refinement cheap if geometry is stabilized first.
- Biggest coupling risk is leaking sidebar UX needs into runtime profile/session model. Avoid that.

## Requirements
- Functional: profiles, sessions, empty rows, and errors remain readable with many items.
- Non-functional: smooth wheel behavior, zero invisible click targets, no flicker during snapshot refresh.

## Architecture
- Keep `ChatminalSidebar` responsible for local UI state only: expansion, local active marker, scroll offset.
- Keep row composition in `termwindow/render/chatminal_sidebar.rs`.
- Keep mouse routing in `desktop_termwindow_mouseevent.rs`.

## Files Likely Touched
- Modify:
  - `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
  - `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- Create: none preferred
- Delete: none

## Implementation Steps
1. Refine row hierarchy: profile headers, nested session cards, empty/error rows.
2. Tighten vertical rhythm and indentation tokens.
3. Improve scroll thumb visibility and viewport clipping.
4. Recheck hit-test clipping after translation and resize.
5. Test large datasets: many profiles, many sessions, repeated expand/collapse.

## Todo List
- [ ] Dense tree rows readable
- [ ] Scrollbar/thumb polished
- [ ] Scroll offset clamp stable after resize/refresh
- [ ] Hit-test clipping stable

## Success Criteria
- 50+ visible tree items still navigable.
- Active session/profile is obvious without overpowering content.
- No bleed into footer or terminal pane.

## Risk Assessment
- Risk: local UI ordering diverges from runtime ordering.
- Mitigation: keep ordering source from existing runtime/session snapshot only.

## Security Considerations
- No change to create/activate/close authorization path.
- No persistence mutation beyond already supported sidebar actions.

## Next Steps
- Reuse polished tree tokens in session bar/footer chrome.
- Unresolved questions: none.
