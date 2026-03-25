---
title: "Single-Flow Desktop Path Domain Removal"
description: "Refactor desktop product path to stop exposing or routing by domain while preserving private runtime compatibility."
status: pending
priority: P1
effort: 2.5d
branch: main
tags: [architecture, desktop, runtime, cleanup]
created: 2026-03-25
---

# Overview

Goal: desktop product path nhìn và chạy như 1 luồng duy nhất. `domain` không còn xuất hiện hay điều khiển flow ở public/product-facing path. Private host-runtime compat được cô lập rồi dọn sau.

## Phases

1. `pending` [Phase 01](./phase-01-cut-product-facing-domain-surface.md)
Cut toàn bộ `domain` khỏi UI/menu/command/public labels và desktop-facing command routing.

2. `pending` [Phase 02](./phase-02-single-flow-spawn-resolution.md)
Collapse spawn resolution của desktop path về một đường duy nhất, không còn `SpawnSessionDomain::*` trong desktop product flow.

3. `pending` [Phase 03](./phase-03-collapse-desktop-adapter-domain-routing.md)
Rút `domain_id`/domain lookup khỏi desktop adapter path nơi product không cần biết đến nó.

4. `pending` [Phase 04](./phase-04-compat-tail-docs-and-guardrails.md)
Khoanh `domain` còn lại vào compat/private zone, cập nhật docs, thêm guard không cho public path tái lộ `domain`.

## Slice nên làm ngay

Làm **Phase 01** trước.

Lý do:
- ROI cao nhất: user-facing confusion biến mất ngay.
- Rủi ro thấp: chủ yếu dọn command surface, labels, routing entry points.
- Tạo nền cho phase sau: sau khi product path không còn dùng `domain`, mới an toàn collapse spawn path và adapter internals.

## Key dependencies

- `apps/chatminal-desktop/src/desktop_commands.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_{impl,items}.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/*`
- `crates/chatminal-config/src/keyassignment.rs`
- `crates/chatminal-host-runtime/*`
- `crates/chatminal-lua-bridge/*`

## Done criteria

- Desktop product path không còn menu/label/command public nào dùng chữ `domain`.
- Desktop spawn/focus path không cần caller product truyền `domain`.
- `domain` chỉ còn trong private compat zone hoặc bị xoá hẳn nếu phase tương ứng hoàn tất.
- `cargo check -p chatminal-desktop` pass ở mỗi phase; full workspace verify ở phase cuối.
