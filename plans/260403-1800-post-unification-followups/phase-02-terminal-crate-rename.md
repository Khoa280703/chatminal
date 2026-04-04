---
phase: 02
status: done
priority: low
effort: medium
risk: low
---

# Phase 02: Terminal Crate Rename

## Scope
- `260401-0949-architecture-unification` Phase 02

## Goal
- Rename `chatminal-engine-*` crates và aliases liên quan sang vocabulary terminal/chatminal rõ ràng hơn.

## Notes
- Đây là naming/cosmetic sweep.
- Không phải blocker của runtime ownership closeout.
- Đã thực hiện sau khi ownership/config boundaries đã ổn định.
- Lượt này giữ `lib.name` và compatibility alias `engine-*` để tránh churn import Rust hàng loạt; package/path/docs vocabulary đã chuyển sang `chatminal-terminal-*` / `chatminal-*`.

## Done Criteria
- workspace không còn crate/package naming gây hiểu nhầm
- import aliases, Cargo manifests, docs đều sync
- build/test xanh toàn workspace

## Verification
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
- `git diff --check`
