# Phase 5: Docs Update

## Overview
- **Priority**: P2
- **Status**: pending
- **Effort**: 15min
- **Blocked by**: Phase 1-4

Update architecture docs to reflect Phase 2 changes.

## Files to Update

### `docs/system-architecture.md`
- Note engine split path fully removed (not just sealed)
- Document workspace layout persistence (JSON blob in app_state)
- Update component diagram if tab.rs split code was removed

### `docs/architecture-analysis.md`
- Mark remaining issues from Phase 1 analysis as resolved
- Update line counts / code reduction metrics

### `docs/codebase-summary.md`
- Update crate descriptions if significant code removed
- Note `chatminal-store` now handles layout persistence

## Implementation Steps

1. Read current state of each doc
2. Update sections reflecting completed changes
3. Remove stale TODO items that were addressed
4. Add metrics: lines removed, functions deleted

## Success Criteria
- Docs accurately reflect post-Phase-2 codebase
- No stale references to removed code
