---
title: "Sidebar Session Context Menu"
description: "Short plan to add right-click session actions in the desktop sidebar with rename and delete."
status: completed
priority: P2
effort: 2h
branch: main
tags: [chatminal, desktop, sidebar, session, context-menu]
created: 2026-03-23
---

# Plan

## Files to edit
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `crates/chatminal-runtime/src/state.rs`
- `crates/chatminal-runtime/src/state/native_api.rs`

## Reuse first
- Reuse existing `UIItemType::ChatminalSidebarSession(session_id)` hit target instead of adding a second session-row control tree.
- Reuse `apps/chatminal-desktop/src/overlay/prompt.rs` for rename input and `apps/chatminal-desktop/src/overlay/confirm.rs` for delete confirmation, so scope stays inside existing overlay UX.
- Reuse `crates/chatminal-store/src/lib.rs` `rename_session` and `delete_session`; avoid new persistence model.
- Reuse sidebar snapshot refresh path already driven by runtime workspace/session events after mutation.

## Short approach
1. Extend sidebar session row interaction so right-click on `ChatminalSidebarSession` opens a tiny desktop context menu anchored to that row, with `Rename` and `Delete`.
2. Keep left-click behavior unchanged in `desktop_termwindow_mouseevent.rs`; branch only on right-click / secondary press.
3. Route `Rename` to existing prompt overlay with current session name as initial value, then commit through a thin runtime/client API that updates store and republishes sidebar/workspace state.
4. Route `Delete` to existing confirmation overlay, then call the existing close/delete path; if current runtime API only closes active runtime, add the smallest state/native API needed for explicit sidebar delete by `session_id`.

## Notes
- Prefer a sidebar-local menu state in `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`; no generic global context-menu framework.
- If rename needs no runtime side effect beyond store + publish, keep it out of session engine and host runtime.
- If delete of the active session needs focus fallback, reuse existing workspace/session refresh logic rather than bespoke sidebar selection code.
- Implemented via overlay direct callbacks (`selector_direct`, `show_line_prompt_overlay_direct`, `show_confirmation_overlay_direct`) so session menu/rename/delete stay in desktop shell layer.
- Validation hiện có: `cargo check -p chatminal-desktop`.

## Main risks
- Easy to mismatch render bounds vs right-click hit target, especially if the menu anchor is computed from row layout in one place and click handling in another.
- Delete semantics can drift: `session_close` today removes runtime + store entry, while rename has no exposed runtime API yet; plan must keep both mutations publishing consistent workspace updates.
- Overlay/menu focus can conflict with existing pane overlays or mouse capture if the sidebar menu is treated like a full overlay instead of a narrow shell interaction.

## Unresolved Questions
- Delete should behave as hard delete immediately, or mirror current close-session confirmation semantics for active running session before removing it from store?
