---
title: "Merge Terminal Core And Emulator"
description: "Hợp nhất 2 terminal layer hiện tại thành một terminal domain duy nhất cho active product path."
status: done
priority: P1
effort: 2-4d
branch: main
tags: [architecture, terminal, runtime, cleanup]
created: 2026-04-04
---

# Merge Terminal Core And Emulator

## Goal
Xóa duplicate terminal architecture giữa `crates/chatminal-terminal-core` và `crates/chatminal-terminal-emulator`, chốt một canonical terminal layer duy nhất cho active product path, rồi dọn dependency/docs tương ứng.

## Assumption Freeze
Plan này hiểu “2 terminal layer” là:
1. `chatminal-terminal-core`
2. `chatminal-terminal-emulator` (`lib.name = engine_term`)

Nếu về sau user muốn nói tới layer khác như `termwindow` vs `session_engine`, đó là plan khác.

## Ground Truth Từ Source
- `chatminal-terminal-core` hiện gần như chỉ còn giá trị active ở `TerminalSize` và vài lightweight type; không có active callsite dùng `chatminal_terminal_core::Terminal`.
- Terminal behavior thật của desktop/host/session runtime đang nằm ở `engine_term::Terminal`.
- `apps/chatminal-desktop` hiện phụ thuộc đồng thời cả hai crates, nên duplication là thật trong active product path.

## Canonical Direction
- Canonical terminal layer sau merge: `chatminal-terminal-emulator`
- `chatminal-terminal-core` không được giữ như một “layer thứ hai” sau closeout.
- Steady-state sau plan này chỉ được còn một terminal architecture; không giữ adapter/compat crate song song dài hạn.
- Không tạo crate thứ ba kiểu `chatminal-terminal-types`; như vậy chỉ chuyển duplicate thành overkill.

## Phases
| Order | Phase | Status | Purpose |
|---|---|---|---|
| 1 | [phase-01-freeze-single-terminal-contract.md](./phase-01-freeze-single-terminal-contract.md) | done | Chốt boundary và contract terminal duy nhất |
| 2 | [phase-02-cut-session-engine-off-terminal-core.md](./phase-02-cut-session-engine-off-terminal-core.md) | done | Đổi session-engine/desktop host khỏi `chatminal-terminal-core` |
| 3 | [phase-03-delete-terminal-core-and-collapse-deps.md](./phase-03-delete-terminal-core-and-collapse-deps.md) | done | Xóa crate cũ, dọn Cargo/docs/tests/deadcode |
| 4 | [phase-04-verify-closeout-and-doc-sync.md](./phase-04-verify-closeout-and-doc-sync.md) | done | Verify, doc sync, quyết định residual naming debt |

## Non-Goals
- Không đổi UI shell `termwindow/*`.
- Không rewrite host-runtime/Mux architecture trong wave này.
- Không rename toàn bộ `engine_term` import path ở wave đầu nếu không phải blocker merge.
- Không thêm compatibility shim dài hạn chỉ để giữ `chatminal-terminal-core` sống.

## Success Criteria
- `rg` trong active source không còn dependency product-path vào `chatminal_terminal_core::*`.
- `chatminal-terminal-core` được xóa hoàn toàn khỏi workspace active path; không tồn tại adapter crate dài hạn thay tên đổi vỏ.
- `session_engine`, `desktop_host_runtime`, `host_runtime` cùng dùng một terminal type system thống nhất.
- Docs active scope không còn mô tả “2 terminal layer” như current reality.
- `cargo check --workspace`, desktop tests quan trọng, và `make window` smoke đều xanh.

## Main Risks
- Lẫn giữa “type merge” và “behavior merge”, dẫn tới cắt nhầm layer đang giữ emulator semantics.
- Churn import/type alias lớn làm phát sinh compile fallout rộng.
- Nếu rename `engine_term` ngay cùng wave thì diff sẽ phình to không cần thiết.

## Verification Spine
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- `make window`
- `rg -n "chatminal_terminal_core::|chatminal-terminal-core" apps crates docs README.md`

## Closeout Notes
- `chatminal-terminal-core` đã bị xóa khỏi workspace active path.
- Desktop/session-native runtime đã converge về `engine_term::TerminalSize`.
- `cargo check -p chatminal-desktop`: pass
- `cargo check --workspace`: pass
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`: pass (`92` tests)
- `cargo test --workspace --lib --bins --tests`: pass
- `make window`: smoke pass
