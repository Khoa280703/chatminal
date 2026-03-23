# Review: UI Shell Without Terminal Core Touch Plan

**Date:** 2026-03-23 | **Reviewer:** planner | **Plan:** `260323-0759-ui-shell-without-terminal-core-touch`

## Issues Found

### CRITICAL
1. **Direction misalignment** - Plan implicitly treats desktop as secondary/deprecated. Desktop is now PRIMARY (only frontend after daemon removal). Language needs update.
   - Affected: `plan.md` framing, delivery rules

### HIGH
2. **Phase 1 (Boundary Freeze) is overhead** - For solo dev, a full inventory/naming freeze phase before touching code is busywork. The boundary rules are already clear in plan.md. Merge essential content (no-touch list, safe-zone definition) into Phase 2 header.
   - Impact: saves 0.5d, removes blocking dependency
3. **Phase 6 (Icon Animation) is premature** - Touches GPU render pipeline (`paint.rs`, `has_animation`), high regression risk for cosmetic gain. No existing animation infrastructure beyond `has_animation` flag. Should defer until core shell is stable.
   - Impact: saves 1d, removes risk vector
4. **Phase 3 effort underestimated** - Sidebar rebuild at 1.5d is optimistic. WezTerm render pipeline has deep coupling between render/hit-test/scroll. Realistic: 2.5d.

### MEDIUM
5. **Success metrics too vague** - "nhat quan hon" (more consistent) not measurable. Need concrete criteria per phase.
6. **Phase 7 too formal for current stage** - Full regression matrix/rollback rules assumes team. Simplify to build gate + manual smoke checklist merged into each phase's exit criteria.

### LOW
7. **All file references verified** - Every referenced file exists in codebase. No broken refs.
8. **No deleted module dependencies** - Recent cleanup (connui.rs, update.rs, engine split) does not affect any files referenced in this plan.

## Recommended Phase Structure

| Original | Action | New Phase | Effort |
|----------|--------|-----------|--------|
| Phase 1 (Boundary Freeze) | **MERGE into Phase 2** | - | - |
| Phase 2 (Layout Primitives) | Keep, absorb P1 boundary content | Phase 1 | 1.5d |
| Phase 3 (Sidebar Rebuild) | Keep, re-estimate | Phase 2 | 2.5d |
| Phase 4 (Session Bar/Footer) | Keep | Phase 3 | 1d |
| Phase 5 (Overlay Polish) | Keep | Phase 4 | 1d |
| Phase 6 (Icon Animation) | **DEFER** - remove from plan | - | - |
| Phase 7 (Verification) | **MERGE** exit criteria into each phase | - | - |

## Updated Total Effort
- Old: 6.5d (7 phases)
- New: **6d** (4 phases) - more realistic due to sidebar re-estimate

## Concrete Success Criteria (per phase)

**Phase 1 (Layout Primitives):**
- Single source of truth for sidebar/footer/content bounds (one struct/fn, not scattered)
- `cargo check -p chatminal-desktop` passes
- Session bar top/bottom + sidebar on/off render correctly (manual smoke)

**Phase 2 (Sidebar Rebuild):**
- Scroll 50+ sessions without ghost hitbox or clipping artifacts
- Click targets match visible rows after scroll
- Expand/collapse preserves scroll position within 1 row

**Phase 3 (Session Bar/Footer):**
- Tab titles truncate with ellipsis, no overflow
- Footer stays within bounds during resize
- Hover areas >= 24px hit target

**Phase 4 (Overlay Polish):**
- All overlay types (launcher/quickselect/confirm/copy) use same padding/close affordance
- Cancel cleans up completely (no stale overlay state)
- Overlay respects content bounds from Phase 1

## Unresolved Questions
- None. All file refs verified, no dependency conflicts.
