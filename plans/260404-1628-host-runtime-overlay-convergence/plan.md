---
title: "Host Runtime And Overlay Convergence"
description: "Collapse remaining desktop host legacy seam, retire Mux compatibility facade from active architecture, and converge overlay shell contract without touching history/config compatibility."
status: completed
priority: P1
effort: 4-6d
branch: main
tags: [architecture, cleanup, desktop, host-runtime, overlay]
created: 2026-04-04
---

# Host Runtime And Overlay Convergence

## Goal
Làm sạch 3 debt kiến trúc còn lại sau terminal-layer merge: cắt `legacy_*` desktop host seam, hạ `Mux` xuống internal-only/private adapter hoặc xóa khỏi active product path, và gom `overlay_compat` về một shell contract duy nhất.

## In Scope
- `apps/chatminal-desktop/src/desktop_host_runtime/*`
- `crates/chatminal-host-runtime/*`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/overlay/*`
- `apps/chatminal-desktop/src/termwindow/*` chỉ ở phần import/contract cần đổi theo overlay cutover

## Explicit Non-Goals
- Không đụng legacy/canonical history compat: `scrollback_chunks`, `canonical_scrollback`, prompt reconstruction
- Không dọn `desktop_commands.rs` / config backward-compat
- Không thêm feature UI mới
- Không rewrite terminal core/emulator thêm lần nữa

## Ground Truth From Source
- `desktop_host_runtime/mod.rs` active product path đã không còn route behavior chính qua `legacy_*`
- `session_host.rs` bootstrap/shutdown product path và test seam đã gom về helper chung; active client/workspace không còn cache legacy song song
- `chatminal-host-runtime` public init handle canonical là `HostRuntimeHandle`; `mux_default()` chỉ còn explicit compat seam
- overlay product path đã cắt bridge `chatminal_runtime::overlay_shell`; canonical owner là `desktop_host_runtime::overlay_shell`

## Phases
| Order | Phase | Status | Purpose |
|---|---|---|---|
| 1 | [phase-01-collapse-desktop-host-legacy-surface.md](./phase-01-collapse-desktop-host-legacy-surface.md) | completed | Cắt dual surface `host`/`legacy_*` ở desktop host adapter |
| 2 | [phase-02-retire-mux-compat-facade.md](./phase-02-retire-mux-compat-facade.md) | completed | Bỏ `Mux` khỏi active ownership/default vocabulary |
| 3 | [phase-03-converge-overlay-shell-contract.md](./phase-03-converge-overlay-shell-contract.md) | completed | Gom overlay render/input contract về shared shell surface duy nhất |
| 4 | [phase-04-closeout-verify-and-prune.md](./phase-04-closeout-verify-and-prune.md) | completed | Verify, prune deadcode/docs active scope, khóa steady-state |

## Success Criteria
- Active desktop product flow không còn fallback runtime behavior qua `legacy_*` wrappers
- `MuxHandle` không còn là public/default mental model cho product path; `mux_default()` không còn là default seam sống
- Overlay/termwindow/import path không còn phụ thuộc vào `overlay_compat` như một contract song song
- `cargo check --workspace`, `cargo test --workspace --lib --bins --tests`, `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` xanh
- Grep active scope không còn residual seam ngoài intentional test-only/private shims được ghi rõ

## Main Risks
- Cutover host seam có thể làm gãy startup/focus/spawn/shutdown nếu xóa wrapper sai thứ tự
- Overlay contract cutover có thể làm gãy hit-test, render clipping, scroll/cursor sync ở overlay panes
- Nếu phase 1 chưa chốt boundary sạch, phase 2 và 3 sẽ chỉ đổi tên chứ không giảm coupling thật

## Verification Spine
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- `make window`
- `rg -n "legacy_|MuxHandle|mux_default\(|overlay_compat" apps/chatminal-desktop crates/chatminal-host-runtime docs README.md`

## Closeout Note
- Phase 01 done: desktop host product path không còn behavioral fallback qua `legacy_*`; `session_host.rs` cũng không còn state cache legacy song song.
- Phase 02 done: active desktop bootstrap dùng `HostRuntimeHandle` canonical; `MuxHandle` không còn là owner mental model ở product path; `mux_default()` chỉ còn compat seam explicit trong host-runtime.
- Phase 03 done: `chatminal_runtime::overlay_shell` bridge đã bị xóa; overlay/termwindow active imports đi thẳng vào `desktop_host_runtime::overlay_shell`.
- Phase 04 done: docs active scope đã sync, grep seam residuals sạch trong active scope ngoài intentional historical/docs mentions, và targeted verification xanh:
  - `cargo check -p chatminal-desktop`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml desktop_host_runtime::session_host -- --test-threads=1`
  - `cargo check -p chatminal-host-runtime`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
