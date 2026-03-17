# Phase 1.3: Localize ID mapping

**Context:** [plan.md](./plan.md) | Tier 1 Critical | Depends on Phase 1.2

## Overview

- **Priority:** P1
- **Status:** completed
- **Effort:** 1-2h
- **Description:** Move `Mux.chatminal_session_id_index` from global Mux (host-runtime) to desktop-local `DesktopSessionHost`. Daemon only needs `session_id` + `RuntimeId`, not PaneId mapping.

## Results

- Deprecated `Mux::chatminal_session_id_index` field in host-runtime
- Added desktop-local lookup via `DesktopSessionHost::session_pane` map
- Migrated all desktop callers from `Mux::get_tab_by_chatminal_session_id` to `DesktopSessionHost` wrapper
- Updated runtime_bridge.rs docs to clarify daemon-only needs
- cargo check + cargo test pass

## Key Insights

- `chatminal_session_id_index` is a `RwLock<HashMap<String, PaneId>>` at `lib.rs:124`
- Used in 3 places within Mux:
  - `lib.rs:789` — `get_tab_by_chatminal_session_id()`: lookup pane by session_id
  - `lib.rs:814` — `add_pane()`: insert into index when pane has `chatminal_session_id` metadata
  - `lib.rs:855` — `remove_pane_internal()`: evict from index on pane removal
- `DesktopSessionHost` already has `session_pane: Mutex<HashMap<String, Arc<ChatminalSessionPane>>>` at `session_host.rs:59` — this is the desktop-native equivalent
- **Goal:** Daemon shouldn't know about PaneId mapping; that's a desktop concern

## Related Code Files

**Modify:**
- `crates/chatminal-host-runtime/src/lib.rs` (lines 122-124, 460, 789-792, 808-819, 849-858)
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` (add mapping method if not present)
- `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs` (update attachment resolution)
- `crates/chatminal-runtime/src/state/runtime_bridge.rs` (add doc comment)

## Implementation Steps

1. **Deprecate `chatminal_session_id_index` in Mux:**
   - `lib.rs:122-124`: Add `#[deprecated]` comment above the field
   - Keep the field for now (removing breaks add_pane/remove_pane); mark for removal in follow-up

2. **Add `get_pane_by_session_id` to `DesktopSessionHost`:**
   - `session_host.rs`: Add public method that looks up `session_pane` map (already at line 59)
   - This replaces `Mux::get_tab_by_chatminal_session_id` for desktop callers

3. **Migrate callers of `get_tab_by_chatminal_session_id`:**
   - Find all callers (likely in desktop_termwindow or execution_bridge)
   - Route them through `DesktopSessionHost` instead of global Mux

4. **Document daemon-only needs in runtime_bridge.rs:**
   - `runtime_bridge.rs:2-7`: Update comments to clarify daemon uses `session_id` + `RuntimeId` only, no PaneId

5. **Optional (follow-up):** Remove the Mux field entirely once all desktop callers migrated

## Todo List

- [x] Grep all callers of `get_tab_by_chatminal_session_id`
- [x] Add desktop-local lookup method to DesktopSessionHost
- [x] Migrate desktop callers to use DesktopSessionHost
- [x] Deprecate Mux.chatminal_session_id_index
- [x] Document daemon-only needs in runtime_bridge.rs
- [x] Run verification

## Success Criteria

- Desktop code no longer uses `Mux::get_tab_by_chatminal_session_id` for session lookup
- `cargo test -p chatminal-runtime` passes
- `cargo check -p chatminal-desktop` passes
- Mux field marked deprecated; no new callers added

## Risk Assessment

- **Medium risk:** Multiple call sites may depend on `get_tab_by_chatminal_session_id`
- **Mitigation:** Deprecate first, migrate incrementally, remove field in follow-up
- **Watch:** Thread safety — Mux uses RwLock, DesktopSessionHost uses Mutex

## Verification

```bash
cargo test -p chatminal-runtime
cargo check -p chatminal-desktop
# Confirm no direct Mux session_id index usage from desktop:
grep -rn "get_tab_by_chatminal_session_id\|chatminal_session_id_index" --include="*.rs" apps/
```
