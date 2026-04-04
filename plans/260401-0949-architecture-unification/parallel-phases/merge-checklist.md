# Merge Checklist

## Goal
File này chỉ còn giữ trạng thái merge-wave và phần integration backlog đã absorb. Nó không còn là nguồn quyết định `plan done`.

Nguồn done-gate cuối cùng là:
- [../phase-05-final-closeout.md](../phase-05-final-closeout.md)
- [../final-closeout-checklist.md](../final-closeout-checklist.md)

## Current State
- [x] 4 employee stream đã được merge vào nhánh tích hợp
- [x] merge-wave verify đã từng xanh:
  - [x] `cargo check --workspace`
  - [x] `cargo test --workspace --lib --bins --tests`
  - [x] `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - [x] `make window`
- [x] caller-side typed boundary đã tiến xa
- [x] integration backlog lớn của merge wave đã absorb
- [x] closeout sau merge-wave đã hoàn tất trong [../phase-05-final-closeout.md](../phase-05-final-closeout.md)

## Historical Note
- Các checkbox mở bên dưới là backlog đã tồn tại ngay sau merge-wave.
- Chúng đã được đóng ở phase closeout sau đó; file này chỉ giữ ngữ cảnh merge history.

## Remaining Work After Merge Wave
- [x] Runtime ownership closeout
- [x] PTY default owner final cut
- [x] Raw ID final audit/internalization
- [x] Config/rename scope decision
- [x] Final docs + verification + status sync

## Rule
- Chỉ dùng file này để biết merge-wave đã tới đâu.
- Khi muốn trả lời "plan `260401-0949-architecture-unification` đã 100% chưa", luôn nhìn sang `phase-05-final-closeout.md` và `final-closeout-checklist.md`.
