# Phase Implementation Report

### Executed Phase
- Phase: consolidate-dirs-dependency
- Plan: none (standalone task)
- Status: completed

### Files Modified

**Cargo.toml (root)**
- Replaced `dirs-next = "2.0"` → `dirs = "6.0.0"` in `[workspace.dependencies]`

**crates/chatminal-runtime/Cargo.toml**
- `dirs = "6.0.0"` → `dirs.workspace = true`

**crates/chatminal-store/Cargo.toml**
- `dirs = "6.0.0"` → `dirs.workspace = true`

**crates/chatminal-config/Cargo.toml**
- `dirs-next.workspace = true` → `dirs.workspace = true`

**crates/chatminal-window/Cargo.toml**
- `dirs-next.workspace = true` → `dirs.workspace = true`

**crates/chatminal-env-bootstrap/Cargo.toml**
- `dirs-next.workspace = true` → `dirs.workspace = true`

**apps/chatminal-desktop/Cargo.toml**
- `dirs-next = { version = "2.0" }` → `dirs.workspace = true`

**Source files (dirs_next:: → dirs::)**
- `apps/chatminal-desktop/src/download.rs` — 1 occurrence
- `crates/chatminal-config/src/lib.rs` — 1 occurrence
- `crates/chatminal-config/src/config.rs` — 3 occurrences
- `crates/chatminal-window/src/os/x11/cursor.rs` — 1 occurrence
- `crates/chatminal-env-bootstrap/src/lib.rs` — 1 occurrence

### Tasks Completed
- [x] Add `dirs = "6.0.0"` to workspace deps
- [x] Remove `dirs-next = "2.0"` from workspace deps
- [x] Update 6 crate Cargo.toml files to use workspace `dirs`
- [x] Replace all `dirs_next::` imports with `dirs::` in 5 source files
- [x] `cargo check --workspace` — PASS (21.73s, 0 errors)

### Tests Status
- Type check: PASS
- Unit tests: not run (no logic change, API identical)

### Issues Encountered
- None. API surface of `dirs 6.0` is identical to `dirs-next 2.0`.

### Next Steps
- Task #29: Audit and clean crate-level `#![allow(dead_code)]`
