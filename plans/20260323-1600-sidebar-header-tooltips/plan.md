---
title: "Add tooltips for sidebar header actions"
description: "Very small plan for showing tooltips on the sidebar header settings and plus buttons."
status: pending
priority: P3
effort: 30m
branch: main
tags: [chatminal, desktop, sidebar, tooltip]
created: 2026-03-23
---

# Plan

## Files to edit
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`

## Short approach
1. Add a dedicated `UIItemType` for the header `settings` button because it currently has no hover target; keep `plus` using the existing create-profile item.
2. In `chatminal_sidebar.rs`, attach the new item type to the settings icon and render a small hover tooltip label near the header buttons when `last_ui_item/current_mouse_event` points to `settings` or `plus`.
3. In `desktop_termwindow_mouseevent.rs`, handle the new settings item as hover-only / no-op click for now, keep cursor behavior consistent, and rely on existing invalidate-on-hover flow.
4. Validate manually in the desktop app: hover `settings` shows `Settings`, hover `plus` shows `New profile`, tooltip disappears on leave, no click regression.

## Notes
- Keep scope tight: tooltip only for the 2 sidebar header buttons, no generic tooltip system.
- If settings action is not implemented yet, do not add behavior beyond hover state.
