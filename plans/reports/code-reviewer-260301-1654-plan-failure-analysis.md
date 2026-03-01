# Plan Failure Mode Analysis — Chatminal Messenger Terminal

**Scope:** All 8 plan files (plan.md + phase-01 through phase-07)
**Perspective:** Failure Mode Analyst — Murphy's Law, race conditions, deadlocks, PTY lifecycle edge cases

---

## Finding 1: Dual-Subscription Merge Race — PTY Events Lost at Startup

- **Severity:** Critical
- **Location:** Phase 03, section "Implementation Steps" step 4 (subscription); Phase 05, section "Implementation Steps" step 2 (extend subscription)
- **Flaw:** Phase 03 wires `Subscription::run_with(0u64, ...)` for PTY events; Phase 05 later extends it with `Subscription::batch([pty_subscription, event_subscription])`. The plan shows both in `subscription()` but never reconciles that merging them into a `batch()` changes the subscription hash key, causing Iced to tear down and recreate the inner async stream.
- **Failure scenario:** When Phase 05 adds `Subscription::batch(...)`, Iced sees a different subscription identity (batch wraps both subscriptions under a new composite hash). The `Arc<Mutex<Option<Receiver>>>` already has its `Option` consumed to `None` by the Phase 03 subscription. The Phase 05 batch teardown + recreate calls the closure again, hits `guard.take()` → `None` → the log_warn early return fires. Result: PTY events stop arriving in the UI from that point forward. The terminal goes silent — no update messages, no session-exited signals — while shells keep running invisibly in background threads.
- **Evidence:** Phase 03 step 4: `"Subscription::run_with(0u64, move |_key, output| { ... let mut rx = match guard.take() { None => { log::warn!(...); return; } ...}"`. Phase 05 step 2: `"Subscription::batch([pty_subscription, event_subscription])"` — no mention that batch changes the lifetime of the inner subscription or that the `Option<Receiver>` guard needs to survive re-evaluation.
- **Suggested fix:** Move the PTY channel receiver into a `oneshot`-initialized or lazily-initialized structure that is robust to re-entry, OR structure the subscription so the PTY receiver stream is its own `Subscription::run_with` with a stable unique key distinct from the keyboard subscription, and keep them batched from the very start (Phase 03), never changing the batch shape.

---

## Finding 2: `blocking_send` Deadlock Under Full Backpressure — PTY Reader Thread Hangs Forever

- **Severity:** Critical
- **Location:** Phase 02, section "Key Insights" and "Architecture"
- **Flaw:** The plan mandates `tx.blocking_send(SessionEvent::Update {...})` from the OS PTY reader thread into a `bounded(4)` channel. The receiver is polled by Iced's subscription async task. If Iced's event loop stalls (e.g., window minimized, slow GPU frame, system sleep, or the tokio runtime is busy) the subscription stops consuming from the channel. After 4 messages, `blocking_send` blocks indefinitely on the OS thread — which is also the only entity responsible for calling `master_reader.read()`. The PTY master read buffer (kernel-side) fills up. The child shell's writes block. The shell is now stuck, unable to output, unable to receive new input — a complete freeze of that session.
- **Failure scenario:** User minimizes app for 10 seconds. A background shell command (`find /`, `make`) produces burst output. PTY reader calls `blocking_send`, blocks on item 5. Reader thread is now frozen. Kernel PTY buffer (typically 4KB) fills. Child shell blocks on write. When user restores the window, Iced drains the channel — but the frozen reader thread may have missed bytes mid-sequence, corrupting the vte parser state. SGR sequences are now misaligned; terminal renders garbage colors for the rest of the session.
- **Evidence:** Phase 02 Risk: `"cap=4 with blocking_send from OS thread; if UI is too slow, PTY reader thread can block temporarily; acceptable for MVP"` — dismisses the deadlock scenario as "acceptable" without bounding "temporarily" or acknowledging the corruption risk from mid-sequence blocking.
- **Suggested fix:** Use `try_send` with discard-on-full (accept minor output drop), OR increase buffer to 64-256 and use a debounce/batch strategy (accumulate N ms of output, send one merged update). Also: on mid-sequence block recovery, the vte parser must be reset to avoid corrupt state.

---

## Finding 3: `close_session` Drop Order Is Racy on macOS/FreeBSD — `drop(master)` May Not Send SIGHUP

- **Severity:** Critical
- **Location:** Phase 02, section "Implementation Steps" step 4 (`close_session`); Phase 07, section "Implementation Steps" step 4
- **Flaw:** The plan's `close_session` relies on `drop(s.master)` sending SIGHUP to the child, which causes the PTY slave to close, which causes the cloned reader to get EOF. This is documented behavior on Linux. However, `portable-pty`'s `MasterPty` trait `drop` behavior is not guaranteed across all backends: on some platforms, closing the master only sends SIGHUP if no other fds are open on the slave. If `Box<dyn Child>` (stored as `s.child`) holds an open fd to the slave side internally (which `portable-pty` may do depending on implementation), `drop(s.master)` does NOT produce EOF on the reader. The reader thread then blocks forever on `read()`. `reader_handle.join()` then hangs forever, freezing the entire UI thread (since `close_session` is called from `app.rs update()` which runs on the main thread).
- **Failure scenario:** User clicks close on a session. `close_session` called. `drop(master)` executed, but `child` still holds slave fd open internally. Reader thread remains blocking on `read()`. `reader_handle.join()` called — never returns. App UI freezes. Entire window becomes unresponsive. Only option is OS kill.
- **Evidence:** Phase 02 step 4 comment: `"drop(master) BEFORE joining reader: SIGHUP → child exits → PTY slave closes → cloned reader gets EOF → reader thread exits"` — assumes a clean SIGHUP chain that is platform-conditional and may not fire if child holds slave fd open.
- **Suggested fix:** Add a timeout to `reader_handle.join()` using a separate watchdog or `thread::park_timeout`. Alternatively, use an explicit `AtomicBool` shutdown flag that the reader checks after each read iteration, combined with sending a SIGKILL to `child.kill()` before `drop(master)`.

---

## Finding 4: `scroll_offset` Anchor Update Missing Scrollback Line Count — Drift on Rapid Output

- **Severity:** High
- **Location:** Phase 06, section "Requirements" and "Implementation Steps" step 2
- **Flaw:** When `scroll_offset > 0` and new PTY data arrives, the plan says: "add new_lines_count to offset to anchor view". But `new_lines_count` is not defined in the data flow. `TerminalGrid.scroll_up(n)` pushes rows to scrollback and the plan never threads the count of newly-pushed scrollback rows back through the `SessionEvent::Update` message. The `Arc<TerminalGrid>` snapshot contains the new scrollback state but the delta (how many rows were added since last update) is not tracked.
- **Failure scenario:** User scrolls up 50 lines, anchored on a log line. Shell runs `tail -f /var/log/syslog`. Rapid log output adds 20 lines per second to scrollback. Each `TerminalUpdated` event updates the snapshot. `app.rs` cannot know how many lines were added (it only gets the new full snapshot, not the delta). Without the delta, offset cannot be reliably incremented. Either: (a) offset stays fixed, scroll view drifts upward relative to anchored content as new lines push old ones further back; or (b) a naive implementation computes `new_scrollback_len - old_scrollback_len` but requires storing previous scrollback length per session, which is not mentioned anywhere in the plan.
- **Evidence:** Phase 06 step 2: `"if scroll_offset > 0: add new_lines_count to offset to anchor view"` — `new_lines_count` variable is never defined, never added to `SessionEvent::Update`, never computed in `TerminalGrid`.
- **Suggested fix:** Add `lines_added: usize` field to `SessionEvent::Update`, populated by `PtyPerformer::flush_update()` tracking the count of `scroll_up` calls since last flush. Store `prev_scrollback_len: HashMap<SessionId, usize>` in `AppState` as an alternative.

---

## Finding 5: `TerminalGrid::resize` — No Reflow Spec, Cursor Corruption on Resize

- **Severity:** High
- **Location:** Phase 02, section "Implementation Steps" step 1 (`grid.rs`)
- **Flaw:** `TerminalGrid::resize(cols, rows)` is listed as a required method with the note "reflow or truncate" but no implementation strategy is specified. When a PTY resize happens (window drag), the grid is resized but the cursor position (`cursor_row`, `cursor_col`) is not addressed. If new cols < `cursor_col`, the cursor is now outside bounds. If new rows < `cursor_row`, the cursor row is outside bounds. Subsequent `set_cell(cursor_row, cursor_col, ...)` calls hit out-of-bounds — either panicking (if no bounds check) or silently writing to wrong cells.
- **Failure scenario:** User opens vim (which renders to specific row/col coordinates). User drags window to be narrower (e.g., 80→40 cols). `resize_all_sessions(40, rows)` is called. Grid is resized. Cursor at col 60 is now invalid (col 60 >= 40 cols). Next PTY output from vim sends cursor movement to col 60. `set_cell(row, 60, ...)` — if bounds-checked, silently drops; vim's render is now corrupted. If not bounds-checked, out-of-bounds Vec access panics the app.
- **Evidence:** Phase 02 step 1: `"TerminalGrid::resize(cols, rows) — reflow or truncate"` — one line with no spec. The todo list item only says `"grid.rs: TerminalGrid::new, resize, set_cell, scroll_up"`. No mention of cursor clamping.
- **Suggested fix:** Add explicit cursor clamping in `resize()`: `cursor_col = cursor_col.min(new_cols.saturating_sub(1))`, `cursor_row = cursor_row.min(new_rows.saturating_sub(1))`. Mandate bounds checks in `set_cell()` (return `Result` or silently clamp). This must be in the plan spec, not left to implementer intuition.

---

## Finding 6: Font Metrics Hardcoded as Magic Numbers — Cell Size Mismatch Corrupts Rendering

- **Severity:** High
- **Location:** Phase 04, section "Implementation Steps" step 2 ("Font setup")
- **Flaw:** `cell_width` is specified as "hardcode safe default (8.4px at 14pt)". This value is derived from a specific rendering pipeline and DPI assumption. On HiDPI screens (2x scaling), Iced scales coordinates but font rendering may produce different actual advance widths. The 8.4px magic number will be wrong on any non-standard DPI or font substitution scenario, causing text to overlap (if actual width > 8.4) or have gaps between columns (if actual width < 8.4). The plan provides no mechanism to measure actual font metrics at runtime.
- **Failure scenario:** User runs on a 4K monitor at 200% scaling. Iced scales the logical coordinate space. JetBrains Mono at 14pt logical renders at a different physical advance width than 8.4px. Characters overlap by 1-2px per column. At 200 cols wide, columns drift by 200-400px total. The right half of the terminal is garbage — chars stack on top of each other. User cannot read output.
- **Evidence:** Phase 04 step 2: `"cell_width = font advance width (for monospace = fixed); measure via iced::advanced::text::Paragraph or hardcode safe default (8.4px at 14pt)"` — "or hardcode" is taken as the acceptable path; `iced::advanced::text::Paragraph` approach is mentioned but never specced out.
- **Suggested fix:** Mandate using `iced::advanced::text::Paragraph` to measure actual `"M"` advance width after `FontLoaded` is received, storing result in `AppState`. Guard all rendering on this measurement being available (extend `font_loaded: bool` to `font_metrics: Option<FontMetrics>`). Remove the hardcoded fallback from the plan entirely.

---

## Finding 7: `SessionExited` → `CloseSession` Double-Dispatch — Use-After-Free of Session Data

- **Severity:** High
- **Location:** Phase 07, section "Architecture" and "Implementation Steps" step 5
- **Flaw:** When PTY exits, the reader thread sends `SessionEvent::Exited(id)`. The subscription converts this to `Message::SessionExited(id)`. `app.rs update()` then dispatches `CloseSession(id)`. `CloseSession` calls `session_manager.close_session(id)` which removes the `Session` from the `IndexMap`. However, between the `SessionExited` message and the `CloseSession` dispatch, the bounded channel (cap=4) may still hold buffered `SessionEvent::Update` messages. These will be processed as `Message::TerminalUpdated(id, grid)` AFTER the session is already removed. The `TerminalUpdated` arm attempts to store the grid snapshot (`session_grids.insert(id, grid)`) for a session that no longer exists — a zombie grid entry that is never cleaned up, leaking `Arc<TerminalGrid>` in `session_grids` indefinitely.
- **Failure scenario:** Shell runs a long `find /` command, produces rapid output. PTY exits (command finishes). The bounded channel has 4 buffered `Update` events followed by `Exited`. Subscription drains: processes Update, Update, Update, Update (inserts into session_grids), then Exited → CloseSession (removes from session_manager BUT does not touch session_grids). The `Arc<TerminalGrid>` for the dead session lives in `session_grids` forever. With 10+ session open/close cycles, `session_grids` accumulates stale entries. On a 10,000-line scrollback per session × 60KB per grid, this is 600KB per leaked session.
- **Evidence:** Phase 07 step 5: `"TerminalUpdated(session_id, grid) => update grid snapshot"` and `"SessionExited(id) message → auto-dispatch CloseSession(id)"` — no cleanup of `session_grids` entry on `CloseSession`. Phase 03 lists `session_grids: HashMap<SessionId, Arc<TerminalGrid>>` in AppState with no cleanup lifecycle.
- **Suggested fix:** In the `CloseSession` match arm, also remove from `session_grids`: `state.session_grids.remove(&id)`. In `TerminalUpdated`, guard with `if state.session_manager.get(id).is_some()` before inserting.

---

## Finding 8: Alternate Screen Grid Not Cloned in `SessionEvent::Update` — vim Output Corrupts Primary Buffer

- **Severity:** High
- **Location:** Phase 02, section "Key Insights" (alternate screen) and Architecture (clone-on-update)
- **Flaw:** The todo list includes `"TerminalGrid: add primary_grid + alternate_grid buffers, use_alternate: bool field"`. But the `SessionEvent::Update` sends `Arc::new(grid.clone())` — a single `TerminalGrid` snapshot. It is undefined which grid is cloned: the full struct (including both `primary_grid`, `alternate_grid`, and `use_alternate` flag) OR just the active one. If `grid.clone()` clones the whole struct including both buffers, the clone is doubly expensive (2× the stated 60KB). More critically, when rendering (Phase 04), `draw()` accesses `grid.cells` directly — there is no code path that switches between `primary_grid.cells` and `alternate_grid.cells` based on `use_alternate`. The rendering phase has no awareness of the alternate screen flag.
- **Failure scenario:** User opens vim (sends `?1049h` → `use_alternate = true`). vim writes to alternate buffer. `SessionEvent::Update` clones the full `TerminalGrid`. `draw()` renders `grid.cells` — which is `primary_grid.cells` (the shell prompt), not `alternate_grid.cells` (vim UI). Result: vim is running but user sees the old bash prompt. Alternatively if cells refers to active buffer: switching back to primary after vim (`?1049l`) renders the alternate buffer residue. Either way, alternate screen never works correctly.
- **Evidence:** Phase 02 todo: `"TerminalGrid: add primary_grid + alternate_grid buffers, use_alternate: bool field"` but the rendering in Phase 04 step 4 only references `grid.cells` with no conditional for `use_alternate`. The `PtyPerformer::flush_update()` in Phase 02 Architecture sends `Arc::new(grid.clone())` with no specification of which buffer is "current."
- **Suggested fix:** Specify in Phase 02 that `TerminalGrid::active_cells(&self) -> &Vec<Vec<Cell>>` returns either `primary_grid` or `alternate_grid` cells based on `use_alternate`. Mandate Phase 04 use `grid.active_cells()` instead of `grid.cells`. Ensure `flush_update()` always clones the currently-active buffer.

---

## Finding 9: No Zombie Child Reaping Strategy for Abnormal PTY Termination

- **Severity:** Medium
- **Location:** Phase 07, section "Architecture" (dead session detection) and Phase 02, section "Implementation Steps" step 4
- **Flaw:** The plan's dead session flow assumes `read() → EOF → SessionEvent::Exited → CloseSession → child.wait()`. This works for clean shell exits (`exit` command). It does not cover: (1) SIGKILL to child from outside (e.g., `kill -9 <pid>`), (2) PTY master error before reader detects EOF, (3) `close_session` racing with a PTY error mid-read. In case (1), the child becomes a zombie process (Z state) if `child.wait()` is never called. `close_session` only calls `child.wait()` after `reader_handle.join()` — but if the session is never explicitly closed by the user (user just kills the shell from outside), `CloseSession` is never called, `child.wait()` is never called, zombie persists until app exit.
- **Failure scenario:** User runs `bash` in Session 1, then from another terminal runs `kill -9 <bash_pid>`. PTY reader detects read error, sends `SessionEvent::Exited`. UI removes sidebar entry. But `Session` struct is destroyed without calling `child.wait()` if `CloseSession` incorrectly skips the wait (e.g., session already marked as missing from manager). Zombie bash process accumulates in `/proc`. After many shell deaths, system process table fills.
- **Evidence:** Phase 07 step 3: `"On read error or 0 bytes: tx.blocking_send(SessionEvent::Exited(...)) then exit thread"` — no explicit call to `child.wait()` from the reader thread. Phase 07 step 5: `"SessionExited(id) → auto-dispatch CloseSession(id)"` — only triggers `child.wait()` inside `close_session`, which is correct but the code path must be guaranteed even if session was already partially cleaned up.
- **Suggested fix:** Add explicit zombie reaping as a fallback: in the reader thread, after sending `Exited`, attempt `child.try_wait()` in a loop before exiting. Or, register a dedicated reaper thread/task that periodically calls `child.try_wait()` for all sessions with `SessionStatus::Exited`.

---

*Report generated: 2026-03-01*
*Plan version reviewed: 260301-1521*
