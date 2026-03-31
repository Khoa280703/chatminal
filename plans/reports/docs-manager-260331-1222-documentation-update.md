# Documentation Update Report — 2026-03-31

## Summary
Successfully updated 8 core documentation files to reflect recent architectural changes: performance optimization phases 1-4, startup recipes implementation, single-process desktop model, and UI Shell work in progress.

## Files Updated

### 1. system-architecture.md (+17 lines, 163 total)
**Changes:**
- Added performance optimization phases 1-4 summary (SQLite reuse, async persist, SQL retention, resource caps)
- Documented RAM reduction: ~385MB → ~115MB (70%) for 5 sessions
- Added startup recipes overview (per-session, multi-step syntax)
- Replaced topology with single-process desktop model (no daemon)
- Clarified persist worker architecture (background thread, zero lock contention)

**Key additions:**
- Phase 1: SQLite connection reuse (Arc<Mutex<Connection>>)
- Phase 2: Async persist pipeline, coalescing
- Phase 3: SQL-based retention (window function)
- Phase 4: Resource caps (scrollback 10K→3K, OutputHistory 512KB)

### 2. codebase-summary.md (+13 lines, 111 total)
**Changes:**
- Updated runtime baseline: deleted daemon reference, emphasized single-process desktop
- Restructured runtime and execution core section with perf work details
- Added persist_worker.rs, session_event_processor.rs, startup_recipes.rs paths
- Added leaf_runtime.rs (PTY instance runtime) and output_history.rs (512KB cap)
- Clarified desktop facade layer

**Key updates:**
- Noted daemon deletion (2026-03-31)
- Added startup recipes registry path
- Detailed output history and live_output buffer management

### 3. development-roadmap.md (+50 lines, 275 total)
**Changes:**
- Reorganized "Active" section to "Active (2026-03-31)"
- Added performance optimization phases 1-4 (complete status)
- Added startup recipes (complete status)
- Added UI Shell work in progress (Phase 01-04 details, hard boundaries)
- Added single-process desktop (complete status)
- Added Arc<str> migration (complete status)
- New section: Upcoming work priorities

**Key status updates:**
- Phases 1-4 complete with 70% RAM reduction verified
- Startup recipes integrated, persisted in SQLite
- UI Shell Phase 01-04: Phase 03 done, Phase 01/02/04 partial (TODOs listed)
- daemon deleted, single-process now active

### 4. project-roadmap.md (+7 lines, 28 total)
**Changes:**
- Updated last modified date to 2026-03-31
- Expanded milestones table to include 2026-03-31 completions
- Added single-process desktop model completion date
- Added performance optimization phases 1-4 completion date
- Added startup recipes completion date
- Added canonical scrollback completion date
- Updated "Next priorities" section with concrete deliverables

**Key changes:**
- Single-process desktop: Completed (2026-03-31)
- Performance optimization phases 1-4: Completed (2026-03-31)
- Startup recipes: Completed (2026-03-31)
- Clarified upcoming deliverables (UI Shell completion, window parity, profiling)

### 5. code-standards.md (+8 lines, 44 total)
**Changes:**
- Updated date to 2026-03-31, changed scope to desktop-only
- Changed principles from daemon-first to desktop (single-process) as source of truth
- Updated boundaries section: replaced daemon/client references with desktop-embedded architecture
- Changed runtime rules to reflect single-process model (no IPC, no daemon lifecycle)
- Updated validation commands to use desktop app paths (chatminal-desktop instead of chatminald)

**Key shifts:**
- Daemon removed from boundaries entirely
- Startup recipes rules added (per-session, SQLite-persisted, pre-first-prompt execution)
- Desktop invariant clarified (single Window, single Tab, multiple Panes as sessions)
- Persist worker rule added (background thread, zero lock contention)

### 6. project-overview-pdr.md (minimal, 25 total)
**Changes:**
- Updated last modified date to 2026-03-31
- Changed product goal from "daemon-first" to "single-process desktop"
- Kept scope, non-goals, NFR intact (still valid)

**Key change:**
- Single sentence reframing (daemon-first → native desktop single-process)

### 7. deployment-guide.md (-40 lines, 54 total)
**Changes:**
- Updated date to 2026-03-31, retitled for single-process
- Removed daemon build target (chatminald)
- Removed IPC/endpoint transport sections (Unix socket, Named Pipe obsolete)
- Replaced build/run instructions with desktop-only commands
- Added vendor deps bootstrap section (first-time setup)
- Simplified environment variables (removed daemon endpoint, added UI sidebar toggle)
- Added performance tuning environment variables section

**Key removals:**
- Daemon build/run instructions
- Unix socket endpoint policy
- Windows Named Pipe policy
- Input/Window backend kill-switch sections (no longer relevant)

**New content:**
- Desktop-embedded runtime explanation
- Vendor deps bootstrap procedure
- Performance tuning options

### 8. index.md (+2 lines, 16 total)
**Changes:**
- Updated description to reflect single-process desktop + terminal-core + store
- Added [Architecture Audit](./architecture-audit.md) link with description

**Key addition:**
- Linked architecture-audit.md (resource footprint, tech debt, recommendations)

## Verification

### Compilation
```
cargo check --workspace: PASS
```

### File sizes
- All 8 files remain under 800 LOC target
- Total lines across updated docs: 716 LOC
- Largest: development-roadmap.md (275 lines)

### Cross-references verified
- All relative links valid (`./path.md` exists)
- `docs/architecture-audit.md` added to index (user-created, not touched)
- No broken internal references

## Completeness check

| Item | Status |
|---|---|
| Performance phases 1-4 documented | ✅ |
| Startup recipes documented | ✅ |
| Single-process desktop model clarified | ✅ |
| UI Shell work in progress noted | ✅ |
| Daemon deletion reflected | ✅ |
| Resource footprint updated (RAM -70%) | ✅ |
| Persist worker architecture documented | ✅ |
| Arc<str> migration noted | ✅ |
| deployment-guide.md updated (no daemon) | ✅ |
| architecture-audit.md added to index | ✅ |
| All links verified | ✅ |
| Token efficiency maintained | ✅ |

## Unresolved questions
None. All documentation updates complete and verified.

## Next steps (for project team)
1. Continue UI Shell Phase completion (GPU scissor, overlay bounds)
2. Expand test coverage for startup recipes and persist worker
3. Profile hot-path allocations (string clones, workspace_load caching)
4. Consider duplicate terminal parser unification (P3 tech debt)
