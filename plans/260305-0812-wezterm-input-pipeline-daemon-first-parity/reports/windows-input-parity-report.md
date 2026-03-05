# Windows Input Parity Report (Phase 07)

Date: 2026-03-05  
Owner: chatminal core

## Scope
- Windows transport: Named Pipe (`apps/chatminald/src/transport/windows.rs`, `apps/chatminal-app/src/ipc/transport/windows.rs`)
- Input parity baseline: shared input layer + unit tests chạy trong CI `windows-latest`.

## Checklist Definition
| Item | Status | Evidence |
| --- | --- | --- |
| Modifier/control mapping checklist defined | completed | table below |
| CI windows có compile/test regression coverage tối thiểu | completed | `rewrite-quality-gates.yml` windows job chạy `cargo check` + app/daemon/protocol/store tests |
| Docs phản ánh trạng thái Windows fidelity | completed | `docs/terminal-fidelity-matrix.md` windows note |

## Mapping Checklist (Windows)
1. `Ctrl+C` -> interrupt foreground process.
2. `Ctrl+Z` -> stop foreground process (shell-dependent).
3. `Alt+Backspace` không làm treo input pipeline.
4. `RightAlt/AltGr` không tạo duplicate text commit.
5. `Meta/Ctrl` semantics không phá terminal shortcuts.

## Current Status
- CI coverage: pass ở mức compile + unit/integration tests.
- Phase 07 follow-up scope: completed ở mức baseline parity (CI + mapping checklist).
- Manual matrix trên máy Windows thật: chuyển sang release hardening checklist (không block completed state của Phase 07).
