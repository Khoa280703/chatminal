# Architecture Redundancy Cleanup — Phase Completion Report

**Date:** 2026-03-17 | **Plan:** 260317-1443-architecture-redundancy-cleanup

## Summary

All Tier 1 (critical) + Tier 2 (medium) phases completed. 5 phases marked complete:
- Phase 1.1: Delete SSH/tmux/remote crates
- Phase 1.2: Seal engine split path
- Phase 1.3: Localize ID mapping
- Phase 2.1: Unify 3-layer data types
- Phase 2.2: Document terminal parser split

Tier 3 (cleanup) phases remain pending: Ghost reference cleanup (3.1) and third_party deletion (3.2).

## Completion Status by Phase

| Phase | Title | Status | Key Results |
|-------|-------|--------|-------------|
| 1.1 | Delete SSH/tmux/remote crates | completed | 4 crates + 5 modules deleted; ~12,000 lines removed; split_and_insert callers reduced from 4 to 1 |
| 1.2 | Seal engine split path | completed | split_and_insert → pub(crate); split_pane deprecated; fallback tracking added |
| 1.3 | Localize ID mapping | completed | Mux.chatminal_session_id_index deprecated; desktop-local mapping via DesktopSessionHost |
| 2.1 | Unify 3-layer data types | completed | 17 Runtime* types → type aliases; protocol.rs (431 lines) deleted; 39 From impls consolidated |
| 2.2 | Document terminal parser | completed | Doc comments added to terminal-core (daemon-only) + engine-term (desktop) |
| 3.1 | Clean ghost references | pending | Ready for Phase 3 |
| 3.2 | Delete third_party reference | pending | Ready for Phase 3 |

## Phase 2.1 Implementation Detail

**Type Deduplication Results:**
- Identified 17 Runtime* type aliases that were 1:1 copies of Protocol types
- Converted to re-exports: `pub type RuntimeSession = chatminal_protocol::SessionInfo`
- Deleted protocol.rs entirely (431 lines, 39 From impls)
- Moved 5 real Store→Protocol conversions to chatminal-store/src/lib.rs
- Fixed all import sites across workspace (mechanical refactor)

**Verification:**
- cargo check --workspace: PASS
- cargo test --workspace: PASS

## Phase 2.2 Implementation Detail

**Documentation:**
- chatminal-terminal-core/src/lib.rs: Added doc comments explaining lightweight core types for daemon use
- chatminal-engine-term/src/lib.rs: Added doc comments explaining full termwiz emulator for desktop use
- Clarified ownership boundaries and dependency direction

**Verification:**
- cargo doc -p chatminal-terminal-core -p chatminal-engine-term: PASS (clean docs)

## Architecture Impact

**Completed Tiers (1.2) Progress:**
- Tier 1 (Critical): ~12,000 lines deleted; target-split path sealed; ID mapping localized
- Tier 2 (Medium): Type deduplication complete; terminal parser ownership documented

**Remaining Tier 3 (Low):**
- Phase 3.1: Replace 12 stale `chatminal-session-runtime` references in comments (~8 files)
- Phase 3.2: Delete ~3GB WezTerm reference snapshot in third_party/ (storage cleanup)

## Quality Metrics

| Metric | Result |
|--------|--------|
| Compilation | PASS (no warnings from these changes) |
| Test Suite | PASS (all tests pass) |
| Lines Removed (1.1 + 2.1) | ~12,431 lines |
| Type Aliases | 17 unified |
| From Impls Removed | 39 → 5 (kept Store→Protocol only) |

## Next Steps

1. **Phase 3.1:** Clean ghost references (15min effort, no blocking dependencies)
2. **Phase 3.2:** Delete third_party WezTerm snapshot (storage optimization)
3. **Post-cleanup:** Update docs/system-architecture.md with final state

## Unresolved Questions

None. All completed phases verified with cargo check/test. Phase 3 phases remain pending and ready to execute.
