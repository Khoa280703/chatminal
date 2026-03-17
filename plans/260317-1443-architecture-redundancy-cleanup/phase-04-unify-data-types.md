# Phase 2.1: Unify 3-layer data types

**Context:** [plan.md](./plan.md) | Tier 2 Medium | Can start after Phase 1.1

## Overview

- **Priority:** P2
- **Status:** completed
- **Effort:** 2-3h
- **Description:** Eliminate redundant `Runtime*` type wrappers in `chatminal-runtime`. Currently 39 `From<>` impls in `api/protocol.rs` (431 lines) do field-by-field copying between near-identical structs across 3 layers: `Stored*` (store) -> `Protocol*` (chatminal-protocol) -> `Runtime*` (runtime).

## Results

- 17 Runtime* types → type aliases re-exporting from chatminal-protocol
- protocol.rs deleted (431 lines) — all 39 From impls removed
- 5 Store↔Protocol From impls moved to chatminal-store/src/lib.rs
- All workspace imports updated to use Protocol types directly
- cargo check + cargo test pass

## Key Insights

- Three type layers with near-identical fields:
  - `chatminal-store`: `StoredSessionSummary`, `StoredSessionStatus`, `StoredProfile`, etc. (DB-specific, has `shell` field)
  - `chatminal-protocol`: `SessionInfo`, `SessionStatus`, `ProfileInfo`, etc. (wire/API types)
  - `chatminal-runtime`: `RuntimeSession`, `RuntimeSessionStatus`, `RuntimeProfile`, etc. (app-layer wrappers)
- `Runtime*` types add zero value — they're 1:1 copies of protocol types
- 39 `From<>` impls just map identical enum variants or copy fields
- Strategy: Replace `Runtime*` with type aliases or re-exports from `chatminal-protocol`

## Related Code Files

**Modify (heavy):**
- `crates/chatminal-runtime/src/api/mod.rs` — defines `Runtime*` types; replace with re-exports
- `crates/chatminal-runtime/src/api/protocol.rs` — 39 From impls; delete most, keep Store->Protocol only

**Modify (light — update import paths):**
- All files in `crates/chatminal-runtime/src/` that use `Runtime*` types
- `apps/chatminal-desktop/src/` files importing from runtime API
- `crates/chatminal-lua-bridge/src/` files using Runtime types

**Keep unchanged:**
- `crates/chatminal-store/src/lib.rs` — `Stored*` types are DB-specific, keep as-is
- `crates/chatminal-protocol/src/lib.rs` — becomes the canonical source

## Implementation Steps

1. **Audit `Runtime*` types in `api/mod.rs`:**
   - List each `Runtime*` type and its protocol counterpart
   - Identify any `Runtime*` type with extra fields (those must remain as structs)

2. **Replace identical `Runtime*` types with type aliases:**
   ```rust
   // Before:
   pub struct RuntimeSession { ... }
   // After:
   pub type RuntimeSession = chatminal_protocol::SessionInfo;
   ```

3. **Delete corresponding From impls in `protocol.rs`:**
   - Remove `From<SessionInfo> for RuntimeSession` and reverse
   - Keep `From<StoredSessionSummary> for SessionInfo` (store -> protocol is real conversion)

4. **Fix all import sites:** compiler-driven — `cargo check` will surface every breakage

5. **Re-export from api/mod.rs for backward compat:**
   ```rust
   pub use chatminal_protocol::SessionInfo as RuntimeSession;
   ```

## Todo List

- [x] Audit serde attributes on Runtime* vs Protocol* types
- [x] Audit Runtime* vs Protocol types (identify 1:1 matches)
- [x] Replace Runtime* with type aliases in api/mod.rs
- [x] Delete redundant From impls in protocol.rs
- [x] Delete protocol.rs file entirely
- [x] Fix compiler errors across workspace
- [x] Run full test suite

## Success Criteria

- `From<>` impl count in protocol.rs drops from 39 to ~10-15 (Store->Protocol only)
- `protocol.rs` under 150 lines (from 431)
- `cargo test --workspace` passes
- No Runtime* struct that is identical to its Protocol counterpart

## Risk Assessment

- **Medium risk:** Many call sites use `Runtime*` types; refactor is mechanical but wide
- **Mitigation:** Use type aliases first (backward compatible), then inline later
- **Watch:** Lua bridge may pattern-match on Runtime* enum variants — verify compatibility
- **Watch (CRITICAL):** Serde attributes — MUST audit `#[serde(rename)]` on Runtime* vs Protocol* BEFORE aliasing. If they differ, type alias will break serialization silently. Add pre-step: `grep -r 'serde' chatminal-protocol/src chatminal-runtime/src/api/mod.rs`

## Verification

```bash
cargo test --workspace
# Count remaining From impls:
grep -c "impl From<" crates/chatminal-runtime/src/api/protocol.rs
# Verify no orphan Runtime* structs:
grep -n "pub struct Runtime" crates/chatminal-runtime/src/api/mod.rs
```
