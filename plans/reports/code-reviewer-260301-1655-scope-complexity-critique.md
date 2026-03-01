# Scope & Complexity Critique — Chatminal Plan
**Reviewer:** Scope & Complexity Critic (YAGNI/KISS/DRY enforcer)
**Date:** 2026-03-01
**Plan:** `plans/260301-1521-chatminal-messenger-terminal/`

---

## Finding 1: `Arc<Mutex<Option<Receiver>>>` — Unnecessarily Complex Subscription Bridge

- **Severity:** High
- **Location:** Phase 03, section "Architecture" (`AppState.update_rx` field) + Implementation Step 4 (`subscription()`)
- **Flaw:** The plan wraps the `mpsc::Receiver` in `Arc<Mutex<Option<...>>>` to work around the fact that `subscription()` takes `&self`. The `Option` layer adds a one-time `take()` dance with duplicated-init detection via `log::warn!`. This is 3 layers of wrapper (`Arc`, `Mutex`, `Option`) to solve a problem that does not need to exist. In Iced 0.14, the subscription closure receives a unique key and is expected to be a generator; the receiver could instead be held in a `once_cell::sync::OnceCell` or initialized in `boot()` and passed directly into the subscription closure via capture — no mutex needed. Alternatively, the entire channel could be created inside the `Subscription::run_with` closure. The current approach creates a poisonable mutex in a hot path called every render frame.
- **Failure scenario:** If the Iced runtime ever calls `subscription()` before the previous subscription stream completes (e.g. during theme reload or window focus change), the `take()` returns `None`, subscription silently stops receiving PTY events, and the UI freezes with no error surfaced to user.
- **Evidence:** `update_rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<SessionEvent>>>>` — plan itself notes "handle duplicate init with `log::warn!` instead of panic", acknowledging the fragility.
- **Suggested fix:** Create the channel inside `Subscription::run_with` closure (pass `Sender` to `SessionManager` at creation time via `boot()`). The `Receiver` lives entirely inside the subscription future — no `Arc<Mutex<Option>>` needed.

---

## Finding 2: `primary_grid` + `alternate_grid` Dual-Buffer — Premature Implementation

- **Severity:** High
- **Location:** Phase 02, Todo List item: "`TerminalGrid`: add `primary_grid` + `alternate_grid` buffers, `use_alternate: bool` field"
- **Flaw:** The plan calls for a full dual-buffer `TerminalGrid` (primary + alternate grids as separate structs plus a `use_alternate` flag). This is correct terminal emulator behavior but vastly over-engineers the data model for an MVP. The stated goal of Phase 02 is "spawn PTY shells, parse ANSI output into TerminalGrid, stream updates to UI." Supporting `?1049h` (alternate screen for vim/htop) requires: switching the active buffer reference in `PtyPerformer`, correctly saving/restoring cursor state, and ensuring `draw()` renders from the correct buffer. Each of these is a distinct correctness hazard. An MVP that only serves bash (no vim, no htop) does not need alternate screen. The plan has already accepted "alternate screen not handled → vim/htop broken" as a prior red-team finding (Finding 7), then commits to full alternate screen support — adding 30-50 lines of grid-swap logic and a second 60KB allocation per session.
- **Failure scenario:** Implementer wires `primary_grid`/`alternate_grid` but makes a subtle cursor-save bug. vim opens, renders corrupted output. Debugging this is not straightforward — it's a state machine correctness issue requiring cross-phase tracing.
- **Evidence:** Phase 02 todo: "add `primary_grid` + `alternate_grid` buffers, `use_alternate: bool` field" — not in Requirements section (which only says `alternate_screen: bool`), but promoted to a full todo item.
- **Suggested fix:** For MVP, keep `TerminalGrid` as a single grid. Add a `// TODO: alternate screen (Phase N+1)` comment in `esc_dispatch`. Drop the dual-buffer todo from Phase 02 scope. vim/htop are "nice to have" at MVP — they are not listed in Requirements.

---

## Finding 3: `Alt+[1-9]` Session Switch Shortcuts — Undeclared Scope Creep

- **Severity:** Medium
- **Location:** Phase 05, section "Overview" / "Key Insights": `Alt+[1-9]=switch session`
- **Flaw:** The plan defines only two hard-coded shortcut constants (`SHORTCUT_NEW = "n"`, `SHORTCUT_CLOSE = "w"`), but the Key Insights list also includes `Alt+[1-9]=switch session` as a default shortcut. This feature does not appear in Requirements, has no implementation step, no todo item, and no test case. It is mentioned once in Key Insights then silently dropped. Either it is in scope (and must be implemented — adding an index lookup into `SessionManager.sessions: IndexMap` by insertion order) or it is out of scope and should not be in Key Insights. Having it in Key Insights implies an implementer will code it ad-hoc without plan coverage, resulting in inconsistent behavior (does Alt+9 wrap? what if fewer than 9 sessions exist? does it count from 0 or 1?).
- **Failure scenario:** Implementer sees `Alt+[1-9]` in Key Insights, codes it in `handle_keyboard()` without off-by-one analysis. Alt+1 selects index 0 on some days and panics on others depending on `IndexMap` iteration.
- **Evidence:** Phase 05, Key Insights: "Default shortcuts: `Alt+N`=new session, `Alt+W`=close session, `Alt+[1-9]`=switch session" — but only `SHORTCUT_NEW` and `SHORTCUT_CLOSE` constants are defined, and Requirements only mentions `Alt+N` and `Alt+W`.
- **Suggested fix:** Either remove `Alt+[1-9]` from Key Insights entirely (YAGNI — sidebar click covers session switching), or add it to Requirements + Implementation Steps with bounds handling. No half-mentions.

---

## Finding 4: Generation Counter Cache Invalidation — Two-Step Indirection Over-Complex

- **Severity:** Medium
- **Location:** Phase 04, Architecture section ("Generation counter pattern — CANONICAL cache invalidation approach") + Implementation Step 3 and 5
- **Flaw:** The plan introduces a two-step generation counter: `app.update()` bumps `terminal_canvas.generation`, then `canvas::Program::update()` detects `self.generation != state.last_generation` and clears `state.cache`. This is a workaround for not being able to directly clear `TerminalCanvasState.cache` from `app.update()`. The plan documents this with 8 lines of explanation ("WHY NOT clear from app.update() directly"). The complexity exists solely because `TerminalCanvas` is stored in `AppState` while `TerminalCanvasState` is owned by the Iced canvas machinery. A simpler design: do NOT store `TerminalCanvas` in `AppState` — instead, reconstruct `TerminalCanvas` in `view()` with the current grid, and let `canvas::Cache` live inside `AppState` directly. Then `app.update()` can call `self.canvas_cache.clear()` directly. No generation counter, no two-phase sync, no missed-generation bug risk.
- **Failure scenario:** Developer forgets to bump `generation` in one `update()` arm (e.g. `SelectSession`). Canvas cache never clears. User switches sessions, still sees prior session output. Bug is visually obvious but causally non-obvious — wrong code path in `update()`, not in `draw()`.
- **Evidence:** Phase 04: "generation bump signals `canvas::Program::update()` to do the actual clearing" — plan acknowledges the indirection is a workaround, not a clean design.
- **Suggested fix:** Store `canvas::Cache` in `AppState` directly (if Iced API permits). Call `self.canvas_cache.clear()` wherever a redraw is needed. Zero indirection, one fewer struct field per phase.

---

## Finding 5: `ansi_colours` Crate — Unnecessary External Dependency

- **Severity:** Medium
- **Location:** Phase 01, Implementation Step 2 (Cargo.toml): `ansi_colours = "1"` + Phase 04, `color_palette.rs`
- **Flaw:** The plan adds `ansi_colours = "1"` to Cargo.toml (Phase 01), but then Phase 04 immediately defines its own `XTERM_PALETTE: [u32; 256]` const array and `indexed_to_rgb()` function in `color_palette.rs`. This is a direct contradiction: the crate is declared but the plan implements the same functionality from scratch. The `ansi_colours` crate exists to convert between ANSI indices and sRGB values — exactly what `color_palette.rs` does manually. The plan ends up with both a dead dependency and a hand-rolled duplicate. Even if the crate is used, adding a crate dependency for a 256-element lookup table is over-engineering — the table is a 1KB static array, well within "copy-paste acceptable" territory.
- **Failure scenario:** `ansi_colours` is compiled into the binary, increasing binary size and supply chain surface area, but never called. `cargo deny` (also in Phase 01) then requires a license allowlist entry for it. Reviewer/auditor finds unused import warning (`cargo clippy -- -D warnings` in Phase 07 will fail on dead code).
- **Evidence:** Phase 01 Cargo.toml: `ansi_colours = "1"` with comment "256-color palette — dùng crate, không hardcode 256 values". Phase 04: `pub const XTERM_PALETTE: [u32; 256] = [...]` — hardcodes the 256 values anyway.
- **Suggested fix:** Remove `ansi_colours` from Cargo.toml entirely. Keep `color_palette.rs` with the static array — it is simpler, zero-dependency, and already planned. Pick one approach, not both.

---

## Finding 6: `resize_all_sessions` on Every `WindowResized` — Correctness vs. Complexity Trade-off Not Justified

- **Severity:** Medium
- **Location:** Phase 05, Requirements + Architecture; Phase 07, Implementation Step 4 (`manager.rs`)
- **Flaw:** The plan mandates resizing ALL sessions on every `WindowResized` event to prevent "inactive vim/htop from getting wrong size." This is a legitimate correctness concern, but it introduces quadratic work per resize event: N sessions × PTY resize syscall. More critically, `WindowResized` fires continuously during drag-resize on most platforms, not just at drag-end. With 5 sessions, every pixel of window drag triggers 5 PTY resize syscalls and 5 `SIGWINCH` signals to child processes. For an MVP with likely 1-3 sessions this is tolerable, but the plan includes no debounce or rate-limiting. The stated alternative (resizing only on session switch) is simpler and correct for 95% of users who have one active session at a time — inactive sessions will be corrected when switched to.
- **Failure scenario:** User drags window corner. Iced fires 60+ `WindowResized` events per second. Each triggers `resize_all_sessions()` with 3 sessions → 180 PTY resize syscalls/second. htop running in an inactive session gets 60 `SIGWINCH` signals per second, causing repeated full redraws of its output and CPU spike visible in the active session.
- **Evidence:** Phase 05 Architecture: "Resize ALL sessions — prevents inactive vim/htop from getting wrong size." Phase 07: `resize_all_sessions(cols, rows)` listed as "primary resize path on WindowResized". No debounce mentioned anywhere in either phase.
- **Suggested fix:** For MVP: resize only the active session on `WindowResized`, resize inactive sessions lazily on `SelectSession`. This cuts syscall volume by N-1 with zero correctness penalty for the common case. Add debounce (e.g. 50ms) only if measured to be a problem.

---

## Finding 7: `cargo-deny` in Phase 01 — Gold-Plating for a Personal MVP Tool

- **Severity:** Medium
- **Location:** Phase 01, Security Considerations: "Add `cargo-deny` with allowlist to block unlicensed/compromised crates"
- **Flaw:** `cargo-deny` is a supply-chain governance tool used by projects shipping to production users or managing large dependency trees. For a personal MVP terminal emulator under active development, adding a crate allowlist in Phase 01 means: every new crate added in subsequent phases (serde, toml, dirs, libc added in Phases 03/07) must be manually allowlisted or the CI breaks. This is admin overhead with zero security benefit at MVP stage — the developer controls both the dependencies and the machine. `cargo audit` (vulnerability scanning) is justified; `cargo-deny` with allowlist is not.
- **Failure scenario:** Developer adds `dirs = "5"` in Phase 07. `cargo deny check` fails because `dirs` is not on the allowlist. Developer either disables deny (defeating the purpose) or spends time updating the allowlist instead of shipping features.
- **Evidence:** Phase 01 Security: "Add `cargo-deny` with allowlist to block unlicensed/compromised crates" — no allowlist content provided, meaning it will be built incrementally as crates are added, creating a speed bump for every phase.
- **Suggested fix:** Run `cargo audit` only (zero config, pure scanning). Defer `cargo-deny` to post-MVP when the dependency set is stable. Remove it from Phase 01 todo.

---

## Finding 8: `SessionStatus::Exited` + Sidebar Dead Indicator — Contradicts Auto-Removal Logic

- **Severity:** Medium
- **Location:** Phase 07, Implementation Step 2 (`session/mod.rs`: add `SessionStatus` enum) + Implementation Step 6 (sidebar: "Show dead indicator for `SessionStatus::Exited`")
- **Flaw:** The plan simultaneously implements (a) `SessionExited(id) → auto-dispatch CloseSession(id)` (sessions auto-removed immediately on exit) and (b) `SessionStatus::Exited` enum variant displayed as a gray "Exited" indicator in the sidebar. These are contradictory: if sessions are auto-removed immediately via `CloseSession`, `SessionStatus::Exited` is never visible — the session disappears from the sidebar before the next render. The `SessionStatus` enum becomes dead code. If the intent is to show a "Exited" tombstone before removal, the plan provides no duration, no dismiss mechanism, and no UI for it. Phase 07 Integration Test Checklist confirms auto-removal: "Type `exit` in Session 2 → session auto-removed from sidebar (no zombie process)."
- **Failure scenario:** Implementer adds `SessionStatus::Exited` + sidebar rendering branch, then auto-removes sessions in the same message handler. `SessionStatus::Exited` branch is unreachable dead code. `cargo clippy -- -D warnings` fires on unreachable pattern. Alternatively, implementer delays auto-removal to show the indicator, creating actual zombie UI entries that users must manually dismiss.
- **Evidence:** Phase 07 step 2: `pub enum SessionStatus { Active, Exited }` + step 6: "Show dead indicator for `SessionStatus::Exited`" vs. step 5: "`SessionExited(id)` message → auto-dispatch `CloseSession(id)`". Both cannot be true simultaneously.
- **Suggested fix:** Remove `SessionStatus` enum entirely. Auto-remove on `SessionExited` as planned. Drop the dead indicator (YAGNI — the session is gone, no indicator needed). This removes 1 enum, 1 struct field, 1 sidebar rendering branch, and the contradiction.

---

## Finding 9: `vte = "=0.13.1"` Exact Pin — Undocumented Maintenance Burden

- **Severity:** Medium
- **Location:** Phase 01, Implementation Step 2 (Cargo.toml): `vte = "=0.13.1"` + Key Dependencies note "pin exact version — Perform trait changes arity between minor"
- **Flaw:** Pinning `vte` to an exact version (`=0.13.1`) prevents `cargo update` from applying patch fixes and means any future security patch to `vte` (e.g. 0.13.2) requires a manual plan update. The stated reason is "Perform trait changes arity between minor" — but this is only relevant if the project intends to upgrade. For an MVP that will be coded once and never updated, the pin is fine. However, the plan provides no exit criterion for the pin: when is it safe to unpin? What does "arity change" mean concretely for the codebase? Without this, a future maintainer will either leave the pin forever (missing security patches) or attempt an upgrade without knowing what breaks. The plan also does not verify that 0.13.1 is actually the latest 0.13.x — it might already be 0.13.4, making the pin unnecessarily stale.
- **Failure scenario:** `vte 0.13.1` has a known parsing bug (e.g. panics on malformed SGR sequences). `vte 0.13.2` fixes it. Exact pin blocks the fix. PTY flood with malformed sequences causes panic in `pty_reader_thread`, killing the session silently.
- **Evidence:** Cargo.toml: `vte = "=0.13.1"` with comment "pin exact version". No verification step to confirm 0.13.1 is latest patch, no upgrade criteria.
- **Suggested fix:** Use `vte = "~0.13"` (tilde = patch updates only, major/minor locked). This allows `0.13.x` security patches while blocking `0.14` breaking changes. Add a TODO comment: "upgrade when Perform trait stabilizes in 0.14+".

---

## Finding 10: Scrollbar Indicator Rect in Phase 06 — Misclassified as "MVP-tier Optional"

- **Severity:** Low
- **Location:** Phase 06, Implementation Step 6: "Scroll indicator (optional, MVP-tier): render thin scrollbar rect on right edge"
- **Flaw:** The label "MVP-tier" directly contradicts "optional" — these two words cannot coexist in a plan. Either the scrollbar is required for MVP (in which case it belongs in Requirements and Todo List as a checkbox) or it is post-MVP (in which case it must not appear as a numbered implementation step). As written, an implementer will code it because it appears as Step 6 of 7, and the label "optional" will be ignored under time pressure. The scrollbar implementation adds: `scroll_fraction` calculation, a `fill_rectangle` call inside `draw()`, and a mental model of "4px from right edge" that must interact correctly with the per-cell `x` coordinate calculation. None of this is in the Todo List (Phase 06 todo: "Optional: scrollbar indicator rect" — marked checkbox but not required).
- **Failure scenario:** Implementer codes the scrollbar in Phase 06. The 4px rect overlaps the rightmost column of terminal text (off-by-4px in cell width calculation). Last column of every terminal line is obscured by scrollbar. Fix requires adjusting `terminal_pane_view` bounds to subtract scrollbar width — touching Phase 03 and 04 layout math.
- **Evidence:** Phase 06 Step 6: "Scroll indicator (optional, MVP-tier)..." — contradiction in terms. Todo: "Optional: scrollbar indicator rect" — present as a checkbox implies it will be tracked.
- **Suggested fix:** Remove Step 6 entirely from Phase 06. Move scrollbar to a post-MVP backlog note. Keep todo checkbox only if it becomes a hard requirement. YAGNI: mouse wheel scrolling is sufficient; a visual scrollbar is polish, not functionality.

---

**Unresolved Questions**
1. Does `Subscription::run_with` in Iced 0.14 actually support the pattern described, or does it require a different API (`subscription::channel`-equivalent)? The plan's previous red-team review rejected `subscription::channel` but the replacement API is asserted without a citation.
2. Is `canvas::Program::update()` called before or after `draw()` in Iced 0.14? The generation pattern assumes `update()` → `draw()` ordering; if reversed, cache is cleared after rendering, wasting one frame.
3. The plan lists 28h total effort across 7 phases. Phase 02 alone (PTY + vte + TerminalGrid + dual-buffer + full SGR) is listed at 6h. Is this realistic for a first-time Rust/iced developer, or does it assume expert-level familiarity with both libraries simultaneously?
