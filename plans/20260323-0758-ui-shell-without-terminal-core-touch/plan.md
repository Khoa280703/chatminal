---
title: "Chatminal UI Shell Without Terminal Core Touch"
description: "Roadmap nâng cấp shell UI desktop quanh terminal mà giữ terminal core và execution core như black box."
status: pending
priority: P2
effort: "8-10d"
branch: main
tags: [chatminal, ui-shell, desktop, non-core]
created: 2026-03-23
---

# Chatminal UI Shell Without Terminal Core Touch

## Mục tiêu
- Nâng chất lượng `sidebar`, `session bar`, `footer`, `overlay`, `layout primitives`, `scroll-tree-list`, và `icon animation` nhẹ.
- Giữ terminal core, parser, PTY execution, runtime lifecycle như black box.
- Dồn mọi thay đổi vào desktop shell/UI chrome của `apps/chatminal-desktop/src/*`.

## Safe Seams
- `apps/chatminal-desktop/src/termwindow/*`
- `apps/chatminal-desktop/src/desktop_termwindow_*`
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/overlay/*`
- `apps/chatminal-desktop/src/chatminal_layout/*`
- `apps/chatminal-desktop/src/chatminal_render/*`

## Explicit Out Of Scope
- `crates/chatminal-terminal-core/**`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/**`
- `crates/chatminal-runtime/src/state/**`
- PTY/input parser semantics, terminal screen model, shell lifecycle, scrollback persistence contract

## Phases
- Phase 01: [boundary-freeze và inventory shell UI](./phase-01-shell-boundary-and-ui-surface-freeze.md) - pending
- Phase 02: [layout primitives và chrome slots](./phase-02-layout-primitives-and-chrome-slots.md) - pending
- Phase 03: [sidebar và scroll-tree-list polish](./phase-03-sidebar-and-scroll-tree-list-polish.md) - pending
- Phase 04: [session bar và footer refinement](./phase-04-session-bar-and-footer-refinement.md) - pending
- Phase 05: [overlay layering và shell interactions](./phase-05-overlay-layering-and-shell-interactions.md) - pending
- Phase 06: [icon motion và micro-feedback](./phase-06-icon-motion-and-micro-feedback.md) - pending
- Phase 07: [verification, docs sync, rollout gates](./phase-07-verification-docs-sync-and-rollout-gates.md) - pending

## Dependencies
- Phase 01 khóa boundary + naming + safe seams.
- Phase 02 tạo primitive/layout contract dùng lại cho Phase 03-06.
- Phase 03-05 chia theo surface độc lập nhưng phải tuân layout contract của Phase 02.
- Phase 06 chỉ bắt đầu sau khi hover/focus/overlay states đã ổn định.
- Phase 07 chạy sau toàn bộ UI polish để tránh verify noise.

## Completion Gates
- UI shell đẹp hơn nhưng không đổi terminal behavior contract.
- Không có diff trong `crates/chatminal-terminal-core/**`.
- Không có diff trong runtime execution core/private engine zones.
- `cargo check -p chatminal-desktop` pass.
- Có grep gate xác nhận không lỡ kéo thêm dependency vào terminal core path.

## Working Rule
- Nếu cần thêm dữ liệu từ runtime, chỉ dùng facade/query có sẵn ở app layer.
- Nếu vướng vì thiếu data ở facade, dừng phase và mở follow-up plan riêng; không lấn scope vào core.
