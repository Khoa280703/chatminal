# Architecture Redundancy Cleanup — 3-Tier Plan

## Completion Status

All 7 phases completed. Plan finalized on 2026-03-17. Workspace now has cleaner architecture with ~12,600 LOC removed, 3GB reference snapshot deleted, all redundant abstractions eliminated, and ghost references cleaned.

## Context

Chatminal (WezTerm fork) has 7 architectural redundancies from building product layer on top of WezTerm engine without removing old abstractions. Verified against source code (97% accuracy).

**Direction:** Daemon primary, Desktop deprecated (keep compiling). Remove WezTerm business layer (Mux/Tab/Window), keep rendering layer (font/term/GUI).

---

## Phase Files

| Phase | File | Status | Effort |
|-------|------|--------|--------|
| 1.1 | [Delete SSH/tmux/remote crates](./phase-01-delete-ssh-tmux-remote-crates.md) | completed | 1-2h |
| 1.2 | [Seal engine split path](./phase-02-seal-engine-split-path.md) | completed | 30min |
| 1.3 | [Localize ID mapping](./phase-03-localize-id-mapping.md) | completed | 1-2h |
| 2.1 | [Unify 3-layer data types](./phase-04-unify-data-types.md) | completed | 2-3h |
| 2.2 | [Document terminal parser](./phase-05-document-terminal-parser.md) | completed | 15min |
| 3.1 | [Clean ghost references](./phase-06-clean-ghost-references.md) | completed | 15min |
| 3.2 | [Delete third_party reference](./phase-07-delete-third-party-reference.md) | completed | 15min |

---

## TIER 1 — Critical (Split Layout + Identity Mapping + Unused Engine)

### [Phase 1.1: Delete SSH/tmux/remote crates](./phase-01-delete-ssh-tmux-remote-crates.md)

Removes 3/4 callers of `tab.split_and_insert`. ~12,000 lines deleted (4 crates + 5 modules).

### [Phase 1.2: Seal engine split path](./phase-02-seal-engine-split-path.md)

After 1.1, `split_and_insert` has 1 caller: `domain.rs:140`. Reduce visibility, deprecate, add tracking.

### [Phase 1.3: Localize ID mapping](./phase-03-localize-id-mapping.md)

Move `Mux.chatminal_session_id_index` from global Mux to desktop-local `session_host.rs`.

---

## TIER 2 — Medium (Data Type Dedup + Terminal Parser)

### [Phase 2.1: Unify 3-layer data types](./phase-04-unify-data-types.md)

39 `From<>` impls in `api/protocol.rs` (431 lines). Replace `Runtime*` types with re-exports from `chatminal-protocol`.

### [Phase 2.2: Document terminal parser split](./phase-05-document-terminal-parser.md)

No code change. Add doc comments to `chatminal-terminal-core` and `chatminal-engine-term`.

---

## TIER 3 — Low (Cleanup)

### [Phase 3.1: Clean ghost crate references](./phase-06-clean-ghost-references.md)

Replace 12 stale `chatminal-session-runtime` references in comments across 8 .rs files.

### [Phase 3.2: Delete third_party/terminal-engine-reference](./phase-07-delete-third-party-reference.md)

Remove ~3GB WezTerm reference snapshot.

---

## Dependencies

```
Phase 1.1 → 1.2 → 1.3
                ↓
Phase 2.1 (can start after 1.1)
Phase 2.2, 3.1, 3.2 (independent, anytime)
```

## LOC Impact

| Phase | Removed | Added | Net |
|-------|---------|-------|-----|
| 1.1 | ~12,300 (4 crates + 5 modules + ~300 desktop main.rs/update.rs/mod.rs + chatminal-mux binary) | ~20 | -12,280 |
| 1.2 | 0 | ~15 | +15 |
| 1.3 | ~30 | ~50 | +20 |
| 2.1 | ~350 | ~30 | -320 |
| 3.2 | ~3GB | 0 | -3GB |
