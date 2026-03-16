# Phase 01 Current Leakage Inventory

Status: complete
Date: 2026-03-13

## Summary
Leakage inventory đủ chặt để bắt đầu implementation. Không còn vùng xám lớn ở planning level.

## Product/Public Zone Findings

### `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- Vẫn còn business-facing helpers mang semantics cũ:
  - `active_tab_overlay`
  - `active_terminal_instance_from_active_tab`
  - `activate_pane_index_in_active_tab`
  - `activate_pane_direction_in_active_tab`
  - `active_tab_splits`
  - `active_tab_positioned_panes`
- Đây là target chính của Phase 03-04.

### `apps/chatminal-desktop/src/desktop_termwindow_actions_impl.rs`
### `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- Vẫn còn action names cũ:
  - `ActivateTab*`
  - `ActivateLastTab`
  - `MoveTab*`
- Đây là target rename/cutover chính của Phase 04.

### `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Vẫn còn `UIItemType::CloseTab`.
- Cần migrate sang session-view/session-group semantics ở Phase 04.

### `apps/chatminal-desktop/src/desktop_termwindow_session_close_helpers.rs`
### `apps/chatminal-desktop/src/termwindow/mod.rs`
- Vẫn còn `pane_metadata_runtime_id`, `pane_metadata_terminal_instance_id` và close path dựa trên host/pane metadata.
- Đây là ownership leak thật, target của Phase 03.

### `apps/chatminal-desktop/src/desktop_termwindow_state_helpers.rs`
### `apps/chatminal-desktop/src/desktop_termwindow_positioned_session_helpers.rs`
- Vẫn gọi các helper `active_tab_*` ở product shell.
- Target Phase 03-04.

### `apps/chatminal-desktop/src/desktop_commands.rs`
- Còn rất nhiều upstream command names `ActivateTab*`, `MoveTab*`, `Pane` labels.
- Được giữ tạm như compatibility translation layer, nhưng sau Phase 04 không được leak vào product routing nữa.

### `apps/chatminal-desktop/src/overlay/launcher.rs`
- Còn dùng `KeyAssignment::ActivateTab*`.
- Được phép tạm ở launcher translation path, nhưng phải consume translated semantics sau Phase 04.

### `crates/chatminal-lua-bridge/src/lib.rs`
- Public Lua surface còn:
  - `get_host_tab`
  - `get_host_leaf`
  - `all_host_tabs`
  - `all_host_leaves`
- Đây là target chính của Phase 05.

## Allowed Legacy Zones Confirmed

### `apps/chatminal-desktop/src/desktop_host_runtime/*`
- Có `Mux`, `Tab`, `Pane`, `OverlayRenderScope`, `HostRenderScope`, `HostTerminal`.
- Đây là private adapter zone hợp lệ.

### `crates/chatminal-host-runtime/src/*`
- Dùng `Mux/Tab/Pane` xuyên suốt.
- Đây là lower engine library hợp lệ.

### `crates/chatminal-session-runtime/src/session_layout_tree.rs`
- Còn `Leaf` trong layout internals.
- Hợp lệ vì là execution/layout private meaning.

## False Positives Confirmed
- `wgpu::Surface`, `glium::Surface` trong graphics path: không thuộc leak contract.
- `active_tab`, `inactive_tab`, `tab_bar` trong theme/render-only code: không tự động là violation.
- `CloseReason::Pane` và label menu/theme text: không tự động là product leak nếu chỉ là compatibility UI/text.

## File Decision Outcomes
- Keep + refactor:
  - `apps/chatminal-desktop/src/termwindow/*`
  - `apps/chatminal-desktop/src/desktop_termwindow_*`
  - `apps/chatminal-desktop/src/tabbar.rs`
  - `apps/chatminal-desktop/src/desktop_commands.rs`
  - `apps/chatminal-desktop/src/overlay/launcher.rs`
  - `crates/chatminal-lua-bridge/src/*`
- Keep + privatize:
  - `apps/chatminal-desktop/src/desktop_host_runtime/*`
- Keep as allowed engine internals:
  - `crates/chatminal-host-runtime/src/*`
  - relevant parts of `crates/chatminal-session-runtime/src/*`
- Delete candidates to validate later:
  - `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`

## Gates Ready For Later Phases
- Phase 03: remove business lookup leaks from `termwindow`/`desktop_termwindow_*`
- Phase 04: remove product action names `CloseTab|ActivateTab*|MoveTab*`
- Phase 05: remove public Lua APIs `get_host_tab|get_host_leaf`
- Phase 06: shrink public host helper surface + delete dead paths
- Phase 07: full verification + docs sync + final exit checklist

## Unresolved questions
- None
