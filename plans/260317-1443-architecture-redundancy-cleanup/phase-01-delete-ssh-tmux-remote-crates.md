# Phase 1.1: Delete SSH/tmux/remote crates

**Context:** [plan.md](./plan.md) | Tier 1 Critical | Blocks Phase 1.2

## Overview

- **Priority:** P1 (Critical)
- **Status:** completed
- **Effort:** 1-2h
- **Description:** Delete 4 unused crates + 5 host-runtime modules. Removes ~12,000 lines (9,057 crate + 3,171 module). Eliminates 3/4 callers of `tab.split_and_insert`.

## Key Insights

- `chatminal-engine-client/src/spawn_target.rs:923` calls `split_and_insert` — deleted with crate
- `host-runtime/src/tmux_commands.rs:280,455` — 2 more callers, deleted with module
- `spawn_target.rs:6` comment mentions SSH sessions — update needed
- `spawn_target.rs:484` sets `SSH_AUTH_SOCK` from `Mux.agent` — keep (unrelated to SSH crate)
- No other crate depends on these 4 crates (verified via workspace)

## Related Code Files

**Delete (crates):**
- `crates/chatminal-engine-ssh/` (entire directory)
- `crates/chatminal-engine-client/` (entire directory)
- `crates/chatminal-engine-mux-server-impl/` (entire directory)
- `crates/chatminal-ssh-funcs/` (entire directory)

**Delete (modules):**
- `crates/chatminal-host-runtime/src/ssh.rs` (1148 lines)
- `crates/chatminal-host-runtime/src/ssh_agent.rs` (218 lines)
- `crates/chatminal-host-runtime/src/tmux_commands.rs` (1198 lines)
- `crates/chatminal-host-runtime/src/tmux.rs` (435 lines)
- `crates/chatminal-host-runtime/src/tmux_pty.rs` (172 lines)

**Delete (desktop binaries):**
- `apps/chatminal-desktop/src/bin/chatminal-mux/` (uses `engine_mux_server_impl::PKI`)

**Modify:**
- `Cargo.toml` (workspace root)
- `crates/chatminal-host-runtime/Cargo.toml`
- `crates/chatminal-host-runtime/src/lib.rs`
- `crates/chatminal-host-runtime/src/spawn_target.rs`
- `apps/chatminal-desktop/Cargo.toml`
- `apps/chatminal-desktop/src/main.rs` (**~150 lines** — SSH/Connect subcommands, single-instance detection, mux server spawn)
- `apps/chatminal-desktop/src/update.rs` (line 187 — `discover_gui_socks`)
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` (line 978 — `update_mux_targets`)

## Implementation Steps

1. **Delete crate directories:**
   ```
   rm -rf crates/chatminal-engine-ssh
   rm -rf crates/chatminal-engine-client
   rm -rf crates/chatminal-engine-mux-server-impl
   rm -rf crates/chatminal-ssh-funcs
   ```

2. **Delete host-runtime modules:**
   ```
   rm crates/chatminal-host-runtime/src/{ssh,ssh_agent,tmux_commands,tmux,tmux_pty}.rs
   ```

3. **Update `Cargo.toml` (workspace root):**
   - Remove from `[workspace] members` (lines 33, 47, 55, 57):
     - `"crates/chatminal-ssh-funcs"`
     - `"crates/chatminal-engine-client"`
     - `"crates/chatminal-engine-mux-server-impl"`
     - `"crates/chatminal-engine-ssh"`
   - Remove from `[workspace.dependencies]` (lines 258, 259, 297, 306, 308):
     - `ssh-funcs = ...`
     - `ssh2 = "0.9.3"`
     - `engine-client = ...`
     - `engine-mux-server-impl = ...`
     - `engine-ssh = ...`

4. **Update `crates/chatminal-host-runtime/Cargo.toml`:**
   - Remove `engine-ssh.workspace = true` (line 52)

5. **Update `crates/chatminal-host-runtime/src/lib.rs`:**
   - Remove 5 mod declarations (lines 43-49):
     ```rust
     pub mod ssh;
     pub mod ssh_agent;
     pub mod tmux;
     pub mod tmux_commands;
     mod tmux_pty;
     ```

6. **Update `crates/chatminal-host-runtime/src/spawn_target.rs`:**
   - Update comment at line 1-6: remove mention of SSH sessions
   - Keep line 484 (`SSH_AUTH_SOCK`) — this is agent forwarding, not the SSH crate

7. **Update `apps/chatminal-desktop/Cargo.toml`:**
   - Remove (lines 100, 104, 106):
     ```
     engine-client = { workspace = true }
     engine-mux-server-impl = { workspace = true }
     engine-ssh = { workspace = true }
     ```

8. **Update `apps/chatminal-desktop/src/main.rs`:**
   - Remove `use engine_client::target::ClientTarget` (line 15)
   - Remove `use engine_mux_server_impl::update_mux_targets` (line 19)
   - Remove `async_run_ssh()` function (lines 145-210)
   - Remove `connect_to_auto_connect_targets()` function (lines 321-329)
   - Remove single-instance detection block using `engine_client::discovery` (lines 486-530)
   - Remove `spawn_mux_server()` function (lines 614-625)
   - Remove `engine_client::discovery::publish_gui_sock_path` (line 623)
   - Remove `SubCommand::Ssh` match arm (line 1186)
   - Remove `SubCommand::Connect` match arm (lines 1188+)
   - Remove SSH/Connect subcommand definitions from CLI enum

9. **Update `apps/chatminal-desktop/src/update.rs`:**
   - Line 187: remove or stub `engine_client::discovery::discover_gui_socks()`

10. **Update `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`:**
    - Line 978: remove `engine_mux_server_impl::update_mux_targets(config)?`

11. **Delete `apps/chatminal-desktop/src/bin/chatminal-mux/`:**
    - Entire binary uses `engine_mux_server_impl::PKI`

12. **Compile check:** fix any remaining imports/references surfaced by compiler

## Todo List

- [x] Delete 4 crate directories
- [x] Delete 5 host-runtime module files
- [x] Update workspace Cargo.toml members + deps
- [x] Update host-runtime Cargo.toml
- [x] Update host-runtime lib.rs mod declarations
- [x] Update spawn_target.rs comment
- [x] Update desktop Cargo.toml
- [x] Update desktop main.rs (~150 lines: SSH/Connect subcommands, single-instance, mux server)
- [x] Update desktop update.rs (discover_gui_socks)
- [x] Update desktop_host_runtime/mod.rs (update_mux_targets)
- [x] Delete chatminal-mux binary
- [x] Fix compiler errors
- [x] Run verification

## Success Criteria

- `cargo check --workspace` passes
- `cargo test -p chatminal-host-runtime` passes
- No references to deleted crates in any `.toml` file
- `split_and_insert` callers reduced to 1 (spawn_target.rs:140 only, excluding test/internal)

## Risk Assessment

- **Medium risk:** Desktop main.rs has ~150 lines using these crates (single-instance detection, SSH subcommand, mux server)
- **Watch:** Single-instance detection (`resolve_gui_sock_path`) — losing this means multiple desktop instances can launch. Desktop is deprecated so acceptable.
- **Watch:** `ssh2` crate removal — grep for `ssh2::` imports outside deleted files
- **Watch:** `spawn_target.rs:484` SSH_AUTH_SOCK — must keep, not related to engine-ssh
- **Watch:** `bin/chatminal-mux` binary depends on engine-mux-server-impl — delete entire binary

## Verification

```bash
cargo check --workspace
cargo test -p chatminal-host-runtime
# Confirm no dangling refs:
grep -r "engine-ssh\|engine-client\|engine-mux-server-impl\|ssh-funcs" --include="*.toml" .
grep -r "mod ssh\|mod tmux" crates/chatminal-host-runtime/src/lib.rs
```
