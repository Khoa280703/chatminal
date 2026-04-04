---
status: done
created: 2026-04-03
branch: main
---

# Post-Unification Follow-Ups

## Goal
Nhận phần việc đã được tách scope ra khỏi `260401-0949-architecture-unification` để plan closeout kia có thể đóng sạch mà không nói quá hiện trạng source.

## Phases
| Order | Phase | Status | Purpose |
|---|---|---|---|
| 1 | [phase-01-config-ownership-completion.md](./phase-01-config-ownership-completion.md) | done | Hoàn tất `Phase 04 Step 3/4`: config sectioning + end-to-end singleton removal |
| 2 | [phase-02-terminal-crate-rename.md](./phase-02-terminal-crate-rename.md) | done | Rename/cosmetic sweep cho `engine-*` / terminal crates |

## Why This Exists
- `260401-0949-architecture-unification` closeout chỉ chốt product ownership/runtime boundary.
- Config sectioning/full propagation và crate rename không còn là blocker kỹ thuật cho closeout đó, nhưng vẫn là engineering debt hợp lệ.

## Verification
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
