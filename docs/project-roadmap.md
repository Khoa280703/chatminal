# Project Roadmap

Last updated: 2026-03-04

## Milestones
| Milestone | Status |
| --- | --- |
| Workspace chuyển sang native runtime-only | Completed |
| Daemon runtime (`chatminald`) | Completed |
| Native client + wezterm-term integration | In Progress |
| Full native window UX parity | Planned |
| Cross-platform transport hardening (UDS + Named Pipe) | In Progress |

## Next priorities
1. Nâng `chatminal-app` từ CLI/TUI lên native window app đầy đủ.
2. Giảm lock contention trong daemon state path.
3. Tăng coverage integration/soak test cho long-running sessions.
