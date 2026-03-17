# System Architecture

Last updated: 2026-03-17

## Latest changes (Phase 2, 2026-03-17)
- **Phase 2.1 - Type alias consolidation**: 17 Runtime* types (RuntimeSession, RuntimeProfile, etc.) are now **direct type aliases** to `chatminal-protocol` types. Deleted `api/protocol.rs` (431 LOC) conversion boilerplate; moved 5 Store→Protocol From impls to `chatminal-store`.
- **Phase 2.2 - Documentation**: Added doc comments to `chatminal-terminal-core` and `chatminal-engine-term`.
- **Phase 1 completions** (recent): OverlayRenderScope isolated; session/terminal vocabulary unified; chatminal-mux deleted.

## Topology

### Desktop app
```text
chatminal-desktop
  -> chatminal_runtime facade
    -> chatminal-runtime
      -> chatminal-session-runtime
        -> workspace_layout + session engine + terminal runtime registry
    -> desktop_host_runtime (private engine adapter only)
      -> chatminal-host-runtime
        -> Mux/Tab/Pane private engine primitives
  -> termwindow render/input shell
  -> chatminal_sidebar + session bar UI
```

### Persistence and compatibility
```text
chatminal-runtime
  -> chatminal-store (SQLite)
  -> profiles / sessions / scrollback / workspace layout state
  -> native_api + runtime_bridge

chatminald / chatminal-app
  -> compatibility boundary only
  -> reuse protocol/store/runtime contracts
```

## Architecture rules
- Product model: `session -> session_view -> session_group -> workspace_layout -> render_target -> terminal_instance`.
- `apps/chatminal-desktop/src/chatminal_runtime/*` là desktop facade duy nhất cho product state/query/action.
- `apps/chatminal-desktop/src/termwindow/*` và `desktop_termwindow_*` chỉ là render/input shell; không phải source of truth cho business routing.
- `apps/chatminal-desktop/src/desktop_host_runtime/*` là private adapter duy nhất còn chạm host primitives.
- `crates/chatminal-host-runtime/*` được phép giữ `Mux/Tab/Pane`, nhưng chỉ như engine implementation detail.
- `apps/chatminal-desktop/src/desktop_commands.rs` là compatibility translation layer cho `KeyAssignment::*Tab*`; product-facing code không route trực tiếp các symbol đó.

## Runtime flow

### 1. Product state
- `chatminal-runtime` giữ profile/session persistence, workspace snapshot, native API và desktop-facing runtime bridge.
- `chatminal-session-runtime` giữ live execution model: session engine, runtime registry, focus manager, workspace layout registry.
- `workspace_layout` là public execution/layout model cho app layer; không expose host split tree ra desktop product path.

### 2. Desktop facade
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` expose desktop bindings như session/view/render-target/terminal handle/window snapshot.
- `client.rs` và facade helpers resolve active session, ordered session entries, focus/close/swap routing qua runtime boundary.
- Desktop shell không còn tự ghép business state từ host tab/pane metadata như source of truth.

### 3. Render/input shell
- `termwindow/*` render terminal, overlay, selection, launcher, mouse/key events.
- `tabbar.rs` đã trở thành session bar state/product UI model.
- `chatminal_layout/*` và `layout_render.rs` map workspace layout sang geometry render thực tế.

### 4. Private engine adapter
- `desktop_host_runtime/*` bridge từ facade/runtime sang engine host thực tế.
- Adapter này giữ host window/session pane/runtime pane/domain internals.
- Host vocabulary bị thu xuống `pub(crate)` hoặc private trong desktop app path.
- Render path: `WorkspaceLayout → session_id → DesktopSessionHost.pane(session_id) → GPU draw`
  (không còn đi qua `HostRenderScope` để build pane list).
- `HostRenderScope` fully removed; `OverlayRenderScope` isolated from overlay boundary. Session owns pane directly via `session_id → Arc<ChatminalSessionPane>` lookup.

### 5. Lua/config boundary
- `crates/chatminal-lua-bridge/*` expose Chatminal-facing session/window/terminal queries.
- Public APIs `get_host_tab` và `get_host_leaf` đã bị xóa.
- Public Lua surface dùng `terminal`/`terminal_instance_id` thay cho host-tab/host-leaf ids.

## Type alias consolidation (Phase 2.1)
17 Runtime boundary types (`RuntimeSessionStatus`, `RuntimeProfile`, `RuntimeSession`, `RuntimeWorkspace`, `RuntimeCreatedSession`, `RuntimeLifecyclePreferences`, `RuntimeSessionSnapshot`, `RuntimeSessionExplorerState`, `RuntimeSessionExplorerEntry`, `RuntimeSessionExplorerFileContent`, `RuntimePtyOutputEvent`, `RuntimePtyExitedEvent`, `RuntimePtyErrorEvent`, `RuntimeSessionUpdatedEvent`, `RuntimeWorkspaceUpdatedEvent`, `RuntimeDaemonHealthEvent`, `RuntimeEvent`) are now **direct type aliases** in `crates/chatminal-runtime/src/api/mod.rs` to their `chatminal-protocol` counterparts.

**Previous structure (pre-Phase 2.1):**
- Store layer: `StoredSession`, `StoredProfile` (SQLite-specific fields)
- Protocol layer: `SessionInfo`, `ProfileInfo` (network protocol)
- Runtime layer: `RuntimeSession`, `RuntimeProfile` (redundant duplicates)
- Conversion: `api/protocol.rs` (431 LOC) with `From` impls

**Current structure (post-Phase 2.1):**
- Store layer: `StoredSession`, `StoredProfile` (SQLite-specific, unchanged)
- Shared: `chatminal-protocol` types (used directly by Runtime via aliases)
- Conversion: `From` impls for Store→Protocol moved to `chatminal-store` crate
- **Benefit**: No more type redundancy at runtime boundary; desktop/daemon both use protocol types directly.

## Verification freeze
- `cargo check --workspace`: pass
- `cargo check --workspace --all-targets`: pass
- `cargo test -p chatminal-runtime -- --test-threads=1`: pass
- `cargo test -p chatminal-session-runtime -- --test-threads=1`: pass
- `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminald/Cargo.toml -- --test-threads=1`: pass

## Remaining intentional compatibility
- Engine internals vẫn có `Mux/Tab/Pane` trong `chatminal-host-runtime` và private adapter desktop.
- Command/config compatibility vẫn giữ upstream `KeyAssignment::*Tab*` translation trong `desktop_commands.rs` để không gãy config cũ.
- `OverlayRenderScope` dùng cho launcher/confirm/prompt overlays nhưng không còn coupled với render scope; boundary fully isolated.
- `SessionExecutionStatus` enum thêm vào `chatminal-runtime/state.rs` để track running status.
- Các phần trên là intentional private/compatibility zones, không còn là product-facing architecture.
