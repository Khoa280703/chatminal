# Phase 05 - Input Handling

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-03-iced-ui-layout.md](phase-03-iced-ui-layout.md)
- Research: [Terminal Architecture](../reports/researcher-260301-1520-terminal-architecture.md)

## Overview
- **Priority:** P1
- **Status:** in-progress
- **Effort:** 3h
- **Goal:** Route keyboard events to active PTY session; handle app shortcuts; propagate window resize to PTY

## Key Insights
- **Terminal keyboard input arrives ONLY via `iced::event::listen()`** — works regardless of which widget has focus. `canvas::Program::update()` only receives canvas-specific events (mouse wheel, mouse click within canvas bounds) and only when mouse is hovering the canvas. Do NOT rely on canvas for keyboard input — keyboard events are dropped when user hovers the sidebar.
- Terminal input = raw bytes sent to PTY master writer — must convert Iced `Key` → ANSI byte sequences
- Arrow keys, function keys, Delete, etc. require ANSI escape encoding (e.g. Up = `\x1b[A`)
- **App shortcuts use Alt+key ONLY** — Ctrl+key ALWAYS pass-through to PTY (Ctrl+C, Ctrl+W, Ctrl+D must reach shell)
- Default shortcuts: `Alt+N`=new session, `Alt+W`=close session, `Alt+[1-9]`=switch session
- **Shortcuts are HARDCODED constants** in input layer code for MVP (no config overhead, YAGNI)
- `Alt+R` (rename) REMOVED — no UI backing in MVP
- Window resize → `SessionManager::resize_all_sessions(new_cols, new_rows)` → `PtySize` update for all sessions
- `new_cols = (window_width - SIDEBAR_WIDTH) / cell_width` as usize
- `new_rows = window_height / cell_height` as usize
- Input must ONLY go to `active_session_id` — never broadcast

## Requirements
- Capture all `KeyPressed` events in Iced event subscription
- Intercept app shortcuts: **Alt+N** (new), **Alt+W** (close active) — hardcoded constants (MVP, no config)
- Ctrl+key combinations: **NEVER intercept**, always forward raw bytes to PTY
- Convert printable chars + control sequences to PTY byte sequences
- Write bytes to `SessionManager::send_input(active_id, bytes)`
- On window resize: recalculate cols/rows, call `resize_all_sessions`
- Mouse clicks on sidebar handled by button `on_press` (already in Phase 03)
- No config for keybindings in MVP — hardcoded constants, easy to change later

## Architecture

```
Iced event stream
  → subscription: iced::event::listen()
  → Message::KeyboardEvent(event) | Message::WindowResized(w, h)

// Hardcoded shortcut constants — change only here, no config needed (YAGNI)
const SHORTCUT_NEW: &str = "n";
const SHORTCUT_CLOSE: &str = "w";

update() match:
  KeyboardEvent(Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, .. })) =>
    // Alt+key → check hardcoded shortcuts
    if modifiers.alt() {
        match key {
            Key::Character(s) if s.eq_ignore_ascii_case(SHORTCUT_NEW) =>
                return Task::done(Message::NewSession),
            Key::Character(s) if s.eq_ignore_ascii_case(SHORTCUT_CLOSE) => {
                if let Some(id) = active_id {
                    return Task::done(Message::CloseSession(id));
                }
            }
            _ => {} // Alt+key not a shortcut → fall through to forward_to_pty
        }
    }
    // ALL Ctrl+key → ALWAYS forward to PTY (Ctrl+C, Ctrl+D, Ctrl+W, etc.)
    // Unhandled Alt+key → also forward (e.g. Alt+. in bash)
    forward_to_pty(key, modifiers, active_id)

  WindowResized(w, h) =>
    let cols = ((w as f32 - SIDEBAR_WIDTH) / cell_width) as usize;
    let rows = (h as f32 / cell_height) as usize;
    // Resize ALL sessions — prevents inactive vim/htop from getting wrong size
    session_manager.resize_all_sessions(cols, rows)

fn forward_to_pty(key, modifiers, session_id, manager):
    let bytes = key_to_bytes(key, modifiers);
    // Cap input message size: 64KB max to prevent clipboard paste OOM
    // send_input() returns Err if bytes.len() > MAX_INPUT_BYTES (65_536) or if channel closed
    if let Err(e) = manager.send_input(session_id, bytes) {
        log::warn!("PTY input rejected: {e}");
    }

// IMPORTANT: Ctrl check MUST come before Key::Character — Ctrl+A is Character("a") in Iced
fn key_to_bytes(key, modifiers) -> Vec<u8>:
    // 1. Ctrl+char FIRST (Character("a") with Ctrl → control byte, NOT "a")
    if modifiers.control() {
        if let Key::Character(s) = key {
            return ctrl_byte(s);  // s.bytes()[0] & 0x1F
        }
    }
    match key {
        Key::Enter        => vec![b'\r'],
        Key::Backspace    => vec![b'\x7f'],
        Key::Tab          => vec![b'\t'],
        Key::Escape       => vec![b'\x1b'],
        Key::Up           => b"\x1b[A".to_vec(),
        Key::Down         => b"\x1b[B".to_vec(),
        Key::Right        => b"\x1b[C".to_vec(),
        Key::Left         => b"\x1b[D".to_vec(),
        Key::Home         => b"\x1b[H".to_vec(),
        Key::End          => b"\x1b[F".to_vec(),
        Key::Delete       => b"\x1b[3~".to_vec(),
        Key::PageUp       => b"\x1b[5~".to_vec(),
        Key::PageDown     => b"\x1b[6~".to_vec(),
        // Printable chars LAST — after all control checks
        Key::Character(s) => s.as_bytes().to_vec(),
        _ => vec![],
    }
```

## Related Code Files
- **Write:** `/home/khoa2807/working-sources/chatminal/src/ui/input_handler.rs` (new)
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/app.rs` (add event subscription + resize handler)
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/ui/mod.rs` (add input_handler mod)

## Implementation Steps

1. **`input_handler.rs`** — pure functions, no state
   - `pub fn key_to_bytes(key: &Key, modifiers: Modifiers) -> Vec<u8>` — full match table
   - `fn ctrl_byte(s: &str) -> Vec<u8>` — Ctrl+A..Z → `[char_code & 0x1F]`
   - Keep this file ≤150 lines; exhaustive match with `_ => vec![]` fallback

2. **Extend `app.rs` subscription**
   - **⚠️ F4 warning:** If `iced::event::listen()` is added to the subscription batch HERE (Phase 05) and the batch was not initialized with it in Phase 03, changing `Subscription::batch` structure may cause Iced to restart the PTY subscription. If the PTY subscription used an `Arc<Mutex<Option<Receiver>>>` take-once pattern, `take()` would return `None` after restart → PTY events lost permanently.
   - **Fix (preferred):** Initialize `Subscription::batch([pty_sub, event_sub])` with BOTH subscriptions from the start in Phase 03. This Phase 05 step then becomes a no-op (batch already includes event::listen). If Phase 03 batch was not set up this way, verify that the PTY subscription hash is STABLE after batch merge — it must not change identity.

3. **`update()` new match arms**
   - `Message::KeyboardEvent(e)` → call `handle_keyboard(e, &mut self)`
   - `Message::WindowResized(w, h)` → call `handle_resize(w, h, &mut self)`

4. **`handle_keyboard()` in `app.rs`**
   - Check `modifiers.alt()` → match against hardcoded constants (Alt+N/W) → dispatch app message if matched
   - Otherwise (including ALL Ctrl+key, unhandled Alt+key): `forward_to_pty` unconditionally
   - Guard: if `active_session_id.is_none()` → ignore

5. **`handle_resize()` in `app.rs`**
   - Compute cols/rows from pixel dimensions
   - Minimum: 10 cols × 5 rows (guard against tiny windows)
   - Call `session_manager.resize_all_sessions(cols, rows)` — resize ALL sessions (not just active)
   - Store `current_cols`, `current_rows` in `AppState` for new session creation

6. **Verify:** type `ls` in active terminal, see directory listing; **Alt+N** opens new session

## Todo List
- [x] `input_handler.rs`: key_to_bytes() — printable chars
- [x] `input_handler.rs`: key_to_bytes() — control chars (Enter, BS, Tab, Esc)
- [x] `input_handler.rs`: key_to_bytes() — arrow keys + nav keys (Home, End, PgUp, PgDn, Del)
- [x] `input_handler.rs`: ctrl_byte() for Ctrl+A..Z
- [x] `app.rs`: extend subscription with event::listen()
- [x] `app.rs`: KeyboardEvent match arm → handle_keyboard()
- [x] `app.rs`: WindowResized match arm → handle_resize()
- [x] `app.rs`: Alt+key shortcut dispatch (hardcoded constants); Ctrl+key ALWAYS pass-through to PTY
- [ ] Manual test: type commands, verify PTY receives correct bytes

## Success Criteria
- Typing in terminal pane sends keystrokes to active PTY
- `ls`, `echo hello`, arrow key history navigation all work
- **Ctrl+W in shell deletes word** (not closes session) — confirms no stealing
- **Alt+N creates new session**, **Alt+W closes active** (hardcoded — YAGNI, easy to change in shortcut constants)
- Window resize updates PTY size (verify with `tput cols` in terminal)

## Risk Assessment
- **Iced focus model** — Keyboard events arrive via `iced::event::listen()` globally, NOT via Canvas focus. No explicit focus handling needed for keyboard. Canvas `update()` only fires for mouse events within canvas bounds — if keyboard was routed through canvas, events would drop when user hovers the sidebar. Implementation MUST use `iced::event::listen()` exclusively for keyboard input.
- **Key encoding edge cases** — terminal apps (vim, htop) are sensitive to exact escape sequences; test with common apps
- **Ctrl+C / Ctrl+Z** — must reach PTY as raw bytes (not intercepted by app); test carefully

## Security Considerations
- Input bytes forwarded verbatim to PTY — intentional; no sanitization needed (user controls their own shell)
