---
status: done
created: 2026-04-01
branch: main
---

# Architecture Unification Plan

## Goal
Biến Chatminal từ lớp Chatminal bọc trên host/runtime kiểu WezTerm sang một desktop app có ownership rõ ràng, boundary typed, và lifecycle product path không còn phụ thuộc `Mux` như owner thật.

## Final Status
Plan này đã được closeout theo code thực tế vào `2026-04-03`.

## Phase Summary
| Phase | Status | Result |
|---|---|---|
| [01](./phase-01-prune-dead-wezterm-code.md) | done | Xóa dead WezTerm/config surface |
| [03](./phase-03-kill-mux-singleton.md) | done | Product path ownership/lifecycle đã rời `Mux` owner semantics |
| [04](./phase-04-config-independence.md) | done | Closeout scope đã chốt; phần deep propagation đã tách follow-up |
| [05](./phase-05-final-closeout.md) | done | Final closeout, verify, docs sync hoàn tất |
| [02](./phase-02-rename-engine-crates.md) | done | Đã hoàn tất qua follow-up plan `260403-1800-post-unification-followups` |

## Closeout Evidence
- `initialize_host_runtime()` dựng `HostRuntimeRoot` trực tiếp.
- `shutdown_host_runtime()` chỉ clear installed root, không còn gọi `Mux::shutdown()`.
- `with_mux(` và `with_mux_strict(` sạch trong `crates/` + `apps/`.
- Product PTY/local spawn path dùng `host_default()`; `mux_default()` chỉ còn là explicit compat alias/tests.
- `configuration(` trong closeout scope chỉ còn ở:
  - `crates/chatminal-config/src/lib.rs`
  - `crates/chatminal-config/src/terminal.rs`
  - `apps/chatminal-desktop/src/shapecache.rs` test code
  - comment/documentation references
- Raw `PaneId` / `TabId` không còn là cross-crate blocker; phần còn lại chỉ là crate-local internal hoặc wire-compat shapes.

## Verification
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- `make window` bounded smoke launch

## Follow-Up Status
Các follow-up từng được tách khỏi closeout này đã được hoàn tất tại:
- [Architecture Unification Follow-Ups](/Users/khoa2807/development/2026/chatminal/plans/260403-1800-post-unification-followups/plan.md)
