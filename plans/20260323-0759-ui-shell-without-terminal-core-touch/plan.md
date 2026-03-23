---
title: "Chatminal UI Shell Without Terminal Core Touch"
description: "Roadmap to polish desktop shell UI around the terminal while treating terminal core as a black box."
status: pending
priority: P2
effort: 9d
branch: main
tags: [planning, desktop, ui-shell, terminal-black-box, rust]
created: 2026-03-23
---

# Plan

Scope: chỉ UI shell layer quanh terminal trong `apps/chatminal-desktop`. Mục tiêu: sidebar, session bar, footer, overlay, layout primitives, scroll-tree-list, motion/icon polish nhẹ. Terminal core giữ nguyên black box.

Hard boundary, áp dụng cho mọi phase:
- Không sửa `crates/chatminal-terminal-core/**`.
- Không đổi parser/state/scrollback/input contract của terminal.
- Không đổi session engine/store/runtime schema để phục vụ UI polish.
- Không route UI qua private host primitives mới.

## Phases
1. [Phase 01 - Boundary Freeze And Shell Surface Inventory](./phase-01-boundary-freeze-and-shell-surface-inventory.md) - pending - 0.5d
2. [Phase 02 - Layout Primitives And Shell Spacing Tokens](./phase-02-layout-primitives-and-shell-spacing-tokens.md) - pending - 1.5d
3. [Phase 03 - Sidebar And Scroll Tree List Polish](./phase-03-sidebar-and-scroll-tree-list-polish.md) - pending - 2d
4. [Phase 04 - Session Bar And Footer Information Density](./phase-04-session-bar-and-footer-information-density.md) - pending - 1.5d
5. [Phase 05 - Overlay Shell Cohesion And Focus Behavior](./phase-05-overlay-shell-cohesion-and-focus-behavior.md) - pending - 1.5d
6. [Phase 06 - Lightweight Icon Animation And Motion Budget](./phase-06-lightweight-icon-animation-and-motion-budget.md) - pending - 1d
7. [Phase 07 - Verification, Non-Core Regression Gates, And Docs Sync](./phase-07-verification-non-core-regression-gates-and-docs-sync.md) - pending - 1d

## Key Dependencies
- Existing desktop shell seams: `termwindow/*`, `desktop_termwindow_*`, `tabbar.rs`, `chatminal_sidebar/mod.rs`, `overlay/*`, `chatminal_layout/*`, `chatminal_render/*`.
- Existing runtime snapshots only: ordered session/profile/workspace/render-target data already exposed to desktop.
- Existing animation scheduler: `has_animation` / render invalidation path.

## Delivery Shape
- Prefer direct updates to existing desktop files.
- New files: avoid by default. Only allow if an existing shell file must be split to stay maintainable.
- Validation focus: visual correctness, hit-test correctness, focus correctness, zero terminal behavior drift.

## Exit Conditions
- Shell chrome feels more cohesive and less cramped.
- Sidebar/session bar/footer/overlay share one visual and spacing system.
- Scroll-tree-list handles dense states cleanly.
- Motion stays subtle and cheap.
- Terminal content rendering, input semantics, parser behavior, and core state remain unchanged.
