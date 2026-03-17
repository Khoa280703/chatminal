# Phase 2: Dead Code Removal Report

## Status: Completed (partial — scope adjusted)

## Analysis Finding: Plan Document Error

Plan document claimed `split_and_insert` / `compute_split_size` were dead code with only caller at `domain.rs:140`. This is **incorrect**.

Actual caller chain (still live):
```
lua-bridge/leaf.rs:498
  → Mux::split_pane (lib.rs:1217)
  → domain::split_pane (deprecated default impl, domain.rs:74)
  → tab::compute_split_size (tab.rs:728)
  → tab::split_and_insert (tab.rs:740)
```

Compiler confirms: zero warnings about `split_and_insert` / `compute_split_size` in tab.rs → they are **live code** used by lua split functionality. Cannot delete without breaking lua-bridge pane splitting.

## Actual Dead Code Removed

Compiler warnings identified real dead code from Phase 1 desktop split removal:

### `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Deleted `impl DesktopSplitRequest { into_host_request() }` — 15 lines
  - Only caller was removed in Phase 1 (`split_terminal_handle`)

### `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- Deleted 3 type aliases (lines 88-90): `RuntimeSplitDirection`, `RuntimeSplitRequest`, `RuntimeSplitSize` — 3 lines
  - Only used in `into_host_request` (now deleted)
- Deleted `active_host_domain_name()` — 3 lines
- Deleted `set_default_host_domain()` — 3 lines
- Deleted `new_headless_connection_ui()` — 3 lines
- Deleted `host_client_domains()` — 3 lines

**Total removed: ~30 lines**

## Files Modified
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` (-15 lines)
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` (-15 lines)

## Files NOT Modified
- `crates/chatminal-host-runtime/src/tab.rs` — no dead code found (all used via lua-bridge)
- `crates/chatminal-host-runtime/src/domain.rs` — `split_pane` default impl still needed

## Tests Status
- `cargo check --workspace`: PASS (0 warnings)
- `cargo test --workspace --lib --bins --tests`: PASS (all ok)

## Notes
- tab.rs remains ~2528 lines — target ~500 line reduction is **not achievable** without removing lua split functionality
- If intent is to fully deprecate engine-based splitting, need to first replace `Mux::split_pane` in lua-bridge with session-native split API, then delete the chain
- That is a larger refactor outside Phase 2 scope

## Unresolved Questions
1. Should lua-bridge `SplitSession` be migrated to session-native split (chatminal-runtime API) instead of engine split (`Mux::split_pane`)? If yes, that would allow full deletion of tab.rs split mutation code in a Phase 2b.
