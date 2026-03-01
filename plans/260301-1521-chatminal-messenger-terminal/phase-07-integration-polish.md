# Phase 07 - Integration & Polish

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-06-virtual-scrolling.md](phase-06-virtual-scrolling.md)
- All prior phases must be complete

## Overview
- **Priority:** P2
- **Status:** in-progress
- **Effort:** 3h
- **Goal:** Wire all components end-to-end; session switching; graceful error handling; config file; release build verification

## Key Insights
- Session switching must: update active_id, reset scroll_offset, invalidate canvas cache, resize PTY if needed
- Dead session detection: if PTY child exits, send `Message::SessionExited(id)` → auto-remove or mark dead
- Config file: `~/.config/chatminal/config.toml` — minimal, YAGNI; only essential user-facing knobs
- `config.toml` parsed at startup via `toml` crate; falls back to defaults if missing
- Error handling: never panic on PTY errors — log + close session gracefully
- Session name: default to `"Session {n}"` (incrementing counter); auto-rename not needed for MVP
- Release build: `cargo build --release` must produce working binary with no debug artifacts
- Post-fix: `Config::normalized()` now clamps numeric bounds (`scrollback_lines`, `font_size`, `sidebar_width`) and handles non-finite values safely

## Requirements
- Full end-to-end flow: open app → create session → type commands → switch sessions → close session
- Dead PTY sessions cleaned up automatically (no zombie entries in sidebar)
- Config file supports: `shell`, `scrollback_lines`, `font_size`, `sidebar_width`
- All `unwrap()` / `expect()` in production paths replaced with proper error handling
- `log::error!` / `log::warn!` used throughout; `RUST_LOG=debug` enables verbose output
- `cargo clippy -- -D warnings` passes on release build

## Architecture

```
config.rs (extended):
  #[derive(serde::Deserialize)]
  pub struct Config {
      pub shell: Option<String>,
      pub scrollback_lines: Option<usize>,
      pub font_size: Option<f32>,
      pub sidebar_width: Option<f32>,
  }
  impl Default for Config { ... }
  pub fn load_config() -> Config  // reads ~/.config/chatminal/config.toml

Session lifecycle state machine:
  Active → Exited (PTY child exits)
  Exited → removed from SessionManager on next update tick

Dead session detection in pty_reader_thread:
  when read returns 0 bytes (EOF) or Err:
    tx.blocking_send(SessionEvent::Exited(session_id))
    exit thread (no is_exited field — channel message type is SessionEvent enum)

app.rs update():
  // is_exited flag REMOVED — use separate SessionExited(SessionId) channel message instead
  // When PTY reader thread exits: send SessionExited(id) → auto-dispatch CloseSession
  SessionExited(session_id) => dispatch CloseSession(session_id)
  TerminalUpdated { session_id, grid, lines_added } => update grid snapshot (+ use lines_added for scroll anchor if offset > 0)

Session switching (SelectSession):
  1. active_session_id = Some(id)
  2. scroll_offsets.insert(id, 0)   // reset to bottom
  3. terminal_canvas.generation += 1   // cache invalidation signal; actual clear in canvas::Program::update()
  4. optional size sync: compare cached session size vs current window-derived cols/rows;
     call resize only if mismatch (avoid redundant resize spam on every selection)
```

## Related Code Files
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/config.rs`
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/app.rs`
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/session/pty_worker.rs`
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/session/manager.rs`
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/session/mod.rs`
- **Removed**: `session_state.rs` file not needed — `SessionStatus` inline in `session/mod.rs`
- **Modify:** `/home/khoa2807/working-sources/chatminal/src/ui/mod.rs`

## Implementation Steps

1. **`config.rs`** — full config implementation
   - Add config deps to `Cargo.toml` (latest stable):
     - `serde = { version = "1.0.228", features = ["derive"] }`
     - `toml = "1.0.3"`
     - `dirs = "6.0.0"`
   - `load_config()`: `dirs::config_dir()` + `/chatminal/config.toml`; `toml::from_str()` with `Config::default()` fallback
   - Pass `Config` to `AppState::new()` at startup

2. **`session/mod.rs`** — add `SessionStatus` enum inline (no separate file)
   ```rust
   pub enum SessionStatus { Active, Exited }
   ```
   - Add `status: SessionStatus` field to `Session` struct in same file

3. **`pty_worker.rs`** — EOF handling
   - On read error or 0 bytes: `log::info!("PTY EOF session {}", session_id)`
   - On EOF/Err: `tx.blocking_send(SessionEvent::Exited(session_id))` then exit thread
   - Task exits cleanly (no panic)

4. **`manager.rs`** — error-safe wrappers
   - Implement `resolve_shell_path(config_shell: Option<&str>) -> Result<String, ShellResolveError>` and make `create_session()` use it
   - `send_input()`: return `Result`; if channel closed (session exited), log warn and return `Err`
   - `resize_session()`: guard against missing session id
   - `resize_all_sessions(cols, rows)`: primary resize path on WindowResized (NOT just active); prevents inactive vim/htop from getting wrong terminal size
   - `close_session()`: safe shutdown order (same as phase-02) — `child.kill()` → `drop(input_tx)` → `drop(master)` → `reader_handle.join()` → `child.wait()` → `writer_handle.join()`

5. **`app.rs`** — complete wiring
   - `AppState::new()`: call `load_config()`, init with config values
   - First session auto-created on startup (quality-of-life)
   - `CloseSession` arm: if closed session was active, select previous session or None; **MUST call `state.session_grids.remove(&id)`** to prevent memory leak
   - `SessionExited(id)` message → auto-dispatch `CloseSession(id)` (no is_exited field anywhere)
   - `TerminalUpdated { session_id, grid, lines_added }` arm: **guard with `if state.session_manager.contains(session_id)`** before inserting to `session_grids` — prevents zombie entries from buffered messages arriving after CloseSession; keep `lines_added` to preserve scroll anchor behavior
   - Session name counter: `AppState { next_session_num: usize }` → `format!("Session {}", n)`
   - Replace all remaining `unwrap()` calls with `?` or `log::error!` + continue

6. **Sidebar polish**
   - Show dead indicator for `SessionStatus::Exited` (gray text "Exited") — optional cosmetic
   - Keyboard shortcut legend at sidebar bottom: small gray text "Alt+N New • Alt+W Close"

7. **Final integration test checklist**
   - Open app → auto-creates Session 1 → bash prompt visible
   - Type `echo hello` → output renders
   - **Alt+N** → Session 2 created, sidebar highlights it
   - Click Session 1 in sidebar → switches back, prior output intact
   - Type `exit` in Session 2 → session auto-removed from sidebar (no zombie process)
   - **Alt+W** → closes active session; **Ctrl+W in shell** → deletes word (not intercepted)
   - Resize window → terminal cols/rows update (verify with `echo $COLUMNS`)
   - Mouse wheel scroll → scrollback history visible in primary screen (outside alternate-screen apps like `vim`/`htop`)
   - Close all sessions → app shows empty state (no panic)

8. **Release build**
   ```bash
   cargo build --release
   cargo clippy -- -D warnings
   ./target/release/chatminal
   ```

## Todo List
- [x] `Cargo.toml`: add `serde = { version = "1.0.228", features = ["derive"] }`, `toml = "1.0.3"`, `dirs = "6.0.0"`
- [x] `config.rs`: Config struct with serde, load_config(), Default impl
- [x] `config.rs`: clamp numeric bounds + non-finite handling in `Config::normalized()`
- [ ] `session/mod.rs`: add SessionStatus enum + status field to Session (inline — no separate file)
- [x] `pty_worker.rs`: EOF detection → `tx.blocking_send(SessionEvent::Exited(id))`
- [x] `manager.rs`: implement validated `resolve_shell_path()` and route `create_session()` through it
- [ ] `manager.rs`: error-safe send_input, resize_session, close_session
- [x] `app.rs`: AppState::new() with config
- [x] `app.rs`: auto-create first session on startup
- [x] `app.rs`: CloseSession selects adjacent session
- [x] `app.rs`: SessionExited(id) match arm → auto-dispatch CloseSession(id)
- [x] `app.rs`: next_session_num counter for naming
- [x] `app.rs`: replace all unwrap() in production paths
- [x] Sidebar: shortcut legend text
- [ ] Full integration test checklist (manual)
- [x] `cargo build --release` succeeds
- [x] `cargo clippy -- -D warnings` clean

## Automated Test Matrix

Write unit tests in `src/session/tests.rs` and `src/ui/tests.rs`:

| Module | Test case | Assert |
|--------|-----------|--------|
| `grid.rs` | `print('A')` → cell at cursor = 'A' | cell.c == 'A' |
| `grid.rs` | SGR `\x1b[31m` → fg = Indexed(1) (red) | cell.fg == CellColor::Indexed(1) |
| `grid.rs` | `\n` on last row (primary screen) → pushes row to scrollback | scrollback.len() == 1 |
| `grid.rs` | resize 80→40 cols → cells truncated | grid.cols == 40 |
| `input_handler.rs` | `Key::Up` → `\x1b[A` | bytes == b"\x1b[A" |
| `input_handler.rs` | `Key::Character("A")` + Ctrl → `[0x01]` | bytes == &[1] |
| `scrolling` | build_render_rows offset=10, height=24 → exactly 24 rows | rows.len() == 24 |
| `scrolling` | ring buffer at capacity → old rows evicted | scrollback.len() == MAX |

Add `#[cfg(test)]` mod in each file. No integration tests requiring PTY in unit test suite.

## Success Criteria
- All integration test checklist items pass
- `cargo test` passes with 0 failures
- No panics in normal usage scenarios
- `cargo clippy -- -D warnings` exits 0
- Release binary ~10-20MB, starts in <500ms
- Memory stable over 30 min with 3 active sessions

## Risk Assessment
- **`dirs` crate API** — verify `config_dir()` vs `config_local_dir()` for Linux XDG compliance
- **Auto-close on exit** — `SessionEvent::Update` and `SessionEvent::Exited` may race under heavy output; ignore late updates for unknown session id
- **Empty session state** — no active sessions must not panic view(); add `Option` guard in view()
- **Release binary size** — wgpu pulls in large deps; expect 20-50MB; use `cargo bloat` if unacceptable

## Security Considerations
- Config file loaded from user home — treat as untrusted input (shared machines / dotfile injection are realistic)
- **Shell path from config — validate thoroughly (path injection risk):**
  1. Read raw path metadata with `std::fs::symlink_metadata(path)`; reject if metadata fails
  2. Canonicalize raw path (`std::fs::canonicalize()`); reject if canonicalization fails
  3. Check against `/etc/shells` whitelist with normalization:
     - Parse non-comment lines from `/etc/shells`
     - Compare using `(candidate_raw == raw_path)` OR `(canonicalize(candidate_raw) == canonical_path)` to avoid `/bin` ↔ `/usr/bin` symlink false-negative
  4. Verify canonical path is `is_file()` AND `is_executable()` via `std::os::unix::fs::PermissionsExt` (`mode() & 0o111 != 0`)
  5. If raw path is symlink, allow only when step 3 allowlist check succeeds (symlink itself is not trusted; allowlist + canonical target is the trust boundary)
  6. On validation failure: `log::warn!("Invalid shell path, trying validated default resolver")`; then evaluate fallback candidates in order:
     - `std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())`
     - literal `"/bin/bash".to_string()`
     - **Each fallback candidate MUST pass the same steps 1-5** before use (never bypass validation)
  7. If all candidates fail validation: return recoverable `Err` (show UI error / keep app alive), **never panic**
  > `std::fs::metadata(path).is_ok()` alone does NOT prevent path traversal or arbitrary binary execution. A config file at `~/.config/chatminal/config.toml` can be attacker-controlled on shared machines or via dotfile injection.
- Do not log PTY output content (may contain passwords); log only structural events

## Unresolved Questions
- Should closing all sessions show an empty state or auto-quit the app? (MVP: empty state, no auto-quit)
- Config hot-reload needed? (YAGNI — skip for MVP)
- Session persistence across app restarts? (YAGNI — skip for MVP)
