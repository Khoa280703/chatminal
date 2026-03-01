# Phase 06 - Virtual Scrolling

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-04-terminal-rendering.md](phase-04-terminal-rendering.md)
- Next: [phase-07-integration-polish.md](phase-07-integration-polish.md)
- Research: [Terminal Architecture](../reports/researcher-260301-1520-terminal-architecture.md)

## Overview
- **Priority:** P2
- **Status:** in-progress
- **Effort:** 3h
- **Goal:** Terminal scrollback (mouse wheel / Shift+PgUp/Down); sidebar virtual scroll for large session lists

## Key Insights
- Terminal scrollback: `TerminalGrid.scrollback: VecDeque<Vec<Cell>>` already defined in Phase 02
- Scroll offset = how many lines above current viewport; `scroll_offset: usize` per session in `AppState`
- Render: if `scroll_offset > 0`, take rows from scrollback + partial viewport; else render viewport normally
- If active grid is alternate screen (`use_alternate == true`), disable scrollback view: force `scroll_offset = 0` and render only `active_cells()`
- Mouse wheel events in Iced Canvas: `canvas::Program::update()` receives `canvas::Event::Mouse(MouseEvent::WheelScrolled { delta })`
- Sidebar virtual scroll: Iced `scrollable()` widget handles this natively — no custom impl needed
- Sidebar has at most ~100 sessions in realistic use; native scrollable is sufficient
- Scroll speed: 3 lines per wheel tick (hardcoded constant for MVP; can be made configurable later)

## Requirements
- `AppState` stores `scroll_offsets: HashMap<SessionId, usize>` — **offset = lines from BOTTOM** (not top)
- Mouse wheel up/down adjusts `scroll_offset` for active session
- Shift+PageUp / Shift+PageDown scroll by full viewport height
- When PTY outputs new data AND `scroll_offset > 0`: **increment offset by `lines_added`** from `SessionEvent::Update { lines_added }` to anchor view position (prevents scroll drift)
- When `scroll_offset == 0`: auto-scroll to bottom on new PTY output
- Maximum scrollback: 10,000 lines per session (defined in `config.rs`)
- Sidebar: use Iced native `scrollable()` — no custom impl
- Alternate screen safety: while `grid.use_alternate == true`, `ScrollTerminal` has no effect (offset fixed at 0) and no scrollback rows are rendered

## Architecture

```
AppState fields added:
  scroll_offsets: HashMap<SessionId, usize>,

Message variants added:
  ScrollTerminal { delta: i32 },   // positive = scroll up (toward history)
  ScrollToBottom,

// canvas::Program::update() Iced 0.14 correct signature:
// fn update(&self, state: &mut Self::State, event: &Event, bounds: Rectangle, cursor: Cursor) -> Option<Action<Message>>
canvas::Program::update():
  Event::Mouse(MouseEvent::WheelScrolled { delta }) =>
    let lines = match delta {
        ScrollDelta::Lines { y, .. } => (-y * 3.0) as i32,
        ScrollDelta::Pixels { y, .. } => (-y / cell_height) as i32,
    };
    // Return Action, NOT shell.publish() — shell param does NOT exist:
    return Some(canvas::Action::publish(Message::ScrollTerminal { delta: lines }))

update() match arm ScrollTerminal { delta }:
  let offset = scroll_offsets.entry(active_id).or_insert(0);
  let max_offset = if grid.use_alternate { 0 } else { grid.scrollback.len() };
  *offset = (*offset as i32 + delta).clamp(0, max_offset as i32) as usize;
  // invalidate canvas cache

Rendering with scroll offset (terminal_pane.rs):
  let visible_rows = grid.rows;
  if grid.use_alternate {
      // alternate screen has no scrollback view
      // render only grid.active_cells() and ignore scroll_offset
  } else {
  let scrollback_len = grid.scrollback.len();
  if scroll_offset == 0 {
      // render active buffer rows (primary/alternate decided by active_cells())
  } else {
      // take from scrollback + partial grid
      let start = scrollback_len.saturating_sub(scroll_offset);
      let history_rows: Vec<&Vec<Cell>> = grid.scrollback
          .iter().skip(start).take(visible_rows).collect();
      // render history_rows, pad with grid rows if history < visible_rows
  }
  }

// Shift+PageUp/Down: comes via iced::event::listen() → KeyboardEvent → handle_keyboard() in app.rs
// NOT from canvas::Program::update() — wrong context for canvas::Action::publish()
Shift+PageUp/Down in handle_keyboard() in app.rs:
  if modifiers.shift() {
      match key {
          Key::Named(Named::PageUp) =>
              return Task::done(Message::ScrollTerminal { delta: visible_rows as i32 }),
          Key::Named(Named::PageDown) =>
              return Task::done(Message::ScrollTerminal { delta: -(visible_rows as i32) }),
          _ => {}
      }
  }
```

## Related Code Files
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/app.rs` (scroll_offsets field, scroll message arms)
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/ui/terminal_pane.rs` (canvas mouse event, scroll-aware render)
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/app.rs` (Shift+PgUp/Down in handle_keyboard)
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/config.rs` (SCROLLBACK_MAX_LINES const)

## Implementation Steps

1. **`config.rs`** — add scrollback config
   ```rust
   pub const SCROLLBACK_MAX_LINES: usize = 10_000;
   pub const SCROLL_LINES_PER_TICK: usize = 3;
   ```

2. **`app.rs`** — add `scroll_offsets: HashMap<SessionId, usize>` to `AppState`
   - `ScrollTerminal { delta }` arm: clamp offset to `[0, grid.scrollback.len()]`
     - if `active_grid.use_alternate`: force `offset = 0` (ignore delta)
   - `TerminalUpdated` arm:
     - if `scroll_offset == 0`: keep at bottom (no change)
     - if `scroll_offset > 0`: **add `event.lines_added` to offset** to anchor view (prevent drift). `lines_added` is carried in `SessionEvent::Update { lines_added }` from `PtyPerformer` which tracks `scroll_up()` call count since last flush.
   - `SelectSession` arm: reset `scroll_offset` to 0 for newly selected session (go to bottom)

3. **`terminal_pane.rs`** — scroll-aware rendering
   - Accept `scroll_offset: usize` parameter in `terminal_pane_view()`
   - In `draw()`: build `render_rows: Vec<&Vec<Cell>>` based on offset — **must be O(height), not O(history)**
     ```rust
     fn build_render_rows<'a>(
         grid: &'a TerminalGrid,
         offset: usize,      // 0 = live view (bottom)
         visible_rows: usize,
     ) -> Vec<&'a Vec<Cell>> {
         if offset == 0 {
             // Fast path: only viewport rows needed
             grid.active_cells().iter().take(visible_rows).collect()
         } else {
             // scrollback path: start from (len - offset), take AT MOST visible_rows
             let sb_len = grid.scrollback.len();
             let start = sb_len.saturating_sub(offset);
             let from_sb: Vec<_> = grid.scrollback
                 .iter()
                 .skip(start)
                 .take(visible_rows)  // ← CRITICAL: O(height), not O(history)
                 .collect();
             if from_sb.len() < visible_rows {
                 // fill remaining rows from live grid top
                 let remaining = visible_rows - from_sb.len();
                 let from_grid: Vec<_> = grid.active_cells().iter().take(remaining).collect();
                 from_sb.into_iter().chain(from_grid).collect()
             } else {
                 from_sb
             }
         }
     }
     ```
  - Render `render_rows` instead of direct `grid.cells` access (keep alternate-screen correctness via `active_cells()`)

4. **`terminal_pane.rs`** — canvas mouse event handler
   - Implement `canvas::Program::update()` to catch `WheelScrolled`
   - Return `Some(canvas::Action::publish(Message::ScrollTerminal { delta }))` — no `shell` param

5. **`app.rs`** — Shift+PageUp/Down in `handle_keyboard()`
   - Intercept before forwarding to PTY
   - Return `Task::done(Message::ScrollTerminal { delta: ±visible_rows })`

6. **Scroll indicator** (optional, MVP-tier): render a thin scrollbar rect on right edge of terminal pane
   - `scroll_fraction = scroll_offset as f32 / scrollback_len as f32`
   - Draw 4px wide rect on right edge; height proportional to viewport/total

7. **Verify:** run `seq 1 500` in terminal, scroll up with wheel, verify history visible

## Todo List
- [x] `config.rs`: SCROLLBACK_MAX_LINES, SCROLL_LINES_PER_TICK constants
- [x] `app.rs`: scroll_offsets HashMap field
- [x] `app.rs`: ScrollTerminal message arm with clamp logic
- [x] `app.rs`: reset scroll_offset on SelectSession
- [x] `terminal_pane.rs`: build_render_rows() helper
- [x] `terminal_pane.rs`: use render_rows in draw()
- [x] `terminal_pane.rs`: canvas::Program::update() for WheelScrolled
- [x] `app.rs`: Shift+PageUp / Shift+PageDown handling in `handle_keyboard()`
- [ ] Optional: scrollbar indicator rect
- [ ] Manual test: scroll through `seq 1 500` output

## Success Criteria
- Mouse wheel scrolls terminal history up/down
- Shift+PageUp/Down jumps full page
- New PTY output does not forcibly scroll back to bottom when user is reviewing history
- Selecting different session resets scroll to bottom
- Scrollback stores up to 10,000 lines without OOM
- In alternate-screen apps (`vim`, `htop`), wheel/PageUp does not enter primary scrollback; view stays on alternate buffer

## Risk Assessment
- **`canvas::Program::update()` API** — use the signature already specified in Architecture section; avoid old `Status/Option<Message>` style from older Iced versions
- **VecDeque indexing with skip()** — O(n) iteration; acceptable for ≤10k lines but profile if slow
- **Cache invalidation on scroll** — must clear `canvas::Cache` on every scroll event; easy to miss

## Security Considerations
- N/A — scrollback contains same PTY output as viewport; no new data sources
