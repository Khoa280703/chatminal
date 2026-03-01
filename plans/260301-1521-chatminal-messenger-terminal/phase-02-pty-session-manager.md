# Phase 02 - PTY Session Manager

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-01-project-setup.md](phase-01-project-setup.md)
- Research: [PTY Libraries](../reports/researcher-260301-0820-rust-pty-terminal-libraries.md)
- Research: [Terminal Architecture](../reports/researcher-260301-1520-terminal-architecture.md)

## Overview
- **Priority:** P1 (core — blocks phases 03, 04, 05)
- **Status:** in-progress
- **Effort:** 6h
- **Goal:** Spawn PTY shells, parse ANSI output into TerminalGrid, stream updates to UI via channels

## Key Insights
- `portable-pty` API: `native_pty_system() → PtySystem → openpty(size) → (master, slave) → spawn_command(slave)`
- `vte::Parser` is NOT `Send` — keep it on PTY worker thread, never share across threads
- PTY reader is **sync/blocking** (`Box<dyn Read>`) — use `std::thread::spawn` per session, NOT tokio task
- Data model: PTY worker mutates local `TerminalGrid`, clones snapshot after each read batch, sends via channel
- **Clone-on-update** (not zero-copy): 80×24 grid ≈ 60KB; clone at 60fps = 3.5MB/s — negligible
- `Session` must hold `child: Box<dyn Child>` — needed for explicit reap via `child.wait()`
- Ring buffer for scrollback: use `VecDeque<Vec<Cell>>` with fixed capacity, pop_front when full
- Alternate screen (`use_alternate=true`) is isolated: **never push to `scrollback` while alternate is active**
- Phase 02 shell selection uses compile-safe baseline fallback; Phase 07 refactors to validated resolver helper
- Channel: `tokio::sync::mpsc` **bounded (cap=4)** — use `tx.blocking_send()` from OS thread to tokio channel
- Post-fix: `ESC M` reverse index handled in `esc_dispatch` (`cursor_row == 0` => `scroll_down(1)`), covered by parser-level regression test
- Post-fix: queue-full update path keeps latest dirty snapshot and retries; `Exited` send path uses `blocking_send` for reliable delivery
- Channel message type: `SessionEvent` enum (NOT raw `TerminalUpdate`):
  ```rust
  pub enum SessionEvent {
      Update { session_id: SessionId, grid: Arc<TerminalGrid>, lines_added: usize },
      Exited(SessionId),
  }
  ```
  `lines_added` = number of scrollback lines pushed since last flush (tracked by PtyPerformer via `scroll_up()` call count, reset to 0 after each `flush_update()`). **Only count pushes from primary screen; alternate-screen scroll does not affect `lines_added`.**

## Requirements
- `Session` struct holds PTY master, **child process handle**, session metadata
- `SessionManager` creates/closes sessions; owns `IndexMap<SessionId, Session>` (preserve insertion order for sidebar)
- `PtyWorker` runs in dedicated **OS thread** (`std::thread::spawn`) per session; feeds vte parser; sends `SessionEvent::Update`
- `TerminalGrid` stores `primary_grid` + `alternate_grid` (rows × cols), `use_alternate` flag, cursor position
- `Cell` stores: char, fg color, bg color, bold/italic/underline/blink attrs
- Channel: `tokio::sync::mpsc` **bounded(cap=4)** — PTY OS thread calls `tx.blocking_send(SessionEvent::Update {...})` to Iced Subscription

## Architecture

```
SessionManager
  create_session(name) → SessionId
    → native_pty_system().openpty(PtySize { rows, cols })
    → spawn shell via CommandBuilder → Box<dyn Child>
    → let reader = master.try_clone_reader()?  // portable-pty 0.9 API; NOT direct Box<dyn Read>
    → let writer = master.take_writer()?       // consuming — only call once per master
    → reader_handle = std::thread::spawn(pty_reader_thread(reader, event_tx, session_id))
    → writer_handle = std::thread::spawn(pty_writer_thread(writer, input_rx))
    → store Session { id, name, child, master, input_tx, reader_handle, writer_handle }

PtyReaderThread (std::thread per session, owns vte::Parser + TerminalGrid)
  loop:
    buf = [0u8; 4096]
    n = master_reader.read(&mut buf)?  // blocking, OK on OS thread
    parser.advance(&mut performer, &buf[..n])
    // Performer mutates local TerminalGrid
    // After each batch: clone grid → send SessionEvent::Update
    let lines_added = performer.take_lines_added(); // resets per-flush counter
    let snapshot = Arc::new(grid.clone());  // clone-on-update
    // blocking_send from OS thread to tokio mpsc channel
    let _ = tx.blocking_send(SessionEvent::Update {
        session_id,
        grid: snapshot,
        lines_added,
    });
    // On read EOF or error: send Exited then break
    // tx.blocking_send(SessionEvent::Exited(session_id)); break;

Iced Subscription
  polls tokio::sync::mpsc::Receiver<SessionEvent>
  on SessionEvent::Update { session_id, grid, lines_added } → Message::TerminalUpdated { session_id, grid, lines_added }
  on SessionEvent::Exited(id)                  → Message::SessionExited(id)
  → app.update() applies snapshot / cleans up session
```

## Data Structures

```rust
// src/session/grid.rs
pub type SessionId = uuid::Uuid;

pub struct Cell {
    pub c: char,
    pub fg: CellColor,
    pub bg: CellColor,
    pub attrs: CellAttrs,
}

pub struct CellAttrs {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
}

pub enum CellColor {
    Default,
    Indexed(u8),          // 256-color palette
    Rgb(u8, u8, u8),      // true color
}

pub struct TerminalGrid {
    pub cols: usize,
    pub rows: usize,
    pub primary_grid: Vec<Vec<Cell>>,    // [row][col]
    pub alternate_grid: Vec<Vec<Cell>>,  // [row][col], no scrollback
    pub use_alternate: bool,             // ?1049h => true, ?1049l => false
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scrollback: VecDeque<Vec<Cell>>,  // max 10_000 lines
}

// src/session/mod.rs
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub child: Box<dyn portable_pty::Child + Send>,     // must keep for reap
    pub master: Box<dyn MasterPty>,
    pub input_tx: tokio::sync::mpsc::Sender<Vec<u8>>,   // UI → PTY writer thread
    pub reader_handle: std::thread::JoinHandle<()>,      // must join on close
    pub writer_handle: std::thread::JoinHandle<()>,      // must join on close
    // alternate state lives in TerminalGrid.use_alternate (single source of truth)
}

// src/session/manager.rs
pub struct SessionManager {
    sessions: indexmap::IndexMap<SessionId, Session>,
    // Bounded channel cap=4; PTY OS thread calls blocking_send(); blocks on full (backpressure, no OOM)
    event_tx: tokio::sync::mpsc::Sender<SessionEvent>,
}

// TerminalUpdate REMOVED — use SessionEvent enum instead (see Key Insights)
```

## Related Code Files
- **Write:** `/home/khoa2807/working-sources/chatminal/src/session/grid.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/session/mod.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/session/manager.rs`
- **Write:** `/home/khoa2807/working-sources/chatminal/src/session/pty_worker.rs`

## Implementation Steps

1. **`grid.rs`** — define `Cell`, `CellColor`, `CellAttrs`, `TerminalGrid`
   - `TerminalGrid::new(cols, rows)` fills with blank cells (space, default colors)
   - `TerminalGrid::resize(cols, rows)` — reflow or truncate; **MANDATORY cursor clamping after resize**:
     ```rust
     self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
     self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
     ```
   - `TerminalGrid::set_cell(row, col, cell)` — **MUST be bounds-checked**; silently ignore OOB writes (never panic)
   - `TerminalGrid::scroll_up(n)` — if `use_alternate == false`, push rows to `scrollback` then clear bottom; if `use_alternate == true`, clear/shift only `alternate_grid` (no scrollback writes, no `lines_added` increment)
   - `TerminalGrid::active_cells(&self) -> &Vec<Vec<Cell>>` — returns `&self.alternate_grid` if `self.use_alternate`, else `&self.primary_grid`; **all rendering MUST use `active_cells()`, never `grid.cells` directly**

2. **`pty_worker.rs`** — `PtyPerformer` struct implements `vte::Perform`; `pty_reader_thread` is the thread entry point
   - `PtyPerformer` fields: `grid: TerminalGrid` (owned, not shared), `event_tx: tokio::sync::mpsc::Sender<SessionEvent>`, `session_id`
   - `fn pty_reader_thread(master_reader, event_tx, session_id)` — OS thread, blocking loop:
     ```
     let mut parser = vte::Parser::new();
     let mut performer = PtyPerformer::new(session_id, event_tx);
     loop {
         let n = master_reader.read(&mut buf)?;
         parser.advance(&mut performer, &buf[..n]);
         performer.flush_update(); // send Arc::new(grid.clone()) if dirty
     }
     ```
   - Separate **writer thread** per session: `std::thread::spawn` reads from `input_rx`, writes to `master_writer`
   - Implement `vte::Perform` on `PtyPerformer`:
     - `print(c)` → place char at cursor, advance cursor col
     - `execute(byte)` → handle `\n` (LF), `\r` (CR), `\x08` (BS)
     - `csi_dispatch(params, action)` → full SGR handling:
       - **`vte::Params` is an opaque iterator type — NOT a slice. `params[0]` will NOT compile.**
         Use the correct iterator API:
         ```rust
         // Get first param:
         let mut iter = params.iter();
         let p0 = iter.next().and_then(|sub| sub.first().copied()).unwrap_or(0);
         // For multi-byte sequences (38;5;n or 38;2;r;g;b):
         let flat: Vec<u16> = params.iter().flat_map(|s| s.iter().copied()).collect();
         ```
       - `flat[0] == 0` → reset all attrs (fg=Default, bg=Default, bold=false, underline=false)
       - `flat[0] == 1` → bold, `flat[0] == 3` → italic, `flat[0] == 4` → underline, `22/23/24` → reset each
       - `30-37/90-97` → indexed fg, `40-47/100-107` → indexed bg
       - `flat[0] == 38 && flat[1] == 5` → 256-color fg (`flat[2]` = index)
       - `flat[0] == 48 && flat[1] == 5` → 256-color bg (`flat[2]` = index)
       - `flat[0] == 38 && flat[1] == 2` → truecolor fg (`flat[2..4]` = r,g,b)
       - `flat[0] == 48 && flat[1] == 2` → truecolor bg (`flat[2..4]` = r,g,b)
       - cursor move (CUP/CUF/CUB/CUU/CUD), erase (ED/EL)
       - **Alternate screen (F2 fix):** `1049` with `h` action → save cursor + switch to alternate_grid; `1049` with `l` → restore cursor + switch to primary_grid. Match: `if p0 == 1049 { ... }` inside `csi_dispatch`. `?1049h` is a CSI sequence (`ESC [ ? 1049 h`), NOT an ESC two-byte sequence.
     - `esc_dispatch(intermediates, byte)` → true ESC two-byte sequences ONLY:
       - `ESC c` (byte `b'c'`) → full reset
       - `ESC M` (byte `b'M'`) → reverse index (scroll down)
       - **Do NOT handle `?1049h/l` here** — these are CSI sequences handled in `csi_dispatch` above

3. **`mod.rs`** — `Session` struct, re-exports

4. **`manager.rs`** — `SessionManager`
   - `create_session(name, cols, rows) → SessionId`
     - detect shell (Phase 02 baseline): `std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())`
     - Phase 07 will replace this with validated resolver helper before release hardening
     - `native_pty_system().openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })`
     - `pty.slave.spawn_command(cmd)` → `Box<dyn Child>` — store in Session
     - `std::thread::spawn(pty_reader_thread(...))` — reader thread
     - `std::thread::spawn(pty_writer_thread(...))` — writer thread
   - `close_session(id)` — safe shutdown order to prevent deadlock:
     ```rust
     fn close_session(&mut self, id: SessionId) {
         if let Some(s) = self.sessions.remove(&id) {
             // 1. Force child exit FIRST → reader gets EPIPE/EOF on next read
             //    This unblocks reader even if it is stuck in blocking_send() (channel full):
             //    child kill → slave PTY close → reader's next read() returns error → reader exits
             let _ = s.child.kill();
             // 2. Signal writer: close input channel → writer thread exits cleanly
             drop(s.input_tx);
             // 3. Drop master for belt+suspenders SIGHUP (child already killed above)
             drop(s.master);
             // 4. NOW join reader — safe because reader will get read error/EOF (child was killed)
             let _ = s.reader_handle.join();
             // 5. Reap child
             let _ = s.child.wait();
             // 6. Join writer (already exiting since input_tx dropped)
             let _ = s.writer_handle.join();
         }
     }
     ```
     **⚠️ Deadlock risk without `child.kill()` first:** If reader thread is blocked at `blocking_send()` (channel full — e.g. UI stalled or window minimized) and close_session calls `drop(master)`, the reader never reaches the next `read()` to see EOF. `reader_handle.join()` blocks forever → UI frozen. Fix: kill child BEFORE drop(master) to force EPIPE on reader's current or next read.
   - `send_input(id, bytes) → Result<(), SendError>` → cap input size: if `bytes.len() > MAX_INPUT_BYTES` (= 65_536), log `log::warn!` and return `Err`; prevents clipboard paste OOM. Return `Err` also if channel closed (session exited).
   - `resize_session(id, cols, rows)` → `master.resize(PtySize { ... })`

5. **Verify:** unit test `SessionManager::create_session` spawns bash, reads prompt bytes

## Todo List
- [x] `grid.rs`: Cell, CellColor, CellAttrs, TerminalGrid types
- [x] `grid.rs`: TerminalGrid::new, resize, set_cell, scroll_up
- [x] `pty_worker.rs`: PtyPerformer struct + vte::Perform impl (print, execute, csi_dispatch, esc_dispatch)
- [x] `pty_worker.rs`: full SGR multi-param handler (0=reset, 1=bold, 4=ul, 38/48 for 256+truecolor)
- [x] `pty_worker.rs`: alternate screen handler (?1049h/l in **csi_dispatch**, NOT esc_dispatch — it is a CSI sequence)
- [x] `pty_worker.rs`: blocking pty_reader_thread loop (std::thread, master.try_clone_reader())
- [x] `pty_worker.rs`: pty_writer_thread loop (master.take_writer(), exits on channel RecvError)
- [x] `TerminalGrid`: add primary_grid + alternate_grid buffers, `use_alternate: bool` field
- [x] `pty_worker.rs`: SGR color parsing — use `params.iter().flat_map(|s| s.iter().copied()).collect::<Vec<u16>>()` (NOT `params[0]` — `Params` is opaque iterator, not slice)
- [x] `manager.rs`: SessionManager create/close/resize/input
- [x] `manager.rs`: shell detection + CommandBuilder setup
- [x] `pty_worker.rs`: reverse index `ESC M` implementation + parser regression test
- [x] `pty_worker.rs`: queue-full handling keeps `dirty` snapshot and retries update send
- [x] `pty_worker.rs`: regression tests `reverse_index_esc_m_scrolls_down_from_top_row` and `flush_update_retries_after_queue_full`
- [ ] Smoke test: spawn session, read output, verify SessionEvent::Update received

## Success Criteria
- `cargo check` passes on session module
- Spawning a session produces a live PTY (bash prompt appears in grid)
- `SessionEvent::Update` received within 50ms of PTY output
- vte::Perform handles at minimum: text, colors (SGR), cursor movement, line feed, carriage return

## Risk Assessment
- **vte API changes in 0.15** — check trait method signatures before implementing
- **PTY master reader blocking** — ✅ handled by `std::thread::spawn`; do NOT use tokio task (would block thread pool)
- **Large output bursts** — bash may output KBs at once; batch vte processing per read, debounce UI updates
- **Bounded channel backpressure + deadlock risk** — cap=4 with `blocking_send` from OS thread; if UI stalls (window minimized, heavy load), reader can block at `blocking_send`. If `close_session` is called during this block, `drop(master)` alone does NOT unblock the reader (it's stuck in send, not in read). **Fix:** call `child.kill()` BEFORE `drop(master)` in `close_session` — forces EPIPE on reader's current read or the next one after send completes, guaranteeing join() terminates.
- **Residual medium edge case** — pending dirty snapshot may still be skipped when EOF arrives immediately after a queue-full event; add forced flush-before-`Exited` if strict last-frame guarantee is required.

## Security Considerations
- Shell inherits parent env — do not inject secrets into `CommandBuilder` env
- PTY input is raw bytes — validate length only, never interpret in manager layer
