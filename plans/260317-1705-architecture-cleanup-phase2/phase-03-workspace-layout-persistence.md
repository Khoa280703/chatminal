# Phase 3: Workspace Layout Persistence

## Overview
- **Priority**: P1 (major feature gap)
- **Status**: pending
- **Effort**: 3h

`WorkspaceLayoutRegistry` in `workspace_layout.rs` is memory-only HashMap. Restart = lose all split layout. Need SQLite persistence.

## Key Insights

### Current State
- `WorkspaceLayoutState` already derives `Serialize, Deserialize` (serde)
- Recursive tree: nodes vec + views vec + root_node_id + active_view_id + counters
- `WorkspaceLayoutRegistry` wraps `HashMap<String, WorkspaceLayoutState>`
- Store (`chatminal-store`) already has `app_state` table (key-value TEXT)

### Persistence Strategy: JSON blob in app_state

**Why JSON blob over adjacency list / new table:**
- `WorkspaceLayoutState` already has serde derives
- Tree structure is small (typically <10 nodes)
- No need to query individual nodes from SQL
- KISS: 1 key-value row per workspace, serialize/deserialize via serde_json
- Key format: `workspace_layout:{workspace_id}`

**Alternatives considered & rejected:**
- New `workspace_layouts` table with JSON column — adds migration complexity for minimal benefit
- Adjacency list table — over-engineered for <10 node trees
- Nested set model — complex, read-optimized but we need fast writes too

### When to persist
- After every layout mutation (split_view, close_view, focus_view, attach_session, resize_split)
- On app shutdown (fallback)
- Debounce not needed (mutations are user-initiated, infrequent)

### When to load
- On startup, when `WorkspaceLayoutRegistry` initializes
- Load all `workspace_layout:*` keys from app_state

## Related Code Files

### Modify
- `crates/chatminal-store/src/lib.rs` — add `save_workspace_layout` / `load_workspace_layouts` / `delete_workspace_layout` methods
- `crates/chatminal-runtime/src/workspace_layout.rs` — add persist hooks to `WorkspaceLayoutRegistry` mutations

### New (none — extend existing files)

### Dependencies
- `serde_json` — check if already in workspace deps (likely yes via chatminal-protocol)

## Architecture

```
WorkspaceLayoutRegistry (chatminal-runtime)
  |-- mutate layout
  |-- serialize to JSON
  |-- call Store::save_workspace_layout(workspace_id, json)
       |-- UPSERT into app_state (key="workspace_layout:{id}", value=json)

On startup:
  Store::load_workspace_layouts()
  |-- SELECT * FROM app_state WHERE key LIKE 'workspace_layout:%'
  |-- deserialize each JSON -> WorkspaceLayoutState
  |-- populate WorkspaceLayoutRegistry.layouts HashMap
```

## Implementation Steps

1. **Store API** (`chatminal-store/src/lib.rs`):
   ```rust
   pub fn save_workspace_layout(&self, workspace_id: &str, json: &str) -> Result<(), String>
   pub fn load_workspace_layouts(&self) -> Result<Vec<(String, String)>, String>
   pub fn delete_workspace_layout(&self, workspace_id: &str) -> Result<(), String>
   ```
   - Use `app_state` table with key prefix `workspace_layout:`
   - `save_workspace_layout` → UPSERT
   - `load_workspace_layouts` → SELECT WHERE key LIKE 'workspace_layout:%'
   - `delete_workspace_layout` → DELETE

2. **Registry persistence bridge** (`chatminal-runtime/src/workspace_layout.rs`):
   - Add `WorkspaceLayoutRegistry::load_from_store(store: &Store)` — deserialize all layouts
   - Add `WorkspaceLayoutRegistry::persist_layout(store: &Store, workspace_id: &str)` — serialize + save one layout
   - Each mutation method that returns `Option<WorkspaceLayoutState>` should also accept optional `&Store` ref for auto-persist
   - OR: simpler — caller is responsible for calling persist after mutation (avoids coupling)

3. **Integration** — find where `WorkspaceLayoutRegistry` is initialized in runtime startup:
   - Load persisted layouts on init
   - After each mutation in `chatminal-runtime/src/state/runtime_bridge.rs` (likely), call persist

4. **Cleanup on workspace/session delete**:
   - When workspace removed → `delete_workspace_layout`

5. **Tests**:
   - Unit test: serialize/deserialize round-trip
   - Unit test: store save/load/delete
   - Integration: mutate layout, persist, reload, verify equality

## Success Criteria
- Layout survives app restart
- `cargo test --workspace` passes
- No migration needed (reuses existing `app_state` table)
- Graceful fallback if stored JSON is corrupt (log warning, start fresh)

## Risk Assessment
- **Low risk**: Additive feature, no existing behavior changes
- JSON blob could grow if user creates many workspaces — but practically bounded
- serde_json deserialization failure: handle gracefully, discard corrupt entry

## Security Considerations
- No sensitive data in layout (just node IDs, session IDs, split ratios)
- Session IDs are UUIDs, not secrets
