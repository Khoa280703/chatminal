# Phase 01: Sidebar Render Cache

## Overview
- **Priority:** HIGH — biggest visible perf win
- **Status:** pending
- **Effort:** Medium (~2-3 hours)

## Problem

Sidebar rebuilds nhiều `ComputedElement` mỗi frame dù data không đổi:
- static labels vẫn `.to_string()` / `format!()`
- nhiều `Vec::new()` cho children/tabs/footer parts
- session/profile data bị clone lặp lại trong row builders
- render path hiện tách thành background, header, tree, tooltip, context menu, scrollbar, footer, divider

## Key Insights

- `paint_chatminal_sidebar()` chỉ là orchestration; không nên cache nguyên khối toàn bộ function result
- `ChatminalSidebar` đã có `snapshot.version` / `ChatminalSidebar::version()` để làm cache key
- Hover tooltip, session context menu, scrollbar thumb, divider là dynamic overlay, không nên nhét chung vào cache
- Tree body mới là subtree ổn định và đắt nhất
- Scroll hiện được áp qua `translate(...)`, nên có thể reuse cached tree rồi translate/clip theo frame

## Related Code Files

### Modify
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs` — main sidebar render
- `apps/chatminal-desktop/src/termwindow/mod.rs` — only if cache storage really needs to live on `TermWindow`

### Read-only references
- `apps/chatminal-desktop/src/termwindow/render/paint.rs` — where sidebar paint is called
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs` — sidebar state, snapshot versioning

## Implementation Steps

### Step 1: Add versioned cache key, not global dirty flag

Prefer a compact cache key derived from existing state:
```rust
struct SidebarCacheKey {
    sidebar_version: u64,
    sidebar_width_px: u32,
    sidebar_height_px: u32,
    footer_height_px: u32,
    dpi: u32,
}
```

Use `self.chatminal_sidebar.version()` plus shell bounds. Do not add a second hand-maintained invalidation system unless absolutely needed.

### Step 2: Cache only stable sidebar subtrees

Cache candidates:
- root hit-area element
- header
- tree body before scroll translation / clip
- footer background only

Do not cache:
- tooltip
- session context menu
- scrollbar thumb
- divider/background fill primitives
- footer content that includes realtime system metrics

### Step 3: Reuse cached tree with per-frame scroll transform

Pattern:
```rust
fn build_cached_sidebar_tree(&mut self, key: SidebarCacheKey, ...) -> anyhow::Result<ComputedElement> {
    if let Some(cached) = self.sidebar_tree_cache.get(&key) {
        return Ok(cached.clone());
    }

    let computed = self.build_chatminal_sidebar_tree_uncached(...)?
    self.sidebar_tree_cache.put(key, computed.clone());
    Ok(computed)
}
```

Then per frame:
- fetch cached tree
- clone lightweight render copy
- apply `translate(...)` using current scroll offset
- render with current clip rect

If `ComputedElement` cloning itself is still too expensive, fallback to caching the intermediate row model first.

### Step 3.5: Keep footer content on dynamic path

`build_chatminal_terminal_footer_content()` currently pulls realtime values from `self.system_metrics.snapshot()`, so footer text must remain uncached unless it gets a dedicated metrics-aware cache key.

Safe default:
- cache footer background
- rebuild footer content each frame
- revisit only after measurement proves footer text build is material

### Step 4: Static string optimization

Replace heap-allocated static strings where API allows:
```rust
ElementContent::Text(Cow::Borrowed("Profiles"))
ElementContent::Text(Cow::Borrowed("Sessions"))
```

If `ElementContent` still needs owned text at some call sites, at least remove redundant `format!()` for constant labels.

### Step 5: Vec pre-allocation

Replace repeated `Vec::new()` with realistic capacities:
```rust
let mut children = Vec::with_capacity(16);
let mut footer_parts = Vec::with_capacity(4);
```

### Step 6: Reduce session/profile clone churn in row builders

Keep borrowed references as long as possible:
```rust
let session_id = session.session_id.as_str();
```

Clone only on ownership boundaries such as UI action payload creation.

## Todo

- [ ] Define `SidebarCacheKey` from sidebar version + bounds + dpi
- [ ] Add caches only for stable sidebar subtrees
- [ ] Keep tooltip/context menu/scrollbar/divider on uncached path
- [ ] Keep realtime footer content on uncached path
- [ ] Reuse cached tree with per-frame scroll translation and clipping
- [ ] Replace static `.to_string()` with borrowed constants where possible
- [ ] Add `Vec::with_capacity()` hints
- [ ] Reduce session/profile clone churn in sidebar row builders

## Success Criteria

- `cargo check -p chatminal-desktop` — 0 errors
- Sidebar renders correctly: scroll, expand/collapse, profile switch, session create/delete
- Tooltip, context menu, hover, selection, drag/drop, footer visibility still update immediately
- Realtime footer metrics continue updating without waiting for sidebar version changes
- Idle frame: sidebar stable subtrees hit cache
- Active interaction: rebuild only affected subtrees

## Risk Assessment

- **Medium risk:** Caching whole sidebar too aggressively can stale hover/menu/scroll state. Mitigation: cache only stable subtrees.
- **Medium risk:** Footer text includes realtime metrics and active session/profile labels. Mitigation: keep footer content uncached in first pass.
- **Medium risk:** `ComputedElement` cloning may still be non-trivial. Mitigation: fallback to caching snapshot-derived row model instead of final computed tree.

## Verify

```bash
cargo check -p chatminal-desktop
# Visual test: open app, interact with sidebar, verify no stale rendering
```
