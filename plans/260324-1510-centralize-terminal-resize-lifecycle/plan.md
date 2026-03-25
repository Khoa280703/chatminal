---
title: "Centralize Terminal Resize Lifecycle"
description: "Move Chatminal desktop PTY resize into explicit lifecycle hooks and keep render/layout projection side-effect free."
status: pending
priority: P1
effort: 3h
branch: main
tags: [desktop, terminal, resize, lifecycle, architecture]
created: 2026-03-24
---

# Centralize Terminal Resize Lifecycle

## Goal
Make PTY resize happen only in lifecycle paths of Chatminal desktop session UI. Render/layout code must never mutate live terminal state again.

## Root Cause Recap
- Bug came from render-time `pane.resize(...)` in `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`.
- PTY resize during paint can trigger shell redraws (`SIGWINCH`) and make typed input appear to disappear.

## Invariants To Enforce
- Paint/layout path is pure projection only: no PTY resize, no runtime focus mutation, no shell-facing side effects.
- PTY resize authority is desktop lifecycle, not render code.
- Resize must be idempotent: if target cols/rows/pixels match current pane dims, do nothing.
- Visible joined sessions may be resized together; hidden/offline sessions must not be mutated.
- Overlay resize stays UI-only and separate from PTY resize.

## Single Resize Authority
Create one desktop helper that computes target size per visible session/view from:
- current `TerminalSize`
- active workspace layout from `DesktopWorkspaceLayoutStore`
- render-state pane mapping for each visible session

Recommended location:
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` or `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`

Helper responsibilities:
1. Read current workspace layout.
2. Map each visible session/view to target cols/rows/pixel rect.
3. Resolve active pane/runtime for that session.
4. Call resize only when dims changed.

## Exact Lifecycle Hooks
### 1. Session spawn / first attach
Files:
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`

Hook after:
- `DesktopSessionHost::ensure_runtime_inner`
- `desktop_prepare_host_layout` attach path

Requirement:
- newly attached runtime gets initial size from current window/layout target once, outside paint.

### 2. Session focus / leaf focus
Files:
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`

Hook after:
- `DesktopSessionHost::focus_runtime`
- `DesktopSessionHost::focus_terminal_instance`
- `desktop_activate_session`
- `desktop_focus_session_view_with_previous`
- `desktop_focus_session_terminal_instance`

Requirement:
- when active view changes, resize the focused session to its current layout target.
- if joined layout is visible, apply resize to all visible sessions, not only the focused one.

### 3. Workspace layout commit / join-unjoin / profile layout restore
Files:
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs`

Hook after any successful layout mutation:
- `desktop_prepare_workspace_layout`
- join/unjoin flows
- profile layout swap/restore flows

Requirement:
- after layout state changes, run a single `apply_visible_session_resizes(window_id, terminal_size)` pass.

### 4. Window resize
Files:
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/termwindow/resize.rs`

Hook after:
- `self.terminal_size = size`
- existing host/window resize propagation

Requirement:
- window resize remains the main source of truth for session PTY geometry.
- after terminal size settles, resize all visible sessions from current layout targets.

## Code Changes
- Keep `desktop_termwindow_layout_render.rs` mutation-free.
- Add one shared resize coordinator helper.
- Replace ad-hoc lifecycle resizes with calls into that helper.
- Add tests covering:
  - paint path does not resize PTY
  - joined layout resize resizes all visible sessions on window resize
  - focus/layout restore triggers lifecycle resize once

## Validation
- `cargo check -p chatminal-desktop -p chatminal-runtime`
- `cargo test -p chatminal-desktop`
- manual: split/join, switch session, switch profile, resize window, type in zsh without prompt redraw corruption

## Open Questions
- Whether hidden profile layouts should persist last known target size or recompute only on activation.
