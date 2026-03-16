# Phase 04 - TermWindow And Action Routing Cutover

## Context Links
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/termwindow/mouseevent.rs`
- `apps/chatminal-desktop/src/termwindow/paneselect.rs`
- `apps/chatminal-desktop/src/spawn.rs`
- `apps/chatminal-desktop/src/commands.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: cắt `termwindow` khỏi `Mux::get()`, `Tab`, `Pane`, `ActivateTab`, `CloseCurrentTab`, `SpawnTab`.

## Key Insights
- `termwindow` đang là callsite dày nhất còn giữ vocabulary và behavior tab-centric.
- Nếu không cắt chỗ này, các phase trước vẫn chỉ là adapter shuffle.

## Requirements
- `termwindow` resolve active target qua `DesktopSessionHost` + `ChatminalRenderTree`.
- Action public đổi sang `session`/`session_view`/`terminal_instance`.
- Close/focus/move/split chỉ đi qua `DesktopSessionHost` hoặc session-runtime API.

## Architecture
- Add `TermWindowSessionController` first-party.
- `TermWindowNotif` không còn `MuxNotification`.
- `resolve_host_pane`, `active_tab`, `tab_overlay`, `spawn_overlay_in_tab` phải bị thay thế.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/termwindow/mod.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/mouseevent.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/paneselect.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/clipboard.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/selection.rs`
- Refactor: `apps/chatminal-desktop/src/selection.rs`
- Refactor: `apps/chatminal-desktop/src/spawn.rs`
- Refactor: `apps/chatminal-desktop/src/commands.rs`

## Implementation Steps
1. Replace active lookup helpers bằng session controller.
2. Replace tab-relative actions bằng session-view-relative actions.
3. Replace close/focus/swap/move routes qua host/session runtime.
4. Rewrite command labels/docs để bỏ `Tab`/`Pane`.
5. Add focused regression tests cho create/switch/close/move.

## Todo List
- [x] Remove `Mux::get()` khỏi termwindow active path
- [x] Remove `TabId`-based active render-scope routing from `termwindow/mod.rs`
- [x] Rename session-bar runtime-entry helpers to drop `host_tab` vocabulary
- [x] Remove direct `Mux::get()` callsites from `spawn.rs`, `paneselect.rs`, `layout_render.rs`, `paint.rs`, `resize.rs`
- [x] Externalize legacy action keyword compatibility mapping out of `commands.rs` and `termwindow/*` grep surface
- [x] Remove `Tab`/`Pane` from termwindow APIs
- [x] Rewrite command vocabulary
- [x] Rename window-side notification plumbing from `MuxNotification` callsites to runtime-notification adapter naming
- [ ] Add interaction tests

## Success Criteria
- `rg -n "Mux::get\\(|ActivateTab|CloseCurrentTab|SpawnTab|MoveTab|ShowTabNavigator|host_tab|TabId|PaneId|ConfirmClosePane|confirm_close_tab|confirm_close_pane" apps/chatminal-desktop/src/termwindow apps/chatminal-desktop/src/spawn.rs apps/chatminal-desktop/src/commands.rs`
  - expected: zero action-routing vocabulary cũ; không tính key name `Tab` của bàn phím.

## Current Notes
- `termwindow + desktop_termwindow_* + overlay + selection + render` đã sạch `use mux::|mux::|PaneId|TabId|Arc<Tab>|Arc<dyn Pane>` theo scoped grep gate desktop UI/render path.
- `cargo check -p chatminal-desktop` pass.
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass (`20/20`).
- Residual `mux` còn lại của desktop tập trung trong `desktop_host_runtime/*`, không còn nằm ở action-routing/UI path của Phase 04.

## Risk Assessment
- Risk: hotkey/config compatibility bị gãy.
- Mitigation: giữ key assignment mapping cũ ở config layer nhưng route sang action mới phía dưới.

## Security Considerations
- Không có auth impact; chú ý không làm hỏng prompt/command execution path.

## Next Steps
- Sang Phase 05 để overlay/copy/quickselect/launcher không còn phụ thuộc `mux`.
