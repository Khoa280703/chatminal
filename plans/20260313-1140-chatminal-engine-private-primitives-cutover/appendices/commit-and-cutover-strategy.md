# Commit And Cutover Strategy

Purpose: tránh lặp lại các batch refactor quá lớn, khó review, khó dọn dead path.

## Commit Classes
- `refactor(boundary): ...`
  - introduce types, ownership moves, visibility tightening
- `refactor(termwindow): ...`
  - rename maps, product vocabulary cutover
- `refactor(lua): ...`
  - scripting/config surface cutover
- `chore(cleanup): ...`
  - dead path delete, dead_code removal, import prune
- `docs(architecture): ...`
  - docs sync only

## Phase Cutover Strategy
- Phase 01
  - docs/plan only, no behavior change
- Phase 02
  - commit 1: add boundary types
  - commit 2: swap facade return types
- Phase 03
  - commit 1: runtime lookup ownership
  - commit 2: shrink host public helpers
- Phase 04
  - commit 1: type and enum rename layer
  - commit 2: action routing rename
  - commit 3: render/session bar rename + shim removal
- Phase 05
  - commit 1: add Chatminal-facing Lua APIs
  - commit 2: deprecate/delete host-id APIs
- Phase 06
  - commit 1: privatize exports
  - commit 2: delete dead modules/helpers
- Phase 07
  - commit 1: all-targets fix/delete policy
  - commit 2: docs sync + closeout

## Rules
- No mixed commit that both introduces new compatibility shim and deletes unrelated dead paths.
- Any temporary shim must have a named removal phase in the commit message/body or nearby comment.
- Before moving to next phase, previous phase shims must be either justified or deleted.
