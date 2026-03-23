## Context Links
- `README.md`
- `docs/codebase-summary.md`
- `docs/system-architecture.md`
- `docs/code-standards.md`

## Overview
- Priority: P1
- Current status: pending
- Brief description: freeze exact shell-only scope, define allowed seams, inventory current ownership before any polish work.
- Explicit boundary: không đụng `crates/chatminal-terminal-core/**`; không thay runtime/store/session-engine contracts.

## Objective
- Chốt danh sách module thuộc UI shell.
- Chốt danh sách vùng cấm.
- Chốt acceptance rules: terminal core là black box, UI chỉ tiêu thụ snapshot/action đã có.

## Scope
- In scope: `apps/chatminal-desktop/src/termwindow/*`, `apps/chatminal-desktop/src/desktop_termwindow_*`, `apps/chatminal-desktop/src/tabbar.rs`, `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`, `apps/chatminal-desktop/src/overlay/*`, `apps/chatminal-desktop/src/chatminal_layout/*`, `apps/chatminal-desktop/src/chatminal_render/*`.
- Out of scope: `crates/chatminal-terminal-core/**`, `crates/chatminal-runtime/**`, `crates/chatminal-store/**`, `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/**`, any PTY/parser/state mutation path.

## Key Insights
- Desktop shell already owns chrome, layout mapping, hit-testing, and overlay routing.
- Sidebar mode is feature-flagged and already has its own render + wheel-scroll path.
- Footer is rendered inside sidebar render module, so shell chrome is already centralized enough for non-core polish.

## Requirements
- Functional: produce one explicit ownership matrix and no-touch matrix for follow-up phases.
- Non-functional: no speculative scope expansion, no new host/runtime dependency.

## Architecture
- Safe seam: `TermWindow` orchestrates render/input shell.
- Safe seam: `chatminal_sidebar` owns profile/session snapshot and local scroll state.
- Safe seam: `tabbar.rs` owns session bar composition.
- Safe seam: `overlay/*` owns shell overlays on top of terminal surface.

## Files Likely Touched
- Modify:
  - `apps/chatminal-desktop/src/termwindow/mod.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
  - `docs/system-architecture.md`
- Create: none expected
- Delete: none

## Implementation Steps
1. Document allowed shell modules vs no-touch modules.
2. Mark approved data seams: session snapshot, workspace layout snapshot, render target snapshot, overlay pane capability.
3. Mark forbidden edits: parser, terminal state machine, PTY flow, store schema, runtime event contract.
4. Convert this freeze into a checklist used by every later phase.

## Todo List
- [ ] Shell ownership matrix written
- [ ] No-touch list written
- [ ] Allowed seam list written
- [ ] Later phases linked to freeze rules

## Success Criteria
- Every later phase references explicit shell-only seams.
- No phase requires terminal-core or runtime-contract changes.
- Reviewers can reject scope creep mechanically.

## Risk Assessment
- Risk: polish work leaks into runtime because a UI gap feels easier to solve there.
- Mitigation: hard fail if a phase needs new protocol/state shape.

## Security Considerations
- No auth/data-model changes.
- Keep existing command routing and sandbox assumptions untouched.

## Next Steps
- Feed ownership matrix into layout primitive cleanup.
- Unresolved questions: none.
