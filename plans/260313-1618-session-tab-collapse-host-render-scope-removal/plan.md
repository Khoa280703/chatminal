---
title: "Session = Tab: Remove HostRenderScope, collapse dual state, clean vocabulary"
description: "Eliminate HostRenderScope (Tab wrapper); session owns pane directly; merge parallel state; delete chatminal-session-runtime; rename Tab/Pane vocabulary to Session."
status: completed
priority: P1
effort: 7d
progress: "100%"
branch: main
tags: [refactor, session-runtime, host-render-scope, architecture, vocabulary]
created: 2026-03-13
updated: 2026-03-16
---

# Session = Tab: Full Cutover Plan

## Goal

1. Xóa `HostRenderScope` (Tab wrapper) — thay `session_id → HostRenderScope → pane` bằng `session_id → pane` trực tiếp.
2. Merge dual state machine về `DaemonState`.
3. DELETE `crates/chatminal-session-runtime/` hoàn toàn.
4. Rename toàn bộ Tab/Pane vocabulary trong config API → Session.

## Architecture invariants (bất biến — không được vi phạm)

1. **1 session = 1 terminal unit** — split screen = nhiều session views, không phải split nội bộ session
2. **Desktop là executor duy nhất** — `chatminal-runtime` và `chatminald` chỉ giữ metadata/persistence; không execute sessions
3. **Layer boundary** — `chatminal-runtime` không chứa desktop types (`Arc<ChatminalSessionPane>`, v.v.)
4. **Legacy flows** (`split_terminal_instance`, `focus_direction`, `swap_active_with_terminal_instance`, `move_terminal_instance_*`) phải được migrate/xóa trong Phase 02-06

## Product constraints

- Sessions có thể background (không có layout slot)
- Close session trong split → layout co lại, không có placeholder
- `HostMux`/`Pane` giữ nguyên làm engine ẩn — chỉ `desktop_host_runtime` biết

## Architecture delta

```
Before:
  session_id → HostMux.get_window → iter HostRenderScope(Tab) → ChatminalSessionPane
  KeyAssignment::ActivateTab(n), SpawnTab, CloseCurrentPane, ...

After:
  session_id → DesktopSessionHost.session_pane[session_id] → ChatminalSessionPane
  KeyAssignment::ActivateSession(n), SpawnSession, CloseCurrentSession, ...
```

## Phases

| # | File | Title | Status | Gate |
|---|------|-------|--------|------|
| 01 | [phase-01](phase-01-inventory-boundary-freeze.md) | Inventory + Boundary Freeze | completed | grep audit pass ✓ |
| 02 | [phase-02](phase-02-direct-pane-ownership.md) | Direct Pane Ownership | completed | `cargo check` ✓ |
| 03 | [phase-03-render-entry-cutover.md](phase-03-render-entry-cutover.md) | Render Entry Cutover | completed | `cargo check` + render smoke ✓ |
| 04 | [phase-04-background-session.md](phase-04-background-session.md) | Background Session Support | completed | `cargo test` ✓ |
| 05 | [phase-05-merge-parallel-state.md](phase-05-merge-parallel-state.md) | Merge Parallel State | completed | `cargo test` all crates ✓ |
| 06 | [phase-06-dead-code-deletion-verification.md](phase-06-dead-code-deletion-verification.md) | Dead Code Deletion + Verification | completed | full build + test suite ✓ |
| 07 | [phase-07-single-core-boundary-final.md](phase-07-single-core-boundary-final.md) | Single Core Boundary — đảo dependency *(transitional: layout types move, session-runtime còn tồn tại)* | completed | `grep 0` + full test suite ✓ |
| 08 | [phase-08-final-cleanup-single-dependency-chain.md](phase-08-final-cleanup-single-dependency-chain.md) | Final Cleanup — DELETE chatminal-session-runtime *(final: execution code → desktop_host_runtime, crate bị xóa)* | completed | grep gates 0 + full test suite ✓ |
| 09 | [phase-09-config-vocabulary-session-rename.md](phase-09-config-vocabulary-session-rename.md) | Config Vocabulary: Tab/Pane → Session *(compatibility rename: logic risk thấp, churn blast radius cao — breaking change cho user config/Lua API)* | completed | grep gate 0 + `cargo test` ✓ |

## Key dependencies

- Phase 02 → Phase 03 (pane lookup must exist before render cutover)
- Phase 04 unblocks Phase 05 (background session model needed before state merge)
- Phase 06 chỉ làm sau khi 03+04+05 đều pass gate
- Phase 07 chỉ làm sau Phase 06 (code phải clean trước khi cắt dependency)
- Phase 08 chỉ làm sau Phase 07 (dependency chain phải đúng trước khi xóa dead code)
- Phase 09 chỉ làm sau Phase 08 (structural cleanup xong trước khi rename vocabulary)

## Done when (true "1 core", clean vocabulary)

- `chatminal-runtime/Cargo.toml` không còn depend `chatminal-session-runtime`
- Developer thêm feature mới chỉ cần đọc `crates/chatminal-runtime/`
- `crates/chatminal-session-runtime/` **bị xóa hoàn toàn**
- Code execution engine được move vào `desktop_host_runtime`
- Không còn `SpawnTab`, `ActivateTab`, `CloseCurrentTab/Pane`, `SplitPane`, `PaneDirection`, `tab_bar`, `LeafRef`, `HandySplitDirection` trong chatminal code
- User Lua config dùng `SpawnSession`, `ActivateSession`, `CloseCurrentSession`, `TerminalRef`
- Không còn file dead trong bất kỳ crate nào
- `cargo check --workspace --all-targets` pass

## Files NOT touched

- `third_party/` — hard constraint
- GPU render path: `paint_pane`, font shaping, cell drawing
- PTY engine internals
