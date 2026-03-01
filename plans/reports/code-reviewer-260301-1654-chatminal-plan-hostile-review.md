# Plan Review: Chatminal Messenger Terminal

**Reviewer:** code-reviewer (Assumption Destroyer mode)
**Date:** 2026-03-01
**Plan:** plans/260301-1521-chatminal-messenger-terminal/
**Scope:** plan.md + phase-01 through phase-07

---

## Finding 1: `Subscription::run_with` requires a `fn` pointer, not a closure — Arc clone pattern will not compile

- **Severity:** Critical
- **Location:** Phase 03, section "Implementation Steps" step 4 — subscription() code block
- **Flaw:** The plan's `subscription()` implementation captures `rx_arc` via a closure passed to `Subscription::run_with(0u64, move |_key, output| { ... })`. The verified signature of `run_with` in iced 0.14.0 is:
  ```rust
  pub fn run_with<D, S>(data: D, builder: fn(&D) -> S) -> Subscription<T>
  ```
  The second argument is `fn(&D) -> S` — a **function pointer**, not `impl Fn`. Closures that capture environment (like `move |_key, output| { let rx_arc = rx_arc; ... }`) are **not** function pointers. This will fail to compile with a type mismatch error.
- **Failure scenario:** Implementer writes the closure exactly as specced. `cargo check` produces: `expected fn pointer, found closure`. The entire subscription mechanism is broken. All PTY output is silently dropped. App opens but no terminal output ever appears.
- **Evidence:** Plan code: `Subscription::run_with(0u64, move |_key, output| { let rx_arc = rx_arc; ... })` — captures `rx_arc` via closure environment, incompatible with `fn` pointer bound.
- **Suggested fix:** Use `Subscription::run(builder_fn)` where `builder_fn` is a standalone `fn()` that returns a Stream, passing the Arc via `with()`: `Subscription::run(stream_fn).with(rx_arc)`. The `data` parameter in `run_with` must be the capture context, and `builder` must be a pure `fn` pointer — reconstruct the pattern accordingly.

---

## Finding 2: `ansi_colours` crate API is misrepresented — plan assumes a palette table that does not exist

- **Severity:** High
- **Location:** Phase 01, Cargo.toml; Phase 04, "color_palette.rs" implementation steps
- **Flaw:** The plan adds `ansi_colours = "1"` to Cargo.toml with the comment "256-color palette — dùng crate, không hardcode 256 values". The actual `ansi_colours` crate API (verified) provides:
  - `ansi256_from_rgb(r, g, b) -> u8` — converts RGB to nearest 256-color index
  - `rgb_from_ansi256(n) -> (u8, u8, u8)` — converts index to RGB

  The crate does expose `rgb_from_ansi256`, which can serve the lookup purpose. However, Phase 04 then hardcodes its own `XTERM_PALETTE: [u32; 256]` const array anyway (`color_palette.rs` step 1), making the `ansi_colours` dep completely redundant. The plan contradicts itself: it says "use crate, don't hardcode" but then specifies hardcoding the table regardless. The crate is added to the dependency graph for no reason while also not being used in the pattern described.
- **Failure scenario:** Implementer imports `ansi_colours` expecting a `PALETTE` const or similar, finds no such export. Meanwhile `color_palette.rs` manually embeds the full 256-value table anyway. Dead dependency ships in release binary; or implementer is confused and uses `rgb_from_ansi256()` correctly but the plan's rationale is misleading.
- **Evidence:** Phase 01: `ansi_colours = "1" # 256-color palette — dùng crate, không hardcode 256 values`. Phase 04 step 1: `pub const XTERM_PALETTE: [u32; 256] = [ /* 256 RGB hex values */ ];` — a full hardcoded array. The two statements are mutually contradictory.
- **Suggested fix:** Remove `ansi_colours` from Cargo.toml (YAGNI — the plan already hardcodes the palette). If the crate is retained, use `rgb_from_ansi256(n)` directly and delete `XTERM_PALETTE`. Pick one approach; remove the other.

---

## Finding 3: `esc_dispatch` used to handle `?1049h` alternate screen — ESC dispatch does not receive CSI `?` sequences

- **Severity:** Critical
- **Location:** Phase 02, "Implementation Steps" step 2, `vte::Perform` implementation — `esc_dispatch(intermediates, byte)` section
- **Flaw:** The plan routes alternate screen switching (`?1049h`, `?1049l`) through `esc_dispatch`. This is architecturally wrong. `?1049h` is a **CSI sequence**: `ESC [ ? 1049 h`. The VTE state machine calls `csi_dispatch` for sequences starting with `ESC [`, NOT `esc_dispatch`. `esc_dispatch` handles two-character escape sequences like `ESC c` (RIS), `ESC 7` (DECSC), `ESC M` (reverse index) — sequences that do NOT contain `[`. The `?` in `?1049h` is a CSI private parameter prefix, not part of an ESC sequence body. Verified `esc_dispatch` signature: `fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8)` — no mechanism exists in this callback to receive `1049` as a parameter.
- **Failure scenario:** Implementer implements `esc_dispatch` matching on DEC private modes, waits for `?1049h`. It never fires. Vim, htop, and any full-screen app appear broken — their output bleeds into the primary grid, cursor positioning is wrong, and the primary buffer is corrupted on exit. This is a complete failure for any app that uses alternate screen.
- **Evidence:** Phase 02: `esc_dispatch(intermediates, byte)` → `?1049h → save cursor + switch to alternate_grid; ?1049l → restore + primary_grid`. The `?1049h` pattern is a CSI-PM sequence, not an ESC-only sequence.
- **Suggested fix:** Move `?1049h/l` detection to `csi_dispatch`. Match on: `params` where first param is `1049`, `intermediates` contains `b'?'` (or check action char), and the final byte is `b'h'` or `b'l'`. The `esc_dispatch` should instead handle `ESC 7`/`ESC 8` (cursor save/restore), `ESC M` (scroll reverse), and `ESC c` (hard reset).

---

## Finding 4: `Subscription::run_with(0u64, ...)` — `u64` does not satisfy the `Hash` bound in a meaningful way for singleton identity, AND reuse semantics are unverified

- **Severity:** High
- **Location:** Phase 03, section "Implementation Steps" step 4; plan.md architecture diagram
- **Flaw:** The plan claims "Iced reuses subscription when hash key is unchanged (0u64 = singleton)". Two problems:
  1. `u64` does implement `Hash`, so this compiles. BUT: the identity of a `run_with` subscription is determined by `(data, builder_fn_pointer)` together. If the implementer uses a closure (Finding 1), the fn-pointer identity cannot be guaranteed stable across recomputations. If correctly using a `fn` pointer, it works — but the plan never verifies this.
  2. More critically: if `subscription()` is called again due to state change (which Iced does on every `update()` → `view()` → `subscription()` cycle), and the hash matches, Iced will NOT restart the stream. But the `Arc<Mutex<Option<Receiver>>>` take-once pattern means: if the subscription IS restarted (different hash due to closure identity), the `take()` returns `None` on the second call, and the subscription silently stops receiving events with only a `log::warn!`. This is a latent data loss risk with no recovery path.
- **Failure scenario:** Any state mutation that accidentally changes the closure identity causes Iced to restart the subscription, `take()` returns `None`, all PTY events stop reaching the UI. App appears frozen. No error is surfaced to the user.
- **Evidence:** Phase 03: `None => { log::warn!("subscription already initialized; skip duplicate init"); return; }` — this `return` exits the async block permanently, dropping the subscription silently.
- **Suggested fix:** Use a `fn` pointer (not closure) for `builder`, move `rx_arc` into the `data` parameter of `run_with`. Alternatively, consider using `Subscription::run` with a stream created once in a way that doesn't require Arc/Mutex. If the take-once pattern is kept, replace silent `return` with a `futures::stream::pending()` infinite future so the subscription stream never ends (keeps the slot occupied, Iced won't restart it).

---

## Finding 5: Cell width hardcoded/estimated — will misalign for non-ASCII and at different DPI

- **Severity:** High
- **Location:** Phase 04, "Implementation Steps" step 2 — font metrics section
- **Flaw:** The plan states: `cell_width` = font advance width; measure via `iced::advanced::text::Paragraph` **or hardcode safe default (8.4px at 14pt)**. The fallback "or hardcode 8.4px" is the path most implementers will take when `Paragraph` measurement proves difficult. At 8.4px per cell, a standard 80-column terminal is 672px wide. On a HiDPI (2x) display, iced applies scaling — but the PTY's `PtySize` is calculated from pixel dimensions divided by hardcoded `cell_width`/`cell_height`, which do not account for display scale factor. Result: cols/rows reported to PTY will be double the visual columns, causing all line-wrapping in bash/vim to break at exactly half the visible width.
- **Failure scenario:** User on a 4K monitor sees bash wrap lines at column 40 while the terminal visually shows 80 columns. `tput cols` reports 160. Vim splits its view at wrong positions. The bug is invisible on 1x displays and only manifests on HiDPI hardware.
- **Evidence:** Phase 04: `cell_width` = ... "hardcode safe default (8.4px at 14pt)". Phase 05 resize calculation: `new_cols = ((w as f32 - SIDEBAR_WIDTH) / cell_width) as usize` — uses raw pixel width with no scale factor consideration.
- **Suggested fix:** Retrieve the window scale factor from `iced::window::Settings` or the resize event, divide pixel dimensions by scale before computing cols/rows. Or use `cell_width * scale_factor` in the denominator.

---

## Finding 6: `canvas::Program::update()` signature — plan's Phase 06 claims Event is passed by reference; plan also never wires canvas to receive keyboard events

- **Severity:** High
- **Location:** Phase 06 "Architecture" section; Phase 05 "Key Insights" section
- **Flaw:** Two related issues:
  1. Phase 06 shows `canvas::Program::update()` receiving `event: &Event`. Verified actual signature: `fn update(&self, _state: &mut Self::State, _event: &Event, _bounds: Rectangle, _cursor: Cursor) -> Option<Action<Message>>`. The reference matches — this is correct. However Phase 05 states "Iced keyboard events arrive via `canvas::Program::update()` **or** `iced::event::listen()`" as if both work equivalently. They do not: `canvas::Program::update()` only receives events when the canvas has **focus and is hovered** — it will NOT receive keyboard events if the mouse is over the sidebar. The plan uses `iced::event::listen()` (correct path) but creates ambiguity that implementers may rely on canvas events for keyboard input.
  2. Phase 05 "Risk Assessment" notes: "Canvas widget must be focused to receive keyboard events; may need `canvas.on_key_press()` or explicit focus handling" — this risk is acknowledged but no resolution is provided. The canvas focus model in iced 0.14 requires explicit `canvas.on_key_press()` or the app using `keyboard::on_key_press()` subscription. If the implementer relies on `canvas::Program::update()` for keyboard, typing will silently fail whenever the mouse is not over the terminal pane.
- **Failure scenario:** User opens app, clicks sidebar to select session, then types. Keyboard events go through `iced::event::listen()` correctly IF that subscription is properly set up. But if implementer uses canvas event path (following the ambiguous "or" in Phase 05), keystrokes when hovering sidebar are silently dropped.
- **Evidence:** Phase 05: "Iced keyboard events arrive via `canvas::Program::update()` or `iced::event::listen()`" — the "or" implies equivalence that does not exist for cross-widget keyboard capture.
- **Suggested fix:** Remove the "or `canvas::Program::update()`" reference for keyboard in Phase 05. Mandate `iced::event::listen()` as the sole keyboard capture mechanism. Note that canvas `update()` is only valid for mouse events within canvas bounds.

---

## Finding 7: `blocking_send` from PTY reader thread with bounded(4) will deadlock close_session shutdown sequence

- **Severity:** Critical
- **Location:** Phase 02, "Architecture" section — `close_session()` shutdown order; Phase 02 "Implementation Steps" step 4
- **Flaw:** The plan's `close_session()` sequence is:
  1. `drop(input_tx)` — stops writer thread
  2. `drop(master)` — drops PTY master, should cause SIGHUP → slave closes → reader gets EOF
  3. `reader_handle.join()` — waits for reader thread

  The problem: the PTY reader thread is blocked in `tx.blocking_send(SessionEvent::Update {...})` when the channel is full (cap=4). The UI might be slow — channel is full. Reader is blocked in `blocking_send`. Then `drop(master)` is called from the main/async thread. Even though master is dropped, the reader thread never gets past `blocking_send` to reach the next `read()` that would see EOF. The `reader_handle.join()` blocks forever. The app hangs on session close.

  Additionally: `drop(master)` does not guarantee SIGHUP or EOF on the cloned reader. `try_clone_reader()` returns a clone of the reader end. On Linux, the PTY slave remains open as long as any slave fd exists (the child process holds one). Dropping the master does send SIGHUP to the foreground process group, but the reader clone may not get EOF until the child also exits. The plan assumes `drop(master)` → immediate reader EOF — this is not guaranteed.
- **Failure scenario:** User closes a session with a busy terminal (e.g., `cat /dev/urandom | head -c 1M`). Channel is at capacity. `close_session()` calls `drop(master)`, then `reader_handle.join()` hangs indefinitely. The entire app UI thread (or tokio task) blocks. App freezes.
- **Evidence:** Phase 02 close_session comment: "drop master BEFORE joining reader: SIGHUP → child exits → PTY slave closes → cloned reader gets EOF → reader thread exits". This causal chain is optimistic and ignores the blocking_send race.
- **Suggested fix:** Before `drop(master)`, drain or discard the channel: close the `Receiver` side (or replace it with a dummy) so `blocking_send` returns `Err(SendError)` immediately, unblocking the reader thread. Then `drop(master)`. Then `join()`. Alternatively use `blocking_send` with a timeout, or use a shutdown `AtomicBool` flag checked in the reader loop.

---

## Finding 8: `vte::Params` type — `csi_dispatch` params are NOT `&[&[u16]]`; plan's SGR parsing logic against wrong type

- **Severity:** High
- **Location:** Phase 02, "Implementation Steps" step 2 — `csi_dispatch` SGR parsing description
- **Flaw:** The plan describes SGR parsing as "Iterate params with cursor index (for multi-byte 38/48 sequences)". The verified `csi_dispatch` signature in vte 0.13.1 is:
  ```rust
  fn csi_dispatch(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char)
  ```
  `Params` is an opaque iterator type from the `vte_generate_state_changes` / `vte` internal types — NOT a simple `&[&[u16]]` or `&[u16]`. To access subparams (e.g., `38;2;r;g;b` or `38;5;n`), the implementer must use `params.iter()` which yields `Subparams` iterators. The plan describes "iterate params with cursor index" and "check params[0] == 38/48" — using direct indexing which does NOT work on the `Params` type. This will cause a compile error or require the implementer to figure out the correct iterator API with no guidance.
- **Failure scenario:** Implementer writes `params[0]` — compile error. Or they discover `Params::iter()` and `Subparams` but mishandle sub-param iteration for `38;2;r;g;b`, resulting in truecolor sequences producing wrong colors (or being skipped entirely), breaking any terminal app that uses truecolor output (modern vim themes, bat, delta, etc.).
- **Evidence:** Phase 02: "Iterate params with cursor index (for multi-byte 38/48 sequences)" and `params[0] == 38/48` — implies slice indexing semantics that the opaque `Params` type does not support.
- **Suggested fix:** Document the actual `Params` iteration pattern: `let mut iter = params.iter(); while let Some(subparam) = iter.next() { let p = subparam[0]; ... }`. Show concrete handling for `38;5;n` (two subparams) vs `38;2;r;g;b` (four subparams). This is the most error-prone part of the implementation and the plan leaves it entirely to the implementer's guesswork.

---

## Unresolved Questions

1. `Subscription::run_with` takes `fn(&D) -> S` — the plan's entire subscription design requires refactoring. Is there a confirmed working pattern for bridging an `mpsc::Receiver` through this API that has been prototyped, or is this speculative?
2. PTY slave fd leakage on `close_session`: does `portable-pty 0.9` close the slave fd in the `Child` when `child.wait()` is called, or does the implementer need to explicitly drop slave before spawning? The plan does not address slave fd lifetime at all.
3. `canvas::Cache::draw()` return type: Phase 04 shows `draw()` calling `state.cache.draw(renderer, bounds.size(), |frame| {...})` and returning `Vec<Geometry>`. In iced 0.14, `canvas::Cache::draw()` returns a single `Geometry`, not `Vec<Geometry>`. The outer `draw()` must return `vec![geom]`. This is a compile-time error if the crate's return type changed — needs verification against iced 0.14.0 source.

