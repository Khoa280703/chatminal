# Chatminal Desktop Render Pipeline Exploration

**Exploration Date:** 2026-03-30  
**Focus:** Performance optimization planning for render pipeline

## Summary

The render pipeline is **event-driven** with **VSync frame rate limiting** (Fifo present mode). Key performance bottlenecks identified:

1. **Sidebar rebuilds every frame** with excessive string clones and no dirty tracking
2. **Bind groups & samplers recreated per-frame** instead of cached (WebGPU/Glium)
3. **No centralized invalidation strategy** - 50+ scattered `window.invalidate()` calls
4. **Per-cluster cloning** in screen line rendering (CellAttributes, glyph_info)
5. **No quad allocation optimization** - continuous allocation per render pass

---

## 1. Paint Loop & Frame Rate Tracking

### Location
`/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/paint.rs`

### Frame Counter & Timing
- **Lines 17-144:** `paint_impl()` - main paint entry point
  - Line 18: `self.num_frames += 1` (increments every frame)
  - Lines 28-35: FPS calculation every 1 second window
  - Line 109: `self.last_frame_duration = start.elapsed()` records total paint time
  - Lines 115-116: Metrics histogram recorded for telemetry

### Paint Pass Loop
- **Lines 37-105:** Multi-pass rendering
  - Retries on quad allocation failures
  - Lines 44-45, 67-68, 94-95: Invalidates tab bar & modal on texture atlas resize
  - No explicit frame budget or throttling

### Animation Scheduling (Deferred Invalidation)
- **Lines 118-143:** Handles animated content (blinking text, GIFs)
  - `has_animation` RefCell tracks next frame due time
  - Uses `Timer::at()` + `TermWindowNotif::Apply` for deferred repaint
  - Only invalidates when animation frame is actually due

### Frame Rate Control
- **NO explicit rate limiting** - relies on GPU driver
- WebGPU configured with `Fifo` present mode (VSync)
- Effectively 60 FPS on 60Hz monitor, 120 FPS on 120Hz, etc.

---

## 2. Line Rendering & Clone Patterns

### Location
`/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/screen_line.rs`

### Cloning Hotspots

| Line | Pattern | Frequency | Impact |
|------|---------|-----------|--------|
| 733 | `line.clone()` | Per line with composition | Full line deep copy |
| 875 | `cluster.clone()` | Per cluster in shape | CellAttributes + glyph metadata |
| 848 | `underline_tex_rect.clone()` | Per style change | Texture rect copy |
| 855 | `style_params.clone()` | Per cluster | HSB, colors, texture rect |
| 437, 672 | `images.clone()` (implicit via iter) | Per glyph cluster | Vector of image attachments |

### Line Element Shaping (Lines 719-902)
```rust
// Line 719-902: build_line_element_shape() 
// Main cost driver for text rendering

- Line 733: line.clone() for composition overlay
- Line 736: line.cluster(bidi_hint) - clusters cells by glyph
- Line 741: Vec::new() allocates shaped output vector
- Line 742-881: For each cluster:
    - Line 748-853: Recompute style (fonts, colors, underline)
    - Line 857: cached_cluster_shape() - glyph shaping
    - Line 875: Stores cluster.clone() in result
- Line 883: Rc::new(shaped) wraps result for caching
- Line 885-898: Caches in line_to_ele_shape_cache
```

### Quad Allocation (No Pooling)
- **Line 265:** `allocate(0)` for underlines (per cell per glyph)
- **Line 356-358:** `allocate(cursor_layer)` for cursor quads
- **Line 642:** `allocate(1)` for glyph textures
- Fresh allocation each frame, no reuse pool

---

## 3. Draw Call Batching & Uniform Setup

### Location
`/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/draw.rs`

### WebGPU Path (Lines 20-150)
```rust
// call_draw_webgpu()

Lines 26-28:   output = get_current_texture()  // Fresh each frame
Lines 30-34:   encoder = create_command_encoder()
Lines 39-69:   Creates 2 bind groups (texture_linear, texture_nearest)
               **RECREATED EVERY FRAME** - expensive operation
Lines 79-88:   Orthographic projection computed fresh each frame
Lines 90-142:  For each layer.borrow().iter():
               - For idx in 0..3 (3 vertex buffers per layer):
                 - Lines 123-127: Creates uniform struct fresh
                 - Lines 129-132: Sets bind groups
                 - Lines 134-138: Sets vertex buffer + indices
                 - Line 138: draw_indexed() single call
Line 146:      queue.submit() - single command buffer
Line 147:      output.present() - VSync here
```

**Optimization Gap:** Bind groups & uniforms could be cached/reused.

### Glium Path (Lines 152-276)
```rust
// call_draw_glium()

Lines 214-222: Creates atlas_nearest_sampler + atlas_linear_sampler fresh
               **RECREATED EVERY FRAME**
Lines 233-235: ColorEase uniforms (cursor_blink, blink, rapid_blink)
               computed fresh from state each frame
Lines 237-272: For each layer, for idx in 0..3:
               - Lines 245-255: UniformBuilder.add() builds uniform map
               - Line 257-267: frame.draw() with uniforms
               - Line 270: vb.next_index()
```

**Observation:** Both backends recreate sampler objects every frame (expensive GPU state setup).

### Batching Assessment
- **Single submission per frame** (Fifo present mode)
- **No draw call merging** across layers
- **Uniforms/samplers not reused**

---

## 4. Sidebar - Vec & String Allocations

### Location
`/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`

### Every-Frame Allocations (Lines 180-212: paint_chatminal_sidebar)
```rust
Line 186: sidebar_background = build_chatminal_sidebar(&bounds)?
Line 187: sidebar_header = build_chatminal_sidebar_header(&bounds)?
Line 188: sidebar_tree = build_chatminal_sidebar_tree(&bounds)?  // EXPENSIVE
Line 189: footer_background = build_chatminal_sidebar_terminal_footer_background(&bounds)?
Line 190: footer_content = build_chatminal_sidebar_terminal_footer_content(&bounds)?
Line 191: sidebar_tooltip = build_chatminal_sidebar_header_tooltip()?
Line 192: sidebar_context_menu = build_chatminal_sidebar_session_context_menu(&bounds)?
```

**No dirty tracking** - rebuilds entire tree every single frame.

### Tree Building Cost (Lines 369-464: build_chatminal_sidebar_tree)
```rust
Line 376: snapshot = ordered_sidebar_snapshot(self, self.chatminal_sidebar.snapshot())
          // Clones snapshot from sidebar state

Line 406-407: tree_rows = sidebar_tree_rows(self, &snapshot)
             // Rebuilds entire row vector structure

Line 411-423: tree_children = build_chatminal_sidebar_tree_row_elements(
               &tree_rows, ... many args ...)
             // Maps rows to Element objects
```

### String Clone Hotspots

| Location | Clone | Count | Context |
|----------|-------|-------|---------|
| 729, 735, 740, 744, 750, 756 | `session.session_id.clone()` | 6x | Menu item UIItemType creation |
| 832 | `message.clone()` | 1x | Error display text |
| 937, 945 | `profile.name.clone()`, `.profile_id.clone()` | 2x | Profile row rendering |
| 1045 | `session.name.clone()` | 1x | Session row label (+ "_" suffix in edit mode) |
| 1174 | `session.session_id.clone()` | 1x | UIItemType::ChatminalSidebarSession |
| 1225 | `session.session_id.clone()` | 1x | Chrome tab building |
| 1381, 1387, 1395 | `.map(\|p\| p.name.clone())` etc | 3x | Footer content (active profile/session) |
| 1480, 1492, 1508 | Error/profile/session `.clone()` | 3x | Tree row construction |

**Total: 15+ string allocations per frame in sidebar alone**

### Session ID Reuse Pattern
```rust
// Lines 1475-1514: sidebar_tree_rows()
fn sidebar_tree_rows(...) -> Vec<SidebarTreeRow> {
    let mut rows = Vec::new();  // Line 1486 - allocates vector
    for profile in &snapshot.profiles {
        let is_expanded = term_window.chatminal_sidebar
            .is_profile_expanded(&profile.profile_id);
        rows.push(SidebarTreeRow::Profile {
            profile: profile.clone(),  // Line 1492 - CLONES profile
            is_expanded,
        });
        
        if is_expanded {
            for session in profile_sessions {
                rows.push(SidebarTreeRow::Session(session.clone()));  // Line 1508
            }
        }
    }
    rows
}
```

**Issue:** Entire profile/session objects cloned, not just IDs.

---

## 5. Invalidation Trigger Points & Dirty Tracking

### Explicit Cache Invalidation (paint.rs)
- **Lines 44-45:** `invalidate_fancy_tab_bar()` + `invalidate_modal()` on quad allocation
- **Lines 67-68:** Same on texture atlas recreation
- **Lines 94-95:** Same on shape cache clear
- **Line 136:** Deferred invalidation from animation timer

### invalidate_fancy_tab_bar() (Line 54-56)
```rust
pub fn invalidate_fancy_tab_bar(&mut self) {
    self.fancy_tab_bar.take();  // Clears cached ComputedElement
}
```

Simple cache clear, no partial invalidation.

### window.invalidate() Calls (Scattered across codebase)

**Keyboard input** (`keyevent.rs`): Lines 231, 251, 261, 266, 272, 369, 431, 537, 735, 870, 888, 904

**Core logic** (`mod.rs`): Lines 717, 760, 989, 1168, 1308, 1322, 1392, 1413, 1446, 1467, 1517, 1544, 1607, 2017, 2057, 2092, 2211-2212, 2227, 2340, 2376, 2503

**Palette changes** (`palette.rs`): Lines 809, 813, 892, 916, 935

**Problem:** 50+ scattered invalidation points with no centralized strategy. Hard to distinguish:
- Full redraw vs partial update
- What changed (sidebar only? terminal? both?)
- Optimization opportunities

### Animation-Driven Invalidation (paint.rs Lines 121-142)
```rust
if self.focused.is_some() {
    if let Some(next_due) = *self.has_animation.borrow() {
        let prior = self.scheduled_animation.borrow_mut().take();
        match prior {
            Some(prior) if prior <= next_due => {
                // Already scheduled earlier, skip
            }
            _ => {
                self.scheduled_animation.borrow_mut().replace(next_due);
                let window = self.window.clone().take().unwrap();
                promise::spawn::spawn(async move {
                    Timer::at(next_due).await;  // Wait until frame time
                    let win = window.clone();
                    window.notify(TermWindowNotif::Apply(Box::new(move |tw| {
                        tw.scheduled_animation.borrow_mut().take();
                        win.invalidate();  // Trigger redraw at exact time
                    })));
                })
                .detach();
            }
        }
    }
}
```

**Good:** Only invalidates when animation frame is actually due, not every frame.

---

## 6. VSync & Frame Rate Control

### Location
`/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/webgpu.rs`

### Present Mode Configuration
```rust
// In webgpu surface initialization:
present_mode: wgpu::PresentMode::Fifo
```

**Fifo behavior:**
- Waits for vertical blank to present frame
- 60 FPS on 60Hz monitor
- No tearing
- No explicit frame rate limiting (relies on monitor refresh)

### No swap_interval Equivalent
- wgpu abstracts away platform-specific VSync
- Cannot control per-frame in WebGPU (driver decision)
- Fifo is the VSync-locked option

### Frame Timing
- No frame budget enforcement
- No throttling for low-end devices
- Entirely GPU/monitor driven

---

## 7. Fancy Tab Bar - Clone Patterns

### Location
`/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/fancy_tab_bar.rs`

### Invalidation (Lines 54-56)
```rust
pub fn invalidate_fancy_tab_bar(&mut self) {
    self.fancy_tab_bar.take();
}
```

Simple: stores `Option<ComputedElement>`, clearing it forces rebuild next paint.

### Tab Bar Building (Lines 58-509: build_fancy_tab_bar)
```rust
// Line 67: Items pulled from tab_bar state
let items = self.tab_bar.items();

// Lines 97-309: Closure converts each item to Element
let item_to_elem = |item: &SessionBarEntry| -> Element {
    let element = Element::with_line(&font, &item.title, palette);
    // ... pattern matching on item type ...
};

// Lines 311-326: Count operations (no cache)
let num_tabs: f32 = items.iter().map(|item| { ... }).sum();
let session_item_count: f32 = items.iter().map(|item| { ... }).sum();

// Lines 363-422: Converts items to elements
for item in items {
    match item.item {
        SessionBarItem::RuntimeEntry { entry_idx, active } => {
            let mut elem = item_to_elem(item);
            elem.max_width = Some(Dimension::Pixels(max_tab_width));
            elem.min_width = Some(Dimension::Pixels(max_tab_width));
            if self.config.show_close_tab_button_in_tabs {
                // Line 388-389: Allocates X button element
                kids.push(make_x_button(&font, &metrics, &colors, entry_idx, active));
            }
            left_eles.push(elem);
        }
        // ... similar for sessions, buttons, etc ...
    }
}

// Lines 424-475: Builds final element tree
let mut children = vec![];
children.push(Element::new(&font, ElementContent::Children(left_status)));
children.push(Element::new(&font, ElementContent::Children(left_eles)));
children.push(Element::new(&font, ElementContent::Children(right_eles)));

let tabs = Element::new(&font, ElementContent::Children(children));
let mut computed = self.compute_element(..., &tabs)?;
computed.translate(euclid::vec2(0., sb.session_bar_y));
```

**Cost:** Per-item Element allocation, no caching of unchanged entries.

---

## 8. Render Loop Trigger - Event-Driven Architecture

### Invalidation-Based Triggering
The render loop is **event-driven**, not polling:
- `window.invalidate()` requests a redraw
- Window manager queues repaint event
- Next available event loop cycle calls paint_impl()

### Paint Entry Points
**Line 2176** (`mod.rs` - Glium path):
```rust
pub fn paint_impl(&mut self, frame: &mut RenderFrame)
// Called from window resize/event handler
self.paint_impl(&mut RenderFrame::Glium(&mut frame));
```

**Line 2198** (`mod.rs` - WebGPU path):
```rust
pub fn paint_impl(&mut self, frame: &mut RenderFrame)
self.paint_impl(&mut RenderFrame::WebGpu);
```

### Invalidation Sources

**1. Keyboard Input** (keyevent.rs)
- Text input → invalidate
- Keyboard shortcuts (split, nav) → invalidate
- Multiple call sites (15+)

**2. Mouse Input** (mouseevent.rs)
- Hover/selection changes
- Window resizing
- Pane dragging

**3. State Changes** (mod.rs, palette.rs)
- Cursor blink changes
- Color palette updates
- Configuration changes
- Palette hot-reload
- Theme changes

**4. Terminal Output** (implicit via term_data)
- Pane content updates trigger invalidation
- Search highlights
- Selection changes

**5. Animations** (paint.rs Lines 121-142)
- Blink rate updates
- GIF frame advances
- Uses deferred Timer-based invalidation

### TermWindowNotif - Deferred Calls
**Lines 151-175** (mod.rs enum):
```rust
pub enum TermWindowNotif {
    InvalidateShapeCache,
    PerformAssignmentForTerminalHandle { ... },
    SetRightStatus(String),
    SetLeftStatus(String),
    // ... many more variants ...
    Apply(Box<dyn FnOnce(&mut TermWindow) + Send + Sync>),
    RuntimeNotification(RuntimeNotification),
}
```

**Apply variant** used for deferred operations:
- Animation timer fires → sends Apply notification
- Custom closure executes → calls `window.invalidate()`
- Queues repaint for next event cycle

**Dispatch** (Lines 2202-2316 mod.rs):
```rust
fn dispatch_notif(&mut self, notif: TermWindowNotif, window: &Window) {
    match notif {
        TermWindowNotif::Apply(closure) => {
            closure(self);  // Executes deferred work
        }
        // ... handle other variants ...
    }
}
```

### No Polling or Frame Budgeting
- Render loop entirely reactive to events
- No continuous frame rendering (unless events keep coming)
- Animation frame scheduling is smart: only repaints at animation time

---

## Findings Summary

### Strengths
1. **Event-driven rendering** - no wasted frames when idle
2. **Smart animation scheduling** - only repaints when animated content needs update
3. **Multi-pass resilience** - handles quad allocation overflow gracefully
4. **VSync by default** - tearing-free, power-efficient

### Performance Bottlenecks
1. **Sidebar full rebuild every frame** (no dirty tracking)
   - 50+ session/profile string clones per frame
   - Entire tree re-laid-out even when unchanged
   
2. **Bind groups & samplers recreated every frame** (WebGPU + Glium)
   - Expensive GPU state setup
   - No caching across frames

3. **Line rendering clones per cluster**
   - `cluster.clone()`, `style_params.clone()`
   - Could reuse shaped cache entries

4. **Quad allocation has no pooling**
   - Fresh allocation for each glyph/underline/cursor
   - No reuse of allocated quads

5. **No centralized invalidation strategy**
   - 50+ `window.invalidate()` calls scattered
   - Hard to reason about which changes warrant repaints
   - Difficult to implement partial updates

### Optimization Opportunities (for planning)
1. Sidebar dirty tracking system (track active profile/session changes)
2. Sampler/bind group caching (store in render_state, reuse across frames)
3. Partial invalidation regions (sidebar-only, terminal-only)
4. Cluster shape cache better integration (avoid re-cloning)
5. Quad allocation pool with reuse
6. Frame budget for low-end devices (throttle if paint takes >16ms)

