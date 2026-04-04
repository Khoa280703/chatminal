# Phase 10: Final Rename And Sweep

## Goal
Chạy phần rename/cosmetic/global sweep sau khi functional architecture đã khóa.

## Lane
### Lane 10A: Rename Engine Crates And Final Sweep
- Ownership:
  - `Cargo.toml`
  - `Cargo.lock`
  - toàn bộ workspace references liên quan rename phase 02
  - docs/plan/changelog sweep cuối
- Scope:
  - rename `engine-*` -> `chatminal-terminal-*`
  - xóa compat/dead alias cuối
  - final grep sweep cho `Mux`, `PaneId`, `TabId`, `configuration()` ở boundaries mục tiêu

## Parallel Safety
- Phase cuối, không chạy song song với phase khác.

## Gate
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
- docs sync hoàn chỉnh
