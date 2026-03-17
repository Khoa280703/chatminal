## Phase Implementation Report

### Executed Phase
- Phase: phase-01-delete-ssh-tmux-remote-crates
- Plan: plans/260317-1443-architecture-redundancy-cleanup/
- Status: completed

### Files Modified

**Deleted (crates):**
- `crates/chatminal-engine-ssh/` (entire)
- `crates/chatminal-engine-client/` (entire)
- `crates/chatminal-engine-mux-server-impl/` (entire)
- `crates/chatminal-ssh-funcs/` (entire)

**Deleted (modules):**
- `crates/chatminal-host-runtime/src/{ssh,ssh_agent,tmux,tmux_commands,tmux_pty}.rs`

**Deleted (binary):**
- `apps/chatminal-desktop/src/bin/chatminal-mux/`

**Modified:**
- `Cargo.toml` — removed 4 workspace members + 5 workspace.dependencies (engine-ssh, engine-client, engine-mux-server-impl, ssh-funcs, ssh2)
- `crates/chatminal-host-runtime/Cargo.toml` — removed engine-ssh dep
- `crates/chatminal-host-runtime/src/lib.rs` — removed 5 mod declarations, AgentProxy import/usage, `agent` field from Mux struct
- `crates/chatminal-host-runtime/src/domain.rs` — updated comment, removed agent env injection
- `crates/chatminal-host-runtime/src/client.rs` — replaced AgentProxy::default_ssh_auth_sock() with direct std::env::var
- `crates/chatminal-host-runtime/src/localpane.rs` — removed TmuxDomain/TmuxDomainState/ssh refs, simplified tmux-gated methods
- `crates/chatminal-config/Cargo.toml` — removed engine-ssh dep
- `crates/chatminal-config/src/ssh.rs` — replaced default_domains() with empty vec (no engine_ssh parser)
- `crates/chatminal-env-bootstrap/Cargo.toml` — removed ssh-funcs dep
- `crates/chatminal-env-bootstrap/src/lib.rs` — removed ssh_funcs::register
- `apps/chatminal-desktop/Cargo.toml` — removed engine-client, engine-mux-server-impl, engine-ssh deps
- `apps/chatminal-desktop/src/main.rs` — removed Ssh/Connect subcommands, async_run_ssh, run_ssh, connect_to_auto_connect_domains, Publish enum, spawn_mux_server, update_mux_domains callback
- `apps/chatminal-desktop/src/update.rs` — removed discover_gui_socks, simplified update toast logic
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` — removed update_mux_domains call, removed create_remote_ssh_domain fn

### Tasks Completed

- [x] Delete 4 crate directories
- [x] Delete 5 host-runtime module files
- [x] Update workspace Cargo.toml members + deps
- [x] Update host-runtime Cargo.toml
- [x] Update host-runtime lib.rs mod declarations
- [x] Update domain.rs comment
- [x] Update desktop Cargo.toml
- [x] Update desktop main.rs (~150 lines: SSH/Connect subcommands, single-instance, mux server)
- [x] Update desktop update.rs (discover_gui_socks)
- [x] Update desktop_host_runtime/mod.rs (update_mux_domains)
- [x] Delete chatminal-mux binary
- [x] Fix compiler errors (also fixed chatminal-config, chatminal-env-bootstrap, localpane.rs)
- [x] Run verification

### Tests Status
- Type check: PASS (`cargo check --workspace` — only warnings, no errors)
- Unit tests: PASS (`cargo test -p chatminal-host-runtime` — 4/4 passed)
- Integration tests: N/A

### Issues Encountered

1. Phase file underspecified: `chatminal-config` depended on `engine-ssh` (for `SshDomain::default_domains`), `chatminal-env-bootstrap` depended on `ssh-funcs` — both needed cleanup too
2. `localpane.rs` had heavy tmux/ssh coupling not mentioned in phase file — cleaned up
3. `AgentProxy` removed from lib.rs/client.rs — replaced ssh_auth_sock with direct env var read
4. `create_remote_ssh_domain` fn in desktop_host_runtime/mod.rs also deleted (dead after removing SSH subcommand)

### Next Steps
- Phase 1.2 unblocked: Seal engine split path (`split_and_insert` now has 1 caller)
