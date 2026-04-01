---
status: in_progress
created: 2026-04-01
branch: main
---

# Architecture Unification Plan

## Goal
Biến Chatminal từ "WezTerm core bọc Chatminal layers" → unified codebase.

## Phân loại hiện tại (57 crates)
- **4 crates Chatminal native** (runtime, store, terminal-core, metrics)
- **14 crates adapter/glue** (desktop, config, host-runtime, window, lua-bridge, etc)
- **39 crates WezTerm forks** (engine-*, utility crates)

## Phases (priority order)

| Order | Phase | Name | Effort | Risk | Impact |
|-------|-------|------|--------|------|--------|
| 1st | [01](phase-01-prune-dead-wezterm-code.md) | Prune Dead WezTerm Code | Small | Low | -5K+ LOC, cleaner config |
| 2nd | [03](phase-03-kill-mux-singleton.md) | Kill Mux → RuntimeHost | Large | Medium | True session model |
| 3rd | [04](phase-04-config-independence.md) | Config Independence | Medium-Large | Medium | Single config owner |
| Last | [02](phase-02-rename-engine-crates.md) | Rename engine-* → chatminal-terminal-* | Medium | Low | Naming unity (cosmetic) |

## Progress
- Phase 01 complete on 2026-04-01:
  - removed dead SSH/TLS/WSL config surface
  - removed auto-update config stubs
  - removed `libssh-rs` workspace dependency
  - kept only minimal SSH CLI helpers (`SshParameters`, `username_from_env`) because they are still active compile-path utilities
- Next safe execution target: Phase 03A/03B after separate audit of current dirty runtime worktree

## Dependency Graph
```
Phase 01 (prune) ─→ Phase 03 (kill Mux + 3G: PTY pipeline) ─→ Phase 04 (config) ─→ Phase 02 (rename, cosmetic)
```
Phase 02 deferred to last — rename is cosmetic, pollutes diffs, delivers no architectural value during active refactoring.

## Hard Boundaries
- **DO NOT touch** engine-term internals (VT100 parser, cell model)
- **DO NOT create** terminal abstraction layer (engine-term stays as-is)
- **DO NOT rewrite** rendering pipeline (it works, uses engine-term types directly)
- **DO NOT remove** `parse_buffered_data` PTY I/O pipeline (refactor into session_engine, not delete)
- **Lua scripting** preserved (bridge refactored, not removed)
- **Lua user config** loading must remain functional throughout all phases

## Success Criteria
- `cargo check --workspace` passes after each phase
- Zero `#[deprecated]` markers added (clean removal, not deprecation)
- Chatminal native code > 50% of business-critical LOC
- No dual ID system (RuntimeId only, PaneId/TabId eliminated from public API)

## Estimated Timeline
- Phase 01: 1-2 days
- Phase 03: 5-7 weeks (includes 3G: PTY I/O pipeline migration)
- Phase 04: 3-4 weeks (elevated from Medium due to configuration() propagation depth)
- Phase 02: 2-3 days (deferred to last)
