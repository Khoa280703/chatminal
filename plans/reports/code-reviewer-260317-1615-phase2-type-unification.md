# Code Review: Phase 2 -- Type Unification & Doc Comments

**Score: 9/10**

## Scope

| File | Change |
|------|--------|
| `crates/chatminal-runtime/src/api/mod.rs` | 17 struct/enum replaced with type aliases |
| `crates/chatminal-runtime/src/api/protocol.rs` | DELETED (431 LOC, 39 From impls) |
| `crates/chatminal-store/src/lib.rs` | 5 Store-to-Protocol From impls + ORDER BY change |
| `crates/chatminal-terminal-core/src/lib.rs` | Module doc added |
| `crates/chatminal-engine-term/src/lib.rs` | Module doc updated |

Net: ~530 lines removed, ~70 added. Excellent reduction.

## Build & Tests

- `cargo check --workspace`: PASS (4 pre-existing dead_code warnings in desktop_host_runtime, unrelated)
- `cargo test --workspace --lib --bins`: ALL PASS
- No new warnings introduced

## Type Alias Correctness

All 17 aliases verified against `chatminal-protocol::lib.rs`:

| Alias | Protocol Type | Match |
|-------|--------------|-------|
| RuntimeSessionStatus | SessionStatus | OK |
| RuntimeProfile | ProfileInfo | OK |
| RuntimeSession | SessionInfo | OK |
| RuntimeWorkspace | WorkspaceState | OK |
| RuntimeCreatedSession | CreateSessionResponse | OK |
| RuntimeLifecyclePreferences | LifecyclePreferences | OK |
| RuntimeSessionSnapshot | SessionSnapshot | OK |
| RuntimeSessionExplorerState | SessionExplorerState | OK |
| RuntimeSessionExplorerEntry | SessionExplorerEntry | OK |
| RuntimeSessionExplorerFileContent | SessionExplorerFileContent | OK |
| RuntimePtyOutputEvent | PtyOutputEvent | OK |
| RuntimePtyExitedEvent | PtyExitedEvent | OK |
| RuntimePtyErrorEvent | PtyErrorEvent | OK |
| RuntimeSessionUpdatedEvent | SessionUpdatedEvent | OK |
| RuntimeWorkspaceUpdatedEvent | WorkspaceUpdatedEvent | OK |
| RuntimeDaemonHealthEvent | DaemonHealthEvent | OK |
| RuntimeEvent | Event | OK |

## Store From Impls

5 conversions added, all correct:
- `StoredSessionStatus -> SessionStatus` (bidirectional, 2 impls)
- `StoredProfile -> ProfileInfo`
- `StoredSessionSummary -> SessionInfo`
- `StoredSessionSnapshot -> SessionSnapshot`

**Orphan rule**: No issues. `chatminal-store` owns `Stored*` types and depends on `chatminal-protocol` for target types. Foreign trait (`From`) on local type = valid.

## Serde Compatibility

Protocol types carry `#[derive(Serialize, Deserialize)]`. The old Runtime types did NOT have serde derives.

**Impact**: Runtime consumers now get Serialize/Deserialize for free through the aliases. This is a **net positive** -- no breakage, only additive capability. Code that previously used `RuntimeSessionStatus::Running` still works because the variant names match between the old enum and the protocol enum.

**One nuance**: `SessionStatus` has `#[serde(rename_all = "snake_case")]` so serialization produces `"running"` / `"disconnected"`. Any code that was manually serializing these (unlikely, since the old types lacked serde) would need to match this format. Given the old types didn't derive Serialize, this is a non-issue.

## Downstream Breaking Changes

**None detected.** Type aliases are transparent -- all existing code using `RuntimeSession`, `RuntimeProfile` etc. continues to work. Field names and types in the protocol structs match the old definitions exactly (verified via diff).

## Medium Priority Observations

### 1. Sneaked-in ORDER BY change (unrelated to type unification)

```sql
-- Before
ORDER BY updated_at DESC, created_at ASC
-- After
ORDER BY created_at ASC, rowid ASC
```

Applied to both `list_profiles_with_conn` and `list_sessions_for_profile_with_conn`. This is a **behavioral change** -- profiles/sessions now sort by creation time (oldest first) instead of most-recently-updated first.

**Risk**: Low, but this changes user-visible ordering in the sidebar. Should be called out in commit message or tracked separately.

### 2. Missing reverse From impls in chatminal-store

The old `protocol.rs` had bidirectional conversions for some Store types (e.g., `RuntimeSessionStatus -> StoredSessionStatus`). The new Store impls only go one direction for profiles, sessions, and snapshots:
- `StoredProfile -> ProfileInfo` exists, but `ProfileInfo -> StoredProfile` does not
- Same for `StoredSessionSummary -> SessionInfo` (no reverse)

**Risk**: None currently (reverse impls weren't used in the codebase), but worth noting. The status conversion IS bidirectional, which is the one that matters for write paths.

## Doc Comments

Both doc comments are accurate and helpful:
- `chatminal-terminal-core`: correctly identifies it as lightweight shared types with zero heavy deps
- `chatminal-engine-term`: correctly identifies it as full termwiz-based emulator, references the counterpart

## Summary

Clean, well-executed refactoring. The 431-line `protocol.rs` with its 39 mechanical `From` impls was pure boilerplate -- replacing it with type aliases is the correct approach. No orphan rule issues, no breaking changes, serde gains are additive. The ORDER BY change should be documented separately.

## Recommended Actions

1. **Low**: Document the ORDER BY change in commit message or separate commit
2. **Info**: Consider whether `RuntimeSessionLaunchSpec` (the one remaining unique struct) could also be moved to protocol in a future pass
