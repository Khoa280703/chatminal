## Phase 04 Batch 01

Status: in_progress

### Scope shipped
- Refactor product-facing helper names trong `termwindow` shell từ vocabulary `tab` sang `render target`.
- Nhóm rename đã hoàn tất:
  - `tab_overlay` -> `render_target_overlay`
  - `active_tab_overlay` -> `active_render_target_overlay`
  - `active_terminal_instance_from_active_tab` -> `active_terminal_instance_from_active_render_target`
  - `active_tab_contains_pane` -> `active_render_target_contains_terminal`
  - `activate_pane_index_in_active_tab` -> `activate_terminal_index_in_active_render_target`
  - `activate_pane_direction_in_active_tab` -> `activate_terminal_direction_in_active_render_target`
  - `active_tab_splits` -> `active_render_target_splits`
  - `active_tab_positioned_panes` -> `active_render_target_positioned_panes`

### Files changed
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_positioned_session_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_impl.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_state_helpers.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_overlay_helpers.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`

### Gates
- `cargo check -p chatminal-desktop`: pass
- `rg -n "active_tab_overlay|active_terminal_instance_from_active_tab|activate_pane_index_in_active_tab|activate_pane_direction_in_active_tab|active_tab_splits|active_tab_positioned_panes|tab_overlay\\(" apps/chatminal-desktop/src/termwindow apps/chatminal-desktop/src/desktop_termwindow_* apps/chatminal-desktop/src/tabbar.rs`: zero matches

### Remaining in Phase 04
- `TabBarItem` / `TabBarState` chưa đổi tên.
- `SessionEntryInformation.render_scope_id` vẫn là tên cũ.
- Action/product names kiểu `ActivateTab*`, `MoveTab*`, `CloseTab` ở layer UI/translation vẫn chưa dọn xong.
