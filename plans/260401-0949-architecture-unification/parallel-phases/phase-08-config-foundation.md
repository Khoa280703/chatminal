# Phase 08: Config Foundation

## Goal
Chốt foundation của config để propagation phase sau có contract ổn định.

## Lane
### Lane 08A: Config API Foundation
- Ownership:
  - `crates/chatminal-config/src/*`
- Scope:
  - nếu còn cần, gom/khóa các entry point config chính
  - tránh đổi lan sang desktop/host-runtime trong cùng phase
  - quyết định rõ phần nào của Step 3/4 giữ defer, phần nào sẽ làm thật

## Parallel Safety
- Không tách lane; phase này là contract freeze cho Phase 09.

## Gate
- `cargo check -p chatminal-config`
- caller compile smoke
