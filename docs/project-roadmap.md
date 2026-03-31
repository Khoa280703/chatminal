# Project Roadmap

Last updated: 2026-03-31 (performance optimization, startup recipes, UI shell in progress)

## Milestones
| Milestone | Status |
| --- | --- |
| Workspace chuyển sang native runtime-only | Completed |
| Single-process desktop model (daemon deleted) | Completed (2026-03-31) |
| Native client + internal terminal-core integration | Completed |
| Hard-cut WezTerm runtime dependency | Completed |
| Internal terminal-core boundary established | Completed |
| Cross-platform transport hardening (UDS + Named Pipe) | Completed |
| Input pipeline rollout/rollback (Phase 06) | Completed |
| Windows input parity follow-up (Phase 07) | Completed |
| Non-blocking session creation in native UI | Completed |
| Performance optimization phases 1-4 | Completed (2026-03-31) |
| Startup recipes per-session | Completed (2026-03-31) |
| Canonical scrollback dual-read/canonical-write | Completed (2026-03-27) |
| UI Shell without terminal-core touch | In Progress |
| Full window-parity + multi-session UX | In Progress |

## Next priorities
1. Complete UI Shell phase (GPU scissor, overlay bounds, coordinate calculations)
2. Verify window parity across Linux/macOS (sidebar, footer, tab-bar rendering)
3. Profile remaining hot-path allocations (duplicate terminal parser, workspace_load caching)
4. Expand test coverage for startup recipes, persist worker, session lifecycle
5. Prepare for multi-window session management (optional Phase 46+)
