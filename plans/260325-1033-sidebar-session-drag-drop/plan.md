---
title: "Sidebar Session Drag Drop"
description: "Short plan for drag-drop reorder and cross-profile move in the desktop sidebar tree."
status: completed
priority: P2
effort: 3h
branch: main
tags: [chatminal, desktop, sidebar, session, drag-drop]
created: 2026-03-25
---

# Plan

## Files to edit
- `crates/chatminal-store/src/lib.rs`
- `crates/chatminal-store/tests/store-workspace.rs`
- `crates/chatminal-runtime/src/state.rs`
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-runtime/src/state/tests.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`

## Reuse first
- Reuse single-session move path where possible, nhưng thêm bulk `move_sessions_to_profile(..., target_index)` ở store/runtime để reorder cả cụm theo một transaction.
- Reuse `DesktopWorkspaceLayoutStore::profile_group_session_ids(...)` to expand a dragged joined member into the full cluster.
- Reuse existing sidebar row hit targets (`ChatminalSidebarProfile`, `ChatminalSidebarSession`) and existing mouse drag pattern already used by sidebar resize.
- Reuse profile/group layout persistence helpers (`save_as_profile_layout`, `restore_profile_layout_if_contains`, `clear_profile_group_layouts`, `replace_layout`) instead of inventing a second join-state store.

## TODO / Order
1. Done: add sidebar-local drag state in `chatminal_sidebar/mod.rs`: drag source session ids, anchor session, current drop target (`append-to-profile` vs `insert-before-session`), and helpers to clear/apply preview state.
2. Done: extend sidebar render in `termwindow/render/chatminal_sidebar.rs` to show drag affordance only inside the sidebar tree: highlight dragged rows, profile-row append target, and between-row insertion indicator.
3. Done: extend `desktop_termwindow_mouseevent.rs` to start drag from session rows after left-press movement threshold, update drop target while hovering rows/profiles, and commit on release; right-click/context-menu flow giữ nguyên.
4. Done: add thin `TermWindow` drop orchestrator in `termwindow/mod.rs` that resolves dragged ids:
   `single session` => one id.
   `joined member` => expand via `profile_group_session_ids` and keep sidebar order.
5. Done: in the drop orchestrator, translate UI target into one bulk backend move:
   drop on profile row => target profile + `None`/append.
   drop between rows => target profile + insert index before hovered session.
   same-profile reorder => compute adjusted indices after removing dragged ids first, avoid off-by-one drift.
6. Done: update layout/profile-group bookkeeping after cluster move: clear stale source profile-group aliases and persist target group alias when the dragged cluster stays joined.

## Notes
- Keep MVP narrow: no generic drag-drop framework, no multi-select drag semantics beyond “dragging a joined session moves its whole joined cluster”.
- Prefer cluster order from current sidebar/render order, not raw layout view order, so visual order and persisted order stay aligned.
- Do not touch `crates/chatminal-terminal-core`; all work stays in desktop shell/sidebar/runtime-layout plumbing.

## Validation
- `cargo check -p chatminal-desktop`
- `cargo test --manifest-path crates/chatminal-store/Cargo.toml move_sessions_to_profile -- --nocapture`
- `cargo test -p chatminal-runtime sessions_move_to_profile_reorders_group_without_losing_runtime_state -- --nocapture`
- Manual smoke:
  - reorder inside same profile
  - move to another profile by dropping on profile row
  - move to another profile by dropping between session rows
  - drag joined group and verify markers/unjoin behavior still correct
  - verify active session/focus does not jump unexpectedly

## Main risks
- Sequential cluster moves can drift target index after each `move_session_to_profile` call; same-profile reorder is the easiest place to get off-by-one and inverted order bugs.
- Joined-group layout aliases can become stale in source or target profile if move updates store order but not profile-group layout persistence, causing wrong sidebar join markers or broken unjoin later.
- Drag hit-test can diverge from rendered insertion indicator when tree is scrolled or when hovering profile rows vs session gaps, producing “drop looked valid but landed elsewhere”.

## Unresolved Questions
- When dropping a joined cluster into another profile, should the target profile immediately inherit a persisted joined layout alias for that cluster even if that profile is currently inactive, or only after the user first activates one of those sessions?
