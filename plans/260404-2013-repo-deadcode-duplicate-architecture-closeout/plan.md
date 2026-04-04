---
title: "Repo Deadcode And Duplicate Architecture Closeout"
description: "Cleanup plan for the six remaining repo-wide review findings: duplicate desktop facade, dual host-runtime control APIs, legacy scrollback dual-read, docs drift, semantically-empty compat defaults, and orphan public wrappers."
status: done
priority: P1
effort: 2-4d
branch: main
tags: [architecture, cleanup, deadcode, docs, runtime, scrollback]
created: 2026-04-04
---

# Repo Deadcode And Duplicate Architecture Closeout

## Goal
Đóng 6 finding review theo hướng clean architecture thật sự: một desktop boundary, một host-runtime control-plane surface, một scrollback read model, và active docs phản ánh đúng code hiện tại.

## Source Findings Covered
1. Duplicate desktop facade ở `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
2. Dual public control-plane API trong `crates/chatminal-host-runtime/src/lib.rs`
3. Canonical scrollback read-path vẫn merge legacy `scrollback_chunks`
4. Active docs drift ở `docs/system-architecture.md` và `docs/codebase-summary.md`
5. `compat_default()` family gần như không còn semantic difference
6. Public wrappers `register_runtime_client(...)` và `replace_active_identity(...)` có dấu hiệu vô chủ

## Ground Truth
- Product path hiện là `single-runtime desktop`
- `desktop_host_runtime` là private adapter layer gần canonical nhất cho desktop shell
- `HostRuntimeHandle` đã là bootstrap/control canonical; free-function wrappers public vô chủ đã bị cắt
- Steady-state scrollback read path đã canonical-only; legacy chunks chỉ còn migration residue/backfill-once seam
- Active docs active đã được sync lại với current code reality

## Closeout Status
- Phase 01: completed
- Phase 02: completed
- Phase 03: completed
- Phase 04: completed
- Phase 05: completed
- Verification spine: green

## Phases
- [Phase 01](./phase-01-collapse-desktop-runtime-facade.md): cắt duplicate desktop facade, chốt `desktop_host_runtime` làm entry nội bộ duy nhất cho desktop product path
- [Phase 02](./phase-02-unify-host-runtime-control-plane.md): gom host-runtime về một control-plane surface, xử lý luôn `compat_default()` và orphan wrappers
- [Phase 03](./phase-03-retire-legacy-scrollback-read-path.md): đưa history/scrollback steady-state về canonical read model thật sự
- [Phase 04](./phase-04-sync-active-docs-and-prune-dead-surface.md): đồng bộ active docs với code và xóa dead surface unlocked bởi phases trước
- [Phase 05](./phase-05-closeout-verify-and-guards.md): verify, grep guards, và chốt intentional keeps

## Why This Order
1. Desktop facade duplicate đang giữ 2 mental model ở top-level app boundary.
2. Host-runtime control-plane phải chốt xong trước khi dọn docs, vì nhiều docs drift bám đúng boundary này.
3. Scrollback dual-read là state/storage debt riêng; để phase riêng để không trộn với host-runtime cleanup.
4. Docs/deadcode cleanup chỉ có giá trị sau khi contracts thật đã ổn định.
5. Cuối cùng mới khóa bằng verification + grep guards để tránh reopen cùng class debt.

## Success Criteria
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` không còn là facade one-hop cho desktop host runtime product path
- `chatminal-host-runtime` còn đúng một public control-plane style canonical cho caller mới
- `compat_default()` không còn là public duplicate surface; compat hẹp nếu còn thì phải crate-local
- `register_runtime_client(...)` và `replace_active_identity(...)` không còn public debt vô chủ
- `chatminal-runtime` steady-state snapshot build không còn merge `scrollback_chunks` legacy trong read path
- `docs/system-architecture.md` và `docs/codebase-summary.md` phản ánh đúng ownership/control/dependency story hiện tại

## Non-Goals
- Không thay đổi UI/UX feature surface
- Không rewrite archive/history docs chỉ để đổi wording
- Không gộp thêm các refactor ngoài 6 finding này
- Không merge thêm terminal layers; scope này chỉ là cleanup debt sau merge/unification waves trước

## Verification Spine
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- `rg "chatminal_runtime::(initialize_desktop_host_runtime|runtime_client|shutdown_desktop_host_runtime|subscribe_runtime_notifications)" apps/chatminal-desktop/src`
- `rg "register_runtime_client|replace_active_identity|compat_default\(|list_legacy_scrollback_chunks\(" crates apps`
- bounded `make window` smoke launch
