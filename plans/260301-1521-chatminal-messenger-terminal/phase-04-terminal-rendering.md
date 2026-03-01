# Phase 04 - Terminal Rendering

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-03-iced-ui-layout.md](phase-03-iced-ui-layout.md)
- Next: [phase-05-input-handling.md](phase-05-input-handling.md)
- Research: [Terminal Architecture](../reports/researcher-260301-1520-terminal-architecture.md)

## Overview
- **Priority:** P1
- **Status:** in-progress
- **Effort:** 6h
- **Goal:** Render TerminalGrid cell-by-cell on Iced Canvas using monospace font; correct colors, cursor, basic attrs

## Key Insights
- Iced `Canvas` widget: implement `canvas::Program<Message>` → `draw()` receives `Frame`
- `Frame::fill_text(Text { content, position, color, size, font, ... })` for each cell
- `iced::Font::with_name()` does NOT exist in 0.14 — embed JetBrains Mono TTF via `include_bytes!()`
- Font load is **async**: call `iced::font::load(bytes).map(Message::FontLoaded)` in `boot()` Task
- `Message::FontLoaded(Ok(()))` only signals load completion; terminal rendering is gated by `AppState.font_metrics.is_some()` after runtime metrics measurement
- Reference font in draw via `iced::Font { family: Family::Name("JetBrains Mono"), .. }`
- Cell size: measure font metrics once at startup (`char_width`, `char_height`) — fixed for monospace
- Post-fix: runtime metrics now derived from configured `font_size` via `metrics_for_font()`, no hardcoded magic metrics
- Glyph atlas NOT needed for MVP — `fill_text` per cell is acceptable at ≤200 cols × 50 rows = 10k draw calls
- Cursor: draw filled rectangle behind cell at cursor position using `Frame::fill_rectangle`
- Color mapping: `CellColor::Default` → theme default fg/bg; `Indexed(n)` → xterm-256 palette lookup; `Rgb` → direct
- Only redraw when grid snapshot changes (Iced redraws Canvas only when `cache` is invalidated)
- Use `canvas::Cache` to avoid re-rasterizing unchanged frames

## Requirements
- `TerminalCanvas` implements `canvas::Program<Message>`
- Render all visible cells in viewport (cols × rows)
- Correct fg/bg colors per cell (default, 256-color, truecolor)
- Bold text via font weight; underline via `fill_rectangle` under baseline
- Cursor rendered as blinking block (phase 06 adds blink timer if needed; MVP = solid block)
- Cell size derived from font metrics — not hardcoded magic numbers
- 256-color palette table (xterm standard) embedded as const array

## Architecture

```
// canvas::Program requires associated State type; cache MUST be in State (draw() takes &self)
struct TerminalCanvas {
    grid: Option<Arc<TerminalGrid>>,
    cell_width: f32,
    cell_height: f32,
    generation: u64,
    // NO cache here — would be recreated every frame in view()
}

// Generation counter pattern — CANONICAL cache invalidation approach:
// 1. app.update() on TerminalUpdated: bump terminal_canvas.generation += 1
// 2. canvas::Program::update() detects self.generation != state.last_generation → state.cache.clear() + sync
// 3. draw() just calls state.cache.draw(...) — cache was cleared in step 2
//
// WHY NOT clear from app.update() directly:
//   app.update() cannot reach TerminalCanvasState (owned by Iced canvas machinery, not AppState)
//   app.update() only owns TerminalCanvas struct (which has generation: u64)
//   generation bump signals canvas::Program::update() to do the actual clearing
struct TerminalCanvasState { cache: canvas::Cache, last_generation: u64 }
impl Default for TerminalCanvasState { fn default() -> Self { Self { cache: canvas::Cache::new(), last_generation: 0 } } }

// TerminalCanvas stored in AppState (persists between frames) — NOT recreated in view()

impl canvas::Program<Message> for TerminalCanvas {
    type State = TerminalCanvasState;
    fn draw(&self, state: &TerminalCanvasState, renderer, theme, bounds, cursor) -> Vec<Geometry> {
        state.cache.draw(renderer, bounds.size(), |frame| {
            // 1. Fill background
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), TERMINAL_BG);

            // 2. Per-cell rendering — ALWAYS use active_cells(), never grid.cells directly
            // active_cells() returns alternate_grid when use_alternate=true, else primary_grid
            for (row_idx, row) in grid.active_cells().iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    let x = col_idx as f32 * cell_width;
                    let y = row_idx as f32 * cell_height;

                    // Draw background
                    if cell.bg != CellColor::Default {
                        frame.fill_rectangle(..., bg_color);
                    }

                    // Draw cursor
                    if row_idx == cursor_row && col_idx == cursor_col {
                        frame.fill_rectangle(..., CURSOR_COLOR);
                    }

                    // Draw text
                    if cell.c != ' ' {
                        frame.fill_text(Text { content: cell.c, position, color: fg_color, ... });
                    }

                    // Underline
                    if cell.attrs.underline {
                        frame.fill_rectangle(underline_rect, fg_color);
                    }
                }
            }
        })
    }
}
```

## Related Code Files
- **Write:** `/home/khoa2807/working-sources/chatminal/src/ui/terminal_pane.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/ui/color_palette.rs` (new — 256-color table)
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/ui/mod.rs` (add color_palette mod)

## Implementation Steps

1. **`color_palette.rs`** — xterm 256-color lookup
   > **Note (F13):** Do NOT import `ansi_colours` crate — it is removed from Cargo.toml. Use only the static array below. This avoids a dead dependency that would fail `cargo clippy -D warnings`.
   ```rust
   pub const XTERM_PALETTE: [u32; 256] = [ /* 256 RGB hex values */ ];

   pub fn indexed_to_rgb(idx: u8) -> (u8, u8, u8) {
       let hex = XTERM_PALETTE[idx as usize];
       ((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
   }

   pub fn cell_color_to_iced(color: &CellColor, is_fg: bool) -> iced::Color {
       match color {
           CellColor::Default => if is_fg { Color::WHITE } else { Color::BLACK },
           CellColor::Indexed(n) => { let (r,g,b) = indexed_to_rgb(*n); Color::from_rgb8(r,g,b) },
           CellColor::Rgb(r,g,b) => Color::from_rgb8(*r,*g,*b),
       }
   }
   ```

2. **Font setup in `terminal_pane.rs`**
   - Embed JetBrains Mono or use system monospace
   - Constants: `FONT_SIZE: f32 = 14.0`
   - **`cell_width` and `cell_height` MUST be measured dynamically — hardcoding is NOT acceptable.**
     After `Message::FontLoaded(Ok(()))`, measure actual font advance width and line height using `iced::advanced::text::Paragraph` or platform font metrics. Store as `AppState.font_metrics: Option<(f32, f32)>` (cell_width, cell_height).
     **Gate ALL terminal rendering on `font_metrics` being `Some(...)`** — do not render terminal cells until metrics are measured.
     **Rationale:** Hardcoded 8.4px at 14pt does not account for HiDPI display scale factor. On 2× HiDPI: cols/rows reported to PTY would be double → bash wraps at wrong column. Font metrics must be measured at runtime after the font is loaded on the actual display.

3. **`TerminalCanvas` struct** — fields: `grid`, `cell_width`, `cell_height`, `generation: u64`
   - `TerminalCanvasState` — `cache: canvas::Cache`, `last_generation: u64`, implements `Default`
   - Store `TerminalCanvas` in `AppState` — bump `terminal_canvas.generation += 1` in `update()` on `TerminalUpdated`
   - Cache invalidation: `canvas::Program::update()` detects `self.generation != state.last_generation` → `state.cache.clear()` + sync generation
   - `draw()` then calls `state.cache.draw(...)` — cache already cleared, redraws fresh

4. **`draw()` implementation**
   - Step 1: fill background rect
   - Step 2: iterate cells, skip spaces with default bg (optimization)
   - Step 3: draw bg rect for non-default bg cells
   - Step 4: draw cursor rect at cursor position
   - Step 5: `fill_text` for non-space chars
   - Step 6: underline rect if attr set

5. **Cache invalidation** — in `app.rs update()` on `TerminalUpdated`:
   - `state.terminal_canvas.generation += 1` (bump counter only — cannot touch TerminalCanvasState from here)
   - Iced then calls `canvas::Program::update()` with new self → detects `self.generation != state.last_generation` → `state.cache.clear(); state.last_generation = self.generation;`
   - `draw()` then calls `state.cache.draw(...)` — cache is empty → redraws fresh

6. **Bold rendering** — pass `Font { weight: Weight::Bold, .. }` when `cell.attrs.bold`

7. **Verify:** run app, create session, see bash prompt rendered with correct colors

## Todo List
- [ ] `color_palette.rs`: XTERM_PALETTE const array (all 256 entries)
- [x] `color_palette.rs`: indexed_to_rgb(), cell_color_to_iced()
- [x] `terminal_pane.rs`: TerminalCanvas struct with `generation` field + `TerminalCanvasState { cache, last_generation }`
- [x] `terminal_pane.rs`: canvas::Program impl — background fill
- [x] `terminal_pane.rs`: per-cell bg rect rendering
- [x] `terminal_pane.rs`: per-cell text rendering with color + bold font
- [x] `terminal_pane.rs`: cursor block rendering
- [x] `terminal_pane.rs`: underline attr rendering
- [x] `terminal_pane.rs`: generation-based cache invalidation (`generation` bump on update, `state.cache.clear()` in `canvas::Program::update`)
- [x] `app.rs` + `theme.rs`: runtime terminal metrics from `font_size` (`metrics_for_font`), hardcoded metrics removed
- [ ] Manual test: bash prompt renders with correct green/white colors

## Success Criteria
- Bash prompt visible in terminal pane
- Colors match expected ANSI output (green `$` prompt, white text)
- Bold text renders bolder
- Cursor block visible at correct position
- No flickering on static output (cache working)

## Risk Assessment
- **`fill_text` performance** at 10k calls/frame may be slow — benchmark; if >16ms consider batching or glyph atlas (Phase 07 optimization)
- **Font metrics** — hardcoding `cell_width/cell_height` is NOT acceptable; must measure runtime on actual DPI scale or cols/rows sent to PTY will drift
- **`canvas::Cache` API** — verify Iced 0.14 cache invalidation method name
- **Bold font fallback** — system may not have bold variant of chosen monospace; embed both weights

## Security Considerations
- Cell content is raw chars from PTY — ensure no HTML/script injection (not applicable for Canvas renderer, but relevant if ever switching to WebView-based render)
