---
title: "Single-Flow Desktop Path Legacy Vocabulary Removal"
description: "Refactor desktop product path to stop exposing or routing by legacy execution-target semantics while preserving private runtime compatibility."
status: completed
priority: P1
effort: 2.5d
branch: main
tags: [architecture, desktop, runtime, cleanup]
created: 2026-03-25
---

# Overview

Goal: desktop product path nhìn và chạy như 1 luồng duy nhất. Legacy vocabulary cũ không còn xuất hiện hay điều khiển flow ở public/product-facing path. Active config/runtime vocabulary đã được dọn sang `target`.

## Phases

1. `completed` Phase 01
Cắt toàn bộ legacy vocabulary cũ khỏi UI/menu/command/public labels và desktop-facing command routing.

2. `completed` [Phase 02](./phase-02-single-flow-spawn-resolution.md)
Collapse spawn resolution của desktop path về một đường duy nhất, không còn `SpawnSessionTarget::*` trong desktop product flow.

3. `completed` Phase 03
Rút `spawn_target_id`/target lookup khỏi desktop adapter path nơi product không cần biết đến execution target.

4. `completed` [Phase 04](./phase-04-compat-tail-docs-and-guardrails.md)
Dọn tiếp config/docs/guardrails để active product path không còn legacy naming cũ.

## Current status

- Desktop UI/menu/toolbar/command surface không còn flow public theo execution target.
- Desktop startup/CLI path không còn route theo legacy attach/spawn selection flags; host mux init mặc định ép `local`.
- Lua public API `session.get_target`, `session.all_targets`, `session.set_default_target` đã bị cắt.
- Host/runtime/config vocabulary active path đã đổi sang `spawn target`.
- Config breaking rename hoàn tất: legacy target-list keys -> `*_targets`, legacy default target key -> `default_target`.

## Key dependencies

- `apps/chatminal-desktop/src/desktop_commands.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_{impl,items}.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/*`
- `crates/chatminal-config/src/keyassignment.rs`
- `crates/chatminal-host-runtime/*`
- `crates/chatminal-lua-bridge/*`

## Done criteria

- Desktop product path không còn menu/label/command public nào dùng legacy vocabulary cũ.
- Desktop spawn/focus path không cần caller product truyền execution target.
- Legacy vocabulary cũ không còn nằm trong active desktop/runtime/config product path.
- `cargo check -p chatminal-desktop` pass ở mỗi phase; `cargo check --workspace` pass ở phase cuối.
