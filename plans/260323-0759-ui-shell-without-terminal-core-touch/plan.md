---
title: "Chatminal UI shell without terminal core touch"
description: "Polish desktop UI shell (sidebar, session bar, footer, overlay) keeping terminal core as black box."
status: done
priority: P1
effort: 6d
branch: main
tags: [desktop, ui-shell, rust, layout, sidebar]
created: 2026-03-23
---

# Plan Overview

## Context
- Desktop (`chatminal-desktop`) = PRIMARY and ONLY frontend
- Daemon/CLI removed from workspace; all UI work targets desktop
- Recent cleanup: connui.rs, update.rs deleted; engine split fallback removed; 0 warnings

## Scope
- Polish `apps/chatminal-desktop` shell layer: sidebar, session bar, footer, overlay, layout primitives
- Terminal core = black box; no parser/state/PTY/runtime/store changes

## Hard Boundaries
- No-touch: `crates/chatminal-terminal-core/**`
- No-touch: runtime/store/PTY internals (read-only consumption only)
- No protocol/state ownership changes
- App-side state ownership guard:
  - `apps/chatminal-desktop/src/chatminal_runtime/**` = read-only consumption only
  - `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs` = read-only geometry consumer only
  - Any mutation to ownership, persistence semantics, or runtime state contracts fails the phase

## Phase Map
1. `done` [Phase 01 - Layout Primitives and Chrome Geometry](./phase-01-layout-primitives-and-chrome-geometry.md) (1.5d)
2. `done` [Phase 02 - Sidebar and Scroll Tree List Rebuild](./phase-02-sidebar-and-scroll-tree-list-rebuild.md) (2.5d)
3. `done` [Phase 03 - Session Bar and Footer Polish](./phase-03-session-bar-and-footer-polish.md) (1d)
4. `done` [Phase 04 - Overlay Shell and Interaction Routing](./phase-04-overlay-shell-and-interaction-routing.md) (1d)

## Key Files
- Shell ownership: `apps/chatminal-desktop/src/termwindow/*`
- Sidebar: `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`, `termwindow/render/chatminal_sidebar.rs`
- Session bar: `apps/chatminal-desktop/src/tabbar.rs`
- Layout: `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs` (READ-ONLY geometry consumer — do NOT mutate WorkspaceLayoutState ownership)
- Split render: `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- Overlay: `apps/chatminal-desktop/src/overlay/mod.rs`

## Exit Criteria (every phase)
- `cargo check -p chatminal-desktop` passes
- Boundary gate: `git diff --name-only | grep -E "^crates/chatminal-(terminal-core|runtime|store|protocol)/" → must be empty`
- Boundary gate: `git diff --name-only | grep -E "^apps/chatminal-desktop/src/chatminal_runtime/" → must be empty`
- Boundary gate: if `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs` changes, diff must be geometry-read-only and must not mutate `WorkspaceLayoutState` ownership/persistence semantics
- No PTY internals changed
- Manual smoke: sidebar on/off, session bar top/bottom, overlay spawn/cancel, resize
