# Forbidden Symbols Contract

Purpose: khóa rõ symbol nào bị cấm theo scope và theo phase, để tránh refactor nửa vời hoặc grep false positive.

## Allowed Legacy Zones
- `apps/chatminal-desktop/src/desktop_host_runtime/*`
- `crates/chatminal-host-runtime/src/*`
- `crates/chatminal-session-runtime/src/*` cho engine-private meanings
- `tests`, `docs`, `plans`

## Product/Public Zones
- `apps/chatminal-desktop/src/chatminal_runtime/*`
- `apps/chatminal-desktop/src/termwindow/*`
- `apps/chatminal-desktop/src/desktop_termwindow_*`
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/desktop_commands.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `crates/chatminal-lua-bridge/src/*`

## Phase 03 Forbidden In Product/Public Zones
- `host_active_render_scope`
- `pane_metadata_runtime_id`
- `pane_metadata_terminal_instance_id`
- `active_tab_overlay`
- `active_terminal_instance_from_active_tab`
- `activate_pane_index_in_active_tab`
- `activate_pane_direction_in_active_tab`
- `active_tab_splits`
- `active_tab_positioned_panes`

## Phase 04 Forbidden In Product/Public Zones
- `CloseTab`
- `ActivateTab`
- `ActivateTabRelative`
- `ActivateTabRelativeNoWrap`
- `ActivateLastTab`
- `MoveTab`
- `MoveTabRelative`
- `TabBarItem`
- `TabBarState`
- `UIItemType::CloseTab`
- `active_tab_overlay`
- product-facing methods with names containing `_tab_` or `_pane_`

## Phase 05 Forbidden In Public Lua Surface
- `get_host_tab`
- `get_host_leaf`
- public APIs exposing raw `tab_id()` or `pane_id()` as product identifiers

## Notes About False Positives
- Theme/config names like `active_tab`, `inactive_tab`, `tab_bar` are allowed inside pure rendering/theme code until explicit render cleanup phase, as long as they do not encode product semantics.
- `OverlayPane`, `OverlayRenderScope`, and host primitive names are allowed only in render/adapter compatibility helpers, not in app-facing mutation/query APIs.
- `desktop_commands.rs` may keep upstream command names only behind translation layer if removing them would break config compatibility; product-facing action routing must not depend on them directly after Phase 04.
- Graphics/library types like `wgpu::Surface` or `glium::Surface` are not part of this contract and must be excluded from `surface` product-leak grep review.
- Enum values or theme labels like `CloseReason::Pane`, `Window -> Select Pane`, or `active_tab` in color/theme-only code are not automatic violations; only product semantics and routing names count.
