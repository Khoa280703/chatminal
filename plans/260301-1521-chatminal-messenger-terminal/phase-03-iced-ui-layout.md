# Phase 03 - Iced UI Layout

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-02-pty-session-manager.md](phase-02-pty-session-manager.md)
- Next: [phase-04-terminal-rendering.md](phase-04-terminal-rendering.md)
- Research: [UI Frameworks](../reports/researcher-260301-0820-rust-ui-frameworks-evaluation.md)

## Overview
- **Priority:** P1
- **Status:** in-progress
- **Effort:** 5h
- **Goal:** Iced app skeleton with messenger layout — sidebar left, terminal pane right; wired to SessionManager

## Key Insights
- Iced 0.14 uses `iced::application(boot, update, view)` builder API (not `Application` trait)
- Layout: `row![sidebar(240px fixed), vertical_rule(1), terminal_pane(fill)]`
- Sidebar is a `scrollable(column![...session_items...])` — no custom widget needed
- TerminalPane is a `Canvas` widget implementing `canvas::Program` — renders cells in Phase 04
- Iced `Subscription` bridges `tokio::sync::mpsc::Receiver<SessionEvent>` → `Message::{TerminalUpdated, SessionExited}`
- `AppState` is the single source of truth: `SessionManager` + `active_session_id`
- Avoid `Arc<Mutex<>>` in view() — Iced view must be pure/fast; only read owned data

## Requirements
- `AppState` is the Iced program state (`boot + update + view + subscription`)
- `Message` enum covers all user actions + async events
- Sidebar renders session list; click → `Message::SelectSession(id)`
- TerminalPane placeholder (black rect) renders in right panel — actual cells in Phase 04
- Subscription polls `SessionEvent` receiver, converts `Update` → `Message::TerminalUpdated { session_id, grid, lines_added }`, `Exited` → `Message::SessionExited`
- Window title: "Chatminal"
- Window min size: 800×600

## Architecture

```
main.rs
  → iced::application(AppState::boot, AppState::update, AppState::view)
      .subscription(AppState::subscription)
      .window(settings)
      .run()

AppState {
    session_manager: SessionManager,
    active_session_id: Option<SessionId>,
    // sessions_order removed — use IndexMap<SessionId, Session> in SessionManager for ordered iteration
    session_grids: HashMap<SessionId, Arc<TerminalGrid>>,  // must call .remove(&id) on CloseSession to avoid memory leak
    font_loaded: bool,   // load status / error diagnostics only
    font_metrics: Option<(f32, f32)>, // (cell_width, cell_height), render gate lives here
    // NOTE: update_rx is NOT stored in AppState — see subscription() below for correct pattern
}

// Canonical Message enum — defined in src/message.rs, imported everywhere
Message {
    TerminalUpdated { session_id: SessionId, grid: Arc<TerminalGrid>, lines_added: usize },
    SelectSession(SessionId),
    NewSession,
    CloseSession(SessionId),
    // RenameSession REMOVED — no UI backing in MVP (YAGNI)
    KeyboardEvent(iced::Event),   // raw Iced event — filtered in app.rs
    WindowResized(u32, u32),
    ScrollTerminal { delta: i32 },
    ScrollToBottom,
    FontLoaded(Result<(), iced::font::Error>),  // async font load completion
    SessionExited(SessionId),                    // PTY EOF → auto-cleanup
}

view() layout:
  row![
    sidebar_widget(&state),          // 240px wide
    vertical_rule(1),
    terminal_pane_widget(&state),    // fill remaining
  ]
```

## Related Code Files
- **Write:** `/home/khoa2807/working-sources/chatminal/src/main.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/app.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/message.rs` ← **canonical Message enum, shared by all phases**
- **Write:** `/home/khoa2807/working-sources/chatminal/src/ui/mod.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/ui/sidebar.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/ui/terminal_pane.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/ui/theme.rs`

## Implementation Steps

1. **`theme.rs`** — define color constants
   ```rust
   pub const SIDEBAR_BG: Color = Color::from_rgb(0.12, 0.12, 0.14);
   pub const ACTIVE_SESSION_BG: Color = Color::from_rgb(0.20, 0.20, 0.24);
   pub const TERMINAL_BG: Color = Color::BLACK;
   pub const TEXT_PRIMARY: Color = Color::WHITE;
   pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.6);
   pub const SIDEBAR_WIDTH: f32 = 240.0;
   pub const SESSION_ITEM_HEIGHT: f32 = 56.0;
   ```

2. **`sidebar.rs`** — `pub fn sidebar_view<'a>(sessions, active_id) -> Element<'a, Message>`
   - `scrollable(column(session_items))` where each item is:
     ```rust
     button(
       column![text(name).size(14), text(subtitle).size(11).color(TEXT_SECONDARY)]
     )
     .on_press(Message::SelectSession(id))
     .style(if active { active_style } else { normal_style })
     .width(SIDEBAR_WIDTH)
     .height(SESSION_ITEM_HEIGHT)
     ```
   - Bottom: `button("+ New").on_press(Message::NewSession)`

3. **`terminal_pane.rs`** — `pub fn terminal_pane_view<'a>(session) -> Element<'a, Message>`
   - Phase 03: return `canvas(TerminalCanvas { grid: None })` as placeholder (black background)
   - Phase 04 fills actual rendering logic in `TerminalCanvas`
   - `TerminalCanvas` struct: `pub grid: Option<Arc<TerminalGrid>>`

4. **`app.rs`** — `AppState` + `update()` + `view()` + `subscription()`
   - `update()` match arms:
     - `NewSession` → `state.session_manager.create_session(...)` — SessionManager uses IndexMap, preserves insertion order; no sessions_order field in AppState
     - `SelectSession(id)` → `state.active_session_id = Some(id)`
     - `CloseSession(id)` → remove from manager, update active session selection
     - `TerminalUpdated { session_id, grid, lines_added }` → store snapshot in `session_grids: HashMap<SessionId, Arc<TerminalGrid>>` **only if `session_manager.contains(session_id)`**; keep `lines_added` for Phase 06 scroll anchor logic
     - `CloseSession(id)` → **MUST call `state.session_grids.remove(&id)`** to prevent memory leak (50 sessions × 60KB+ each = 3MB+ without scrollback; with scrollback → 300MB+)
     - `WindowResized(w, h)` → delegate to `handle_resize()` (implemented in phase 05) to compute `cols/rows`, then call `session_manager.resize_all_sessions(cols, rows)`
   - `view()`: `row![sidebar_view(...), vertical_rule(1), terminal_pane_view(...)]`
   - `subscription()`:
     > **⚠️ CRITICAL — F1 + F4 fixes apply here:**
     >
     > **F1:** `Subscription::run_with(data, builder)` takes a **fn pointer** as `builder`, NOT a closure. A closure that captures `rx_arc` cannot coerce to `fn` pointer → compile error. Do NOT use closure with `run_with`.
     >
     > **F4:** The `Arc<Mutex<Option<Receiver<...>>>>` take-once pattern is fragile. If Iced restarts the subscription (e.g. when `Subscription::batch` structure changes in Phase 05 when event::listen() is added), `take()` returns `None` and PTY events are permanently lost.
     >
     > **CORRECT approach:** Create the `tokio::sync::mpsc` channel inside `boot()`. Store the `Sender` in `SessionManager` (already spec'd as `event_tx` field). For the subscription, use a stream created from the receiver. The Sender/Receiver pair is created once; SessionManager holds Sender; the subscription stream holds Receiver. Subscription identity must be STABLE across Phase 05 batch merging.
     >
     > **⚠️ VERIFY Iced 0.14 subscription API before implementing.** Options (verify which exists in 0.14):
     > - `iced::subscription::channel(id, buffer_size, |mut output| async move { ... })` — if available, this is cleanest
     > - `Subscription::run` (no data param) with a stream-from-channel approach
     > - `Subscription::from_recipe` with a custom Recipe impl
     >
     > **Pre-Phase 05 recommendation:** Initialize `Subscription::batch([pty_sub, event_sub])` from the start in Phase 03, where `event_sub = iced::event::listen().map(Message::KeyboardEvent)`. This ensures the batch identity is STABLE and Phase 05 does not change the subscription structure (preventing subscription restart + take-once failure).
     ```rust
     // PSEUDOCODE — verify exact API against Iced 0.14 source before implementing:
     fn subscription(&self) -> Subscription<Message> {
         // Both subscriptions initialized here so batch identity is stable from Phase 03:
         let pty_sub = /* stream-based subscription holding Receiver<SessionEvent> */;
         let event_sub = iced::event::listen().map(Message::KeyboardEvent);
         Subscription::batch([pty_sub, event_sub])
     }
     ```

5. **`main.rs`** — Iced 0.14 correct API
   ```rust
   fn main() -> iced::Result {
       // Mask SIGPIPE — prevents app crash when writing to dead PTY
       unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN); }
       env_logger::init();
       // Iced 0.14: first arg is boot fn (NOT title string)
       iced::application(AppState::boot, AppState::update, AppState::view)
           .title(|_state| String::from("Chatminal"))
           .subscription(AppState::subscription)
           .window(window::Settings {
               min_size: Some(iced::Size::new(800.0, 600.0)),
               ..Default::default()
           })
           .run()
   }

   // boot() returns initial state + startup Task (font loading only in Phase 03)
   // first-session auto-create is finalized in Phase 07 integration wiring
   fn boot() -> (AppState, Task<Message>) {
       let state = AppState::new();
       let font_task = iced::font::load(include_bytes!("../assets/JetBrainsMono.ttf"))
           .map(Message::FontLoaded);
       (state, font_task)
   }
   ```
   **Note:** Add `libc = "0.2.182"` to Cargo.toml dependencies.

6. **Verify:** `cargo run` shows split layout, sidebar has "+ New" button, clicking creates session entry

## Todo List
- [x] `theme.rs`: color constants + layout constants
- [x] `sidebar.rs`: session list view function, active highlight style
- [x] `sidebar.rs`: New Session button at bottom
- [x] `terminal_pane.rs`: placeholder Canvas widget (black rect)
- [x] `app.rs`: AppState struct with all fields
- [x] `message.rs`: canonical `Message` enum (all variants), imported by `app.rs` and UI/session modules
- [x] `app.rs`: update() all match arms (stubs OK for terminal ops)
- [x] `app.rs`: view() with row layout
- [x] `app.rs`: subscription() wrapping mpsc receiver
- [x] `main.rs`: application builder + run
- [ ] `cargo run` shows window without panic

## Success Criteria
- Window opens, shows sidebar + right panel split
- Clicking "+ New" adds entry in sidebar
- Clicking session entry highlights it
- No panic on window resize

## Risk Assessment
- **Iced 0.14 subscription API** changed from 0.12/0.13 — check `iced::subscription::channel` vs `Subscription::from_recipe`
- **Ownership in view()** — Iced view borrows `&self`; all data must be accessible without locks in hot path
- **`vertical_rule` widget** may not exist in 0.14 — fallback to `container` with fixed width + bg color

## Security Considerations
- N/A — no external data rendered in this phase (placeholder only)
