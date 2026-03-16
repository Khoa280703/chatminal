# End-State Manifest

Purpose: chốt file/module nào sau plan sẽ giữ vai trò gì, để Phase 06 không còn vùng xám.

## Desktop Facade
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - final role: desktop product facade duy nhất
  - keep
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
  - final role: thin runtime client wrapper
  - keep

## Desktop Product/UI Shell
- `apps/chatminal-desktop/src/termwindow/mod.rs`
  - final role: render/input shell theo vocabulary Chatminal
  - keep + refactor
- `apps/chatminal-desktop/src/tabbar.rs`
  - final role: session bar renderer/state
  - keep + rename/refactor
- `apps/chatminal-desktop/src/desktop_termwindow_actions_impl.rs`
  - final role: action routing theo `session/session_view/session_group`
  - keep + refactor
- `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
  - final role: action dispatch map theo vocabulary mới
  - keep + refactor
- `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
  - final role: UI/layout types theo naming mới
  - keep + refactor
- `apps/chatminal-desktop/src/desktop_commands.rs`
  - final role: upstream command compatibility + translation layer
  - keep + isolate compatibility semantics
- `apps/chatminal-desktop/src/overlay/launcher.rs`
  - final role: launcher UI dùng translated action names, không own tab semantics
  - keep + refactor

## Desktop Private Engine Adapter
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - final role: private engine adapter entrypoint
  - keep + privatize
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - final role: runtime/render host synchronizer, không own business state
  - keep + shrink
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
  - final role: private terminal instance host wrapper
  - keep private

## Runtime Core
- `crates/chatminal-runtime/src/*`
  - final role: app/source of truth
  - keep + refactor
- `crates/chatminal-session-runtime/src/*`
  - final role: execution subsystem
  - keep + tighten public surface

## Lua / Config Surface
- `crates/chatminal-lua-bridge/src/lib.rs`
  - final role: Chatminal-facing scripting adapter
  - keep + refactor
- `crates/chatminal-lua-bridge/src/session.rs`
  - final role: session/workspace APIs theo model mới
  - keep + cutover
- `crates/chatminal-lua-bridge/src/leaf.rs`
  - final role: delete or reduce to compatibility-only if no owner remains
  - conditional

## Delete Candidates
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
  - delete if Phase 06 proves dead
- `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
  - delete if superseded
- one-phase shims introduced in Phase 04/05
  - delete before Phase 07 closeout
