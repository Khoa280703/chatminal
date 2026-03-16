## Phase Implementation Report

### Executed Phases
- Phase: 07, 08, 09 of `plans/260313-1618-session-tab-collapse-host-render-scope-removal/`
- Status: completed

---

### Phase 07 — Reverse `chatminal-runtime` ↔ `chatminal-session-runtime` dependency

**Problem solved**: `chatminal-runtime` was using `chatminal-session-runtime` as dev-dep for integration tests, creating a diamond dependency — two distinct copies of `chatminal_runtime` types, incompatible with each other.

**Fix**: Created `crates/chatminal-runtime/src/state/test_bridge.rs` — a `TestExecutionBridge` that uses `portable-pty` directly (no session-runtime dependency). Spawns a reader thread to forward PTY output via `OnceLock<SyncSender<SessionEvent>>`. Removed `chatminal-session-runtime` from `chatminal-runtime/Cargo.toml` dev-deps.

**Key change**: Removed `#[cfg(test)]` from `RuntimeSessionHandleTrait::size()` — this method cannot be gated because when `chatminal-runtime` is compiled as dependency (not test target), the method disappears and implementations in `chatminal-desktop` and `chatminald` fail to compile.

**Result**: 65/65 `chatminal-runtime` tests pass.

---

### Phase 08 — Delete `crates/chatminal-session-runtime/`

Moved all 22 source files from `crates/chatminal-session-runtime/src/` into `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/` (new directory). Bulk-replaced `use crate::` → `use super::` across all files. Fixed test module paths (`super::X` → `super::super::X` for items from the `session_engine` container module, because test files declared via `#[path]` are inside submodules where `super::` = the source file's module, not the container).

Removed `crates/chatminal-session-runtime` from workspace `Cargo.toml`, all `Cargo.toml` dependencies, and deleted the directory.

**4 grep gates pass** (0 references to `chatminal-session-runtime`).

**Final compile fix**: `session_focus_manager.rs` test block had `super::LayoutNodeId::new(1)` — `LayoutNodeId` missing from `super::super::` import. Fixed by adding it to the import block.

**Result**: 55/55 `chatminal-desktop` tests pass.

---

### Phase 09 — Config Vocabulary: Tab/Pane → Session rename

**Scope**: All 5 categories (A through E) completed.

**Files modified**:

| Crate | Files |
|-------|-------|
| `chatminal-config` | `keyassignment.rs`, `config.rs`, `color.rs` |
| `chatminal-codec` | `lib.rs` |
| `chatminal-lua-bridge` | `leaf.rs`, `lib.rs`, `session.rs`, `window.rs` |
| `chatminal-host-runtime` | `lib.rs`, `domain.rs`, `tab.rs` |
| `chatminal-engine-client` | `client.rs`, `domain.rs` |
| `chatminal-engine-mux-server-impl` | `sessionhandler.rs` |
| `chatminal-desktop` | `desktop_commands.rs`, `desktop_termwindow_actions_impl.rs`, `desktop_termwindow_actions_items.rs`, `desktop_spawn.rs`, `tabbar.rs`, `termwindow/mod.rs`, `termwindow/resize.rs`, `desktop_termwindow_event_helpers.rs`, `overlay/launcher.rs`, `desktop_host_runtime/mod.rs`, `desktop_host_runtime/session_engine/*.rs`, `chatminal_runtime/mod.rs` |

**Key renames**:
- `SpawnTabDomain` → `SpawnSessionDomain`, `CurrentPaneDomain` → `CurrentSessionDomain`
- `PaneDirection` → `SessionDirection`
- `PaneSelectMode` / `PaneSelectArguments` → `SessionSelectMode` / `SessionSelectArguments`
- `SplitPane` struct/variant → `SplitSession`
- `TabBarColors` / `TabBarStyle` → `SessionBarColors` / `SessionBarStyle`
- All `enable_tab_bar`, `use_fancy_tab_bar`, `tab_bar_at_bottom`, etc. fields renamed
- `LeafRef` → `TerminalRef` (deprecated alias `type LeafRef = TerminalRef` preserved)
- `HandySplitDirection` → `SessionSplitDirection`
- `host_launcher_tabs` → `launcher_sessions`, `show_tab_bar` → `show_session_bar`
- `CloseCurrentTab` + `CloseCurrentPane` → `CloseCurrentSession` (deprecated `CloseCurrentPane` variant kept with `#[deprecated]`)
- `SpawnWhere::NewTab` → `SpawnWhere::NewSession`, `NewTabButton` → `NewSessionButton`

**Grep gate**: 0 non-deprecated results (remaining 3 results are: deprecated `CloseCurrentPane` variant def, deprecated `LeafRef` re-export, deprecated `CloseCurrentPane` match arm).

---

### Files Modified

- `crates/chatminal-runtime/src/state/test_bridge.rs` — new
- `crates/chatminal-runtime/src/state.rs`, `state/tests.rs`, `state/runtime_bridge.rs`, `server.rs`, `Cargo.toml`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/` — new dir (22 files)
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`, `engine_runtime_adapter.rs`, `session_host.rs`, `execution_bridge.rs`
- `apps/chatminal-desktop/Cargo.toml`, `apps/chatminald/Cargo.toml`, `apps/chatminald/src/main.rs`
- `Cargo.toml` (workspace)
- `crates/chatminal-session-runtime/` — deleted
- All files listed in Phase 09 table above

### Tests Status
- Type check: pass (`cargo check --workspace --all-targets`)
- `chatminal-runtime` tests: 65/65 pass
- `chatminal-desktop` tests: 55/55 pass
- Workspace: 1 pre-existing failure in `chatminal-app` (`build_start_args_contains_proxy_command`) — unrelated to this work, confirmed pre-existing on `main` before any changes

### Issues Encountered
- BSD sed doesn't support `\b` word boundaries — had to switch to `perl -i -pe` for word-boundary replacements
- `#[deprecated]` on enum variants: Rust doesn't allow deprecated alias variants, only `#[deprecated]` attribute on the variant itself. Added a `#[allow(deprecated)] CloseCurrentPane { .. } => return None` catch-all arm in `derive_command_from_key_assignment`
- PDU variant names in `chatminal-codec` followed struct names (e.g., `GetPaneDirection` → `GetSessionDirection`), requiring cascades into `chatminal-engine-mux-server-impl` and `chatminal-engine-client`

### Next Steps
- Phases 07, 08, 09 complete
- All plan phases in `260313-1618-session-tab-collapse-host-render-scope-removal/` done
- Docs update (`docs/codebase-summary.md`, `docs/system-architecture.md`) recommended to reflect `chatminal-session-runtime` removal and vocabulary rename
