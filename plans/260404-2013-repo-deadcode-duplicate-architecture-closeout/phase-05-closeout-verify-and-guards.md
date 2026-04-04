---
phase: 05
status: completed
priority: medium
effort: low
risk: low
---

# Phase 05: Closeout Verify And Guards

## Overview
Khóa closeout bằng compile/tests, bounded smoke run, và grep guards cho 6 finding để tránh debt quay lại ngay ở wave sau.

## Closeout
- Verification spine đã xanh:
  - `cargo check -p chatminal-desktop`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
  - `cargo test -p chatminal-runtime -- --test-threads=1`
  - `cargo check --workspace`

## Why This Phase Exists
- Các finding này chủ yếu là architectural cleanup; nếu không khóa bằng grep/verification thì rất dễ regress âm thầm
- Một số debt là duplicate surface theo API/docs, không phải behavior bug, nên verification cần thêm static guards

## Scope
- workspace-wide verification
- targeted grep guards
- docs closeout note nếu còn intentional keep nào chưa xóa được

## Requirements
- Không claim done nếu chưa có compile/test/grep evidence mới
- Nếu còn giữ compat seam nào, phải có owner và lý do rõ

## Implementation Steps
1. Chạy full verification spine trong `plan.md`.
2. Chạy targeted greps cho facade wrappers, dual control-plane APIs, `compat_default()`, và legacy scrollback reads.
3. Bounded `make window` smoke launch để chắc desktop boot path không gãy.
4. Ghi closeout note ngắn: what was removed, what intentionally remains, and why.

## Todo List
- [x] `cargo check --workspace`
- [x] `cargo test --workspace --lib --bins --tests`
- [x] Desktop app tests
- [x] Targeted grep guards all green
- [x] `make window` bounded smoke run
- [x] Closeout note for intentional keeps

## Success Criteria
- Verification spine xanh
- Grep guards không còn match các surface bị retire, trừ intentional keeps đã được ghi rõ
- Plan có thể đóng mà không còn ambiguous debt trong đúng 6 finding này

## Risk Assessment
- Risk thấp; chủ yếu là lộ thêm residual debt ngoài scope
- Mitigation: nếu grep lộ finding mới, ghi riêng follow-up thay vì mở rộng vô hạn phase hiện tại

## Security Considerations
- Không có security impact trực tiếp

## Next Steps
- Nếu phase này xanh, plan được coi là closeout xong cho 6 finding review hiện tại
