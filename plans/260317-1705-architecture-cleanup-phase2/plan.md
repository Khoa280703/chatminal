---
title: "Architecture Cleanup Phase 2"
description: "Remove engine split fallback, dead tab.rs code, persist workspace layout, simplify window.rs, update docs"
status: completed
priority: P1
effort: 6h
branch: main
tags: [refactor, cleanup, persistence, architecture]
created: 2026-03-17
---

# Architecture Cleanup Phase 2

Continues from `260317-1443-architecture-redundancy-cleanup` (completed, ec6bc27).

## Phases

| # | Phase | Effort | Status | Blocker | Notes |
|---|-------|--------|--------|---------|-------|
| 1 | Remove engine split fallback | 30min | completed | -- | bail! + deleted 2 functions |
| 2 | Dead code removal in tab.rs | 1.5h | completed | Phase 1 | ~33 lines; tab.rs split code still live (lua-bridge) |
| 3 | Workspace layout persistence | 3h | completed | -- | Already existed in native_api.rs |
| 4 | Simplify window.rs | 1h | completed | Phase 2 | Doc comment added |
| 5 | Docs update | 15min | completed | Phase 1-4 | architecture-analysis.md updated |

## Key Decisions

- **Layout persistence**: JSON blob in `app_state` table (KISS over adjacency list)
- **Direction**: Daemon primary, Desktop deprecated (keep compiling)
- **tab.rs**: Remove split mutation code; keep pane iteration/focus/positioning for rendering
- **window.rs**: Mark as optional; rendering still depends on Window interface
