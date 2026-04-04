---
status: pending
created: 2026-04-03
branch: main
---

# Architecture Unification Follow-Ups

## Goal
Chứa phần việc đã được tách scope chính thức khỏi closeout của `260401-0949-architecture-unification`, để plan closeout có thể kết thúc mà không để debt ở trạng thái mơ hồ.

## Phases
| Order | Phase | Status | Purpose |
|---|---|---|---|
| 1 | [phase-01-config-deep-independence.md](./phase-01-config-deep-independence.md) | pending | Hoàn tất `Phase 04 Step 3/4`: config sub-struct restructure + propagate config explicit end-to-end |
| 2 | [phase-02-engine-rename-cosmetic.md](./phase-02-engine-rename-cosmetic.md) | pending | Hoàn tất rename/cosmetic sweep `engine-* -> chatminal-terminal-*` |

## Scope Contract
- Hai phase này không còn chặn done-gate của `260401-0949-architecture-unification`.
- Mọi claim đóng current closeout phải trỏ về plan này như destination chính thức cho phần deferred.
