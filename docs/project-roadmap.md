# Project Roadmap

Last updated: 2026-03-06

## Milestones
| Milestone | Status |
| --- | --- |
| Workspace chuyển sang native runtime-only | Completed |
| Daemon runtime (`chatminald`) | Completed |
| Native client + internal terminal-core integration | Completed |
| Hard-cut WezTerm runtime dependency | Completed |
| Internal terminal-core boundary established | Completed |
| Full native window UX parity | In Progress |
| Cross-platform transport hardening (UDS + Named Pipe) | Completed |
| Input pipeline rollout/rollback (Phase 06) | Completed |
| Windows input parity follow-up (Phase 07) | Completed |
| Non-blocking session creation in native UI | Completed |

## Next priorities
1. Nâng `chatminal-app` từ CLI/TUI lên native window app đầy đủ.
2. Tăng coverage integration/soak test cho long-running sessions.
3. Tối ưu UX/perf cho workflow daily-driver trên Linux/macOS trước khi đóng gói rộng.
