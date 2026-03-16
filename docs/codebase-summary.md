# Codebase Summary

Last updated: 2026-03-13

## Runtime baseline
Chatminal hiện chia làm ba lớp rõ ràng:
- `apps/chatminal-desktop` — desktop app first-party, render/input shell + desktop facade + private engine adapter.
- `crates/chatminal-runtime` — app/runtime orchestrator, persistence facade, native API, workspace/session state.
- `crates/chatminal-session-runtime` — execution subsystem, workspace layout model, session engine, runtime registry.

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
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - private session host: builds `ChatminalRenderState` trực tiếp từ `session_pane` map; `HostRenderScope` chỉ còn cho overlay compat.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
  - terminal pane bridge cho output/input/runtime metadata.
- `apps/chatminal-desktop/src/desktop_host_runtime/pane.rs`
  - runtime pane wrapper.
- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs`
  - desktop-side bridge sang execution/runtime core.

### Runtime and execution core
- `crates/chatminal-runtime/src/state/native_api.rs`
  - desktop/native API cho workspace/session lifecycle.
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
  - mapping giữa persisted/runtime-facing state.
- `crates/chatminal-runtime/src/api/mod.rs`
  - app-facing boundary ids/snapshots.
- `crates/chatminal-session-runtime/src/session_engine.rs`
  - facade execution engine.
- `crates/chatminal-session-runtime/src/session_engine_core.rs`
  - native execution mutations/focus/close/spawn core logic.
- `crates/chatminal-session-runtime/src/workspace_layout.rs`
  - public layout model cho session view/group tree.
- `crates/chatminal-session-runtime/src/workspace_layout_registry.rs`
  - registry/source of truth cho layout snapshots theo workspace.
- `crates/chatminal-session-runtime/src/session_runtime_state.rs`
  - runtime state snapshots và render-target tracking.
- `crates/chatminal-session-runtime/src/leaf_runtime_registry.rs`
  - live terminal runtime registry.

### Lower engine/private compatibility
- `crates/chatminal-host-runtime/*`
  - lower engine host internals; được phép giữ `Mux/Tab/Pane`.
- `crates/chatminal-lua-bridge/src/lib.rs`
  - Lua/config bridge theo vocabulary Chatminal.
- `apps/chatminal-desktop/src/desktop_commands.rs`
  - compatibility translation layer cho upstream `KeyAssignment` names.

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

## Current risk
- Engine-side lower layer vẫn giữ host primitives; đó là intentional private debt, không còn leak ra product-facing desktop path.
- Command/config compatibility vẫn phải duy trì translation layer cho upstream-style key assignments.
