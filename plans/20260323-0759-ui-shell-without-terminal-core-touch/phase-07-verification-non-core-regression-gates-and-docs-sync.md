## Context Links
- `docs/code-standards.md`
- `docs/system-architecture.md`
- `docs/development-roadmap.md`
- `docs/project-changelog.md`

## Overview
- Priority: P1
- Current status: pending
- Brief description: verify shell polish did not mutate terminal behavior, then sync docs and rollout notes.
- Explicit boundary: không đụng `crates/chatminal-terminal-core/**`; validation must prove that claim.

## Objective
- Prove shell-only scope held.
- Catch regressions in layout, focus, scrolling, overlays, and render performance.

## Scope
- Compile/test commands, targeted smoke tests, visual/manual QA checklist, docs updates.
- No new feature work in this phase.

## Key Insights
- The risk is not shell code failing to compile; the real risk is shell edits subtly changing focus, hit-testing, or content viewport.
- Documentation must record black-box boundary so follow-up work does not reopen core touch.

## Requirements
- Functional: sidebar/session bar/footer/overlay all work across narrow and wide window sizes.
- Non-functional: no terminal behavior drift; docs reflect shell-only implementation.

## Architecture
- Validation centers on desktop app surface.
- Docs sync only after scope audit passes.

## Files Likely Touched
- Modify:
  - `docs/system-architecture.md`
  - `docs/codebase-summary.md`
  - `docs/development-roadmap.md`
  - `docs/project-changelog.md`
  - `apps/chatminal-desktop/Cargo.toml`
- Create: none expected
- Delete: none

## Implementation Steps
1. Run `cargo check --workspace` and `cargo check -p chatminal-desktop`.
2. Run desktop-targeted tests already present.
3. Execute manual QA matrix: resize, overlay open/close, sidebar long-list scroll, session switching, footer truncation, motion idle behavior.
4. Audit diff to confirm no touched files under terminal core and other black-box zones.
5. Update docs and changelog with shell-only scope.

## Todo List
- [ ] Compile checks pass
- [ ] Desktop tests pass
- [ ] Manual shell QA matrix passes
- [ ] No-touch audit passes
- [ ] Docs synced

## Success Criteria
- No changed files under `crates/chatminal-terminal-core/**`.
- Desktop shell behavior improved with no core regression.
- Docs clearly state shell-only scope and boundaries.

## Risk Assessment
- Risk: last-mile fix tries to patch a runtime/core bug.
- Mitigation: reject fix, document as follow-up outside this roadmap.

## Security Considerations
- Preserve existing confirm/close flows.
- No new persistence, IPC, or execution privileges.

## Next Steps
- Hand off to implementation via cook/code workflow using this plan.
- Unresolved questions: none.
