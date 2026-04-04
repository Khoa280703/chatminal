---
phase: 05
status: done
priority: critical
effort: medium-large
risk: high
---

# Phase 05: Final Closeout

## Overview
Phase này đã hoàn tất. Mục tiêu của phase là đóng nốt blocker thật trong source, chốt deferred scope explicit, rồi sync plan/docs theo đúng code reality.

## Completed
- Runtime ownership closeout:
  - `initialize_host_runtime()` boot từ `HostRuntimeRoot`
  - `shutdown_host_runtime()` clear root trực tiếp
  - notify path active không còn đi qua `Mux::notify_from_any_thread` như owner mặc định
- PTY default owner closeout:
  - product path dùng `host_default()`
  - `mux_default()` chỉ còn explicit compat alias/tests
  - desktop local/serial spawn dùng `LocalSpawnHooks::host_default()`
- Raw ID closeout:
  - cross-crate/public boundary không còn blocker `PaneId` / `TabId`
  - phần còn lại chỉ là crate-local internal hoặc wire-compat
- Scope decision closeout:
  - `Phase 04 Step 3/4` đã tách sang follow-up plan
  - `Phase 02` đã tách sang follow-up plan
- Docs/status sync:
  - parent `plan.md`
  - final checklist
  - phase files liên quan
  - docs `project-changelog`, `system-architecture`, `codebase-summary`

## Verification
- [x] `cargo check --workspace`
- [x] `cargo test --workspace --lib --bins --tests`
- [x] `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- [x] `make window` bounded smoke launch

## Follow-Up Plan
- [Architecture Unification Follow-Ups](/Users/khoa2807/development/2026/chatminal/plans/260403-1800-post-unification-followups/plan.md)

## Done Condition
- [x] Product path không còn `Mux` owner blocker
- [x] Product path không còn hidden/default `mux_default()` blocker
- [x] Raw cross-crate boundary blocker đã được đóng
- [x] Deferred scope đã có destination plan rõ ràng
- [x] Plan/docs/verify đã sync xong
