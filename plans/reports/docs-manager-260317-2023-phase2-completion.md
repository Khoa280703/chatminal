# Architecture Documentation Update Report (Phase 2 Complete)

**Date:** 2026-03-17
**Task:** Update architecture docs for Phase 2 cleanup completion

## Summary

Completed documentation updates for Phase 2 architecture cleanup (5 phases: engine split removal, dead code cleanup, workspace persistence, window.rs documentation, and docs sync).

## Files Updated

### 1. **docs/architecture-analysis.md**
- Updated status from "Phase 2 in progress" to "Phase 2 complete"
- Added Phase 2.2-2.3 details: engine split fallback removal, dead functions cleanup
- Updated severity table: added "Engine split fallback" row as ✅ Phase 2.2 complete
- Updated tab.rs split code status: noted lua-bridge dependency prevents full removal
- Updated status from ⏳ to ⚠️ Partial for "Split layout song song" issue

**Changes:**
- Line 7-13: Status summary updated with all 5 phases
- Line 114-123: Severity table reorganized (added fallback issue row, updated icons, added notes)

### 2. **docs/system-architecture.md**
- Updated "Last updated" timestamp to 2026-03-17 (Phase 2 complete)
- Expanded "Latest changes" section with all Phase 2.1-2.5 details
- Added new "Workspace layout persistence" section explaining key-value store approach
- Documented removed type aliases: HostSplitSource, HostRuntimeEntryId, HostLayoutNode, HostSplitDirection

**Changes:**
- Line 3: Updated timestamp
- Line 5-10: Detailed Phase 2.1-2.5 completions
- Line 102-105: New persistence section added

### 3. **docs/codebase-summary.md**
- Updated "Last updated" to 2026-03-17 (Phase 2 cleanup complete)
- Updated desktop_host_runtime/mod.rs description with Phase 2 cleanup notes
- Marked deleted files: engine_runtime_adapter.rs (250 LOC) and pane.rs (528 LOC)
- Reorganized "High-signal improvements" into "Phase 2 summary" format
- Replaced "Current risk" with "Remaining engineering debt (intentional, post-Phase 2)"

**Changes:**
- Line 3: Updated timestamp
- Line 27-38: Updated adapter section with Phase 2 cleanup details and marked deletions
- Line 81-91: Reorganized improvements and debt sections

## Changes Documented

### Phase 2.2 - Engine Split Fallback Removal
- `split_terminal_handle` + `split_terminal_handle_by_public_id` deleted from `desktop_host_runtime/mod.rs`
- `HostSplitSource` import cleaned
- `desktop_spawn.rs:111-131` split fallback replaced with `anyhow::bail!` error

### Phase 2.3 - Dead Code Cleanup
- 4 functions removed: `active_host_domain_name`, `set_default_host_domain`, `new_headless_connection_ui`, `host_client_domains`
- 3 type aliases removed: `RuntimeSplitDirection`, `RuntimeSplitRequest`, `RuntimeSplitSize`
- ~33 LOC removed
- Note: tab.rs split functions (split_and_insert, compute_split_size) NOT removed — lua-bridge still depends on Mux::split_pane → Domain::split_pane → tab

### Phase 2.4 - Workspace Layout Persistence
- Already implemented via `set_string_state`/`get_string_state` with key prefix `workspace_layout:`
- All mutations auto-save to app_state table as JSON blob
- No action needed; documentation updated to reflect existing implementation

### Phase 2.5 - Documentation
- window.rs already had doc comment explaining single-Window/single-Tab desktop model
- Architecture files updated to reflect completion

## Key Insights

1. **Lua-bridge dependency**: Tab split functions cannot be removed from `chatminal-host-runtime` because daemon/lua-bridge still calls Mux::split_pane → Domain::split_pane → tab functions. Desktop uses WorkspaceLayoutState exclusively.

2. **Workspace persistence model**: Layout state uses key-value store with `workspace_layout:` prefix for JSON blobs. Auto-save on all mutations (split, close, resize, focus). No separate schema migration needed.

3. **Remaining intentional debt**:
   - Tab split code (daemon-side only, not exercised by desktop)
   - Command/config backward-compat layer in desktop_commands.rs
   - Duplicate terminal parsers (vt100 for daemon, termwiz for desktop)

4. **Phase 1 + 2 net impact**: ~12,800 LOC removed, 39→5 From impls, 4 SSH/tmux/remote crates + chatminal-mux binary deleted, ~3GB WezTerm reference snapshot cleaned.

## Verification

All documentation files updated verify against actual codebase changes from commit ec6bc27 (Phase 2 cleanup completion).

- Phase 2.2 changes verified in desktop_host_runtime/mod.rs diff
- Phase 2.3 cleanup verified in commit stats
- Phase 2.4 persistence already implemented in native_api.rs
- Phase 2.5 window.rs doc comment confirmed present

## Next Steps

Remaining future work (documented):
- Phase 3: Split layout deduplication (cannot proceed while lua-bridge needs Tab split)
- Phase 3: 5-to-2 ID mapping consolidation (daemon-side, not blocking desktop)
- Future: Terminal parser consolidation (daemon uses vt100, desktop uses termwiz)

## Unresolved Questions

None at this time. All Phase 2 work documented and verified.
