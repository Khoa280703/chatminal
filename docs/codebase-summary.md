# Codebase Summary

Last updated: 2026-04-01 (architecture unification phase 01, unified desktop shell)

## Runtime baseline
Chatminal hiện là **single-process desktop** với hai lớp:
- `apps/chatminal-desktop` — desktop app first-party, render/input shell + desktop facade + private engine adapter.
- `crates/chatminal-runtime` — embedded orchestrator (không daemon), persistence facade, session state, startup recipes, persist worker.

Daemon `chatminald` đã hoàn toàn bị xóa; mọi session management hiện nằm trong desktop process.

Desktop product path là `single-flow local-first`:
- startup không còn public legacy flags cho attach/spawn selection
- desktop host mux luôn boot theo default local flow
- active keyassignment/config path không còn các action legacy kiểu spawn/attach/detach theo execution target
- host-runtime không còn `SpawnTarget.attach/detach/state` compat flow
- active host engine đã đổi naming sang `spawn target`; legacy vocabulary cũ không còn nằm trong active product path
- desktop app entry mặc định luôn boot unified shell path; sidebar không còn gate bởi env runtime flag

## Current cleanup status
- Architecture unification Phase 01 complete:
  - deleted dead config surface for SSH/TLS/WSL
  - deleted auto-update config stubs
  - removed WSL branch from `crates/chatminal-host-runtime/src/spawn_target.rs`
  - removed root workspace dependency `libssh-rs`
- Intentional keep:
  - `crates/chatminal-config/src/ssh.rs` now only contains minimal active helpers `SshParameters` and `username_from_env()`
  - `engine-gui-subcommands` and `engine-toast-notification` remain active

## High-signal modules

### Desktop facade and shell
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - desktop-facing bindings/query/action cho session/view/render-target/terminal handle/window snapshot.
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
  - desktop runtime client dùng để resolve/dispatch actions qua runtime boundary.
- `apps/chatminal-desktop/src/termwindow/mod.rs`
  - coordinator cho render/input/overlay shell.
- `apps/chatminal-desktop/src/tabbar.rs`
  - product-facing session bar state.
- `apps/chatminal-desktop/src/chatminal_layout/*`
  - layout helpers cho workspace/session views.
- `apps/chatminal-desktop/src/chatminal_render/*`
  - render DTO/adapters dùng boundary types mới.

### Desktop private adapter
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - private adapter boundary duy nhất còn biết host runtime.
  - Phase 2 cleanup: deleted `split_terminal_handle*`, `HostSplitSource`, `HostRuntimeEntryId`, `HostLayoutNode`, `HostSplitDirection` type aliases; removed 4 dead functions.
  - Single-flow cleanup: desktop mux init luôn default `local`; product path không còn resolve startup target từ CLI/config public surface, cũng không còn wrapper attach-by-target.
  - Cleanup mới nhất: internal naming đổi sang `HostSpawnTarget*`, `spawn_target.rs`, `spawn_target_id()`.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - private session host: builds `ChatminalRenderState` trực tiếp từ `session_pane` map; `HostRenderScope` chỉ còn cho overlay compat.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
  - terminal pane bridge cho output/input/runtime metadata.
- **DELETED** `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs` — Phase 2 cleanup (was only 250 LOC, subsumed by session_engine)
- **DELETED** `apps/chatminal-desktop/src/desktop_host_runtime/pane.rs` — Phase 2 cleanup (528 LOC test-only wrapper, tests refactored)

### Runtime and execution core (single-process embedded)
- `crates/chatminal-runtime/src/state/native_api.rs`
  - desktop/native API cho workspace/session lifecycle.
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
  - mapping giữa persisted/runtime-facing state.
- `crates/chatminal-runtime/src/state/persist_worker.rs`
  - background thread, coalescing persist jobs, zero lock contention.
- `crates/chatminal-runtime/src/state/session_event_processor.rs`
  - in-memory event processor, 5 SQLite writes → 0 under lock.
- `crates/chatminal-runtime/src/state/startup_recipes.rs`
  - per-session startup command registry (run/type/enter/wait/wait-for).
- `crates/chatminal-runtime/src/api/mod.rs`
  - app-facing boundary ids/snapshots.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/mod.rs`
  - session engine facade.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime.rs`
  - PTY instance runtime (CoreTerminal + IoTerminal, 3K scrollback lines).
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/output_history.rs`
  - output history bounded to 512KB per session.
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - desktop facade bindings for session/view/render-target/terminal handle/window snapshot.

### Lower engine/private compatibility
- `crates/chatminal-host-runtime/*`
  - lower engine host internals; được phép giữ `Mux/Tab/Pane`.
  - Cleanup mới nhất đã bỏ `SpawnTarget.attach/detach/state`, `spawnable`, `target_label`, `iter_targets`, `target_was_detached`, rồi đổi naming lõi sang `SpawnTarget`.
- `crates/chatminal-config/*`
  - config/runtime boundary giờ cũng dùng vocabulary `target`.
  - public fields đã đổi sang `exec_targets`, `wsl_targets`, `ssh_targets`, `unix_targets`, `default_target`, `default_mux_server_target`.
  - Lua helper đổi sang `default_wsl_targets()` và `exec_target(...)`.
- `crates/chatminal-lua-bridge/src/lib.rs`
  - Lua/config bridge theo vocabulary Chatminal.
  - các helper public để query/chọn execution target đã bị cắt khỏi public surface.
  - `terminal.get_target_name` và public execution-target override cho `spawn_window` / `spawn_session` / `split` cũng đã bị cắt.
- `apps/chatminal-desktop/src/desktop_commands.rs`
  - compatibility translation layer cho upstream `KeyAssignment` names.
  - active `SpawnSession` đã thành single-flow action không còn payload target.

## Architectural ownership
- Product source of truth: `chatminal-runtime` + `chatminal-session-runtime`.
- Desktop source of truth trong app layer: `apps/chatminal-desktop/src/chatminal_runtime/*`.
- Render/input shell: `termwindow/*`.
- Engine/private host zone: `desktop_host_runtime/*` + `crates/chatminal-host-runtime/*`.

## Verification snapshot
- `cargo check --workspace`: pass
- `cargo check --workspace --all-targets`: pass
- `cargo test -p chatminal-runtime -- --test-threads=1`: pass
- `cargo test -p chatminal-session-runtime -- --test-threads=1`: pass
- `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminald/Cargo.toml -- --test-threads=1`: pass

## High-signal improvements (Phase 2 summary)
- **Phase 2.1**: 17 Runtime* types unified as chatminal-protocol type aliases; `api/protocol.rs` (431 LOC) deleted; 5 Store→Protocol From impls moved to chatminal-store. Net: -431 LOC conversion boilerplate, zero redundant types at runtime boundary.
- **Phase 2.2-2.3**: Engine split fallback path fully sealed; `split_terminal_handle*` functions deleted from desktop_host_runtime; 4 dead functions removed; ~33 LOC cleaned; type aliases consolidation (HostSplitSource, HostRuntimeEntryId, HostLayoutNode, HostSplitDirection removed).
- **Phase 1 completions**: `OverlayRenderScope` boundary fully isolated; session vocabulary unified; 4 SSH/tmux/remote crates deleted; chatminal-mux binary deleted; ~3GB WezTerm reference snapshot deleted.
- **Phase 2.4**: Workspace layout persistence confirmed via `set_string_state`/`get_string_state` key-value store with auto-save to app_state table.

## Remaining engineering debt (intentional, post-Phase 2)
- **Tab split functions**: `tab.rs` split_and_insert/compute_split_size cannot be removed (lua-bridge calls Mux::split_pane → SpawnTarget::split_pane → tab). Desktop-only uses WorkspaceLayoutState; daemon/lua still need engine split support.
- **Engine split at daemon level**: Mux/Tab/Pane still fully present in `chatminal-host-runtime` (intentional lower private layer); daemon may use split via lua-bridge.
- **Command/config compatibility**: `desktop_commands.rs` still translates upstream `KeyAssignment::*Tab*` for config backward-compat (not exercised by desktop product, only preserved for old configs).
- **Lower parser duplication**: `chatminal-terminal-core` (vt100-based) vs `chatminal-engine-term` (termwiz-based) — both in codebase; daemon uses core, desktop uses engine-term.
