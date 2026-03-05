# code-reviewer-260305-window-delay-char-review

Date: 2026-03-05
Reviewer: code-reviewer
Work context: `/home/khoa2807/working-sources/chatminal`
Scope:
- `apps/chatminal-app/src/window/native_window_wezterm_actions.rs`
- `apps/chatminal-app/src/window/native_window_wezterm.rs`
- `apps/chatminal-app/src/terminal_wezterm_core.rs`

## Edge-case scouting (pre-review)
Scouting source:
- `git diff --name-only HEAD~1`
- dependency trace by symbol usage (`run_window_wezterm`, `visible_text`, input write path)

Scout findings:
- Input path in window app is synchronous request/response on UI frame loop.
- Render cache refresh happens before per-frame input handling, then can fall back to idle repaint interval.
- `terminal_wezterm_core::visible_text` now preserves trailing spaces; downstream renderer copies full snapshot text each repaint.

## Findings (ordered by severity)

### High
1. **UI thread can block up to input timeout on each send, causing typing lag/jank**  
   - File: `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:345`  
   - Related lines: `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:377`, `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:383`, `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:391`  
   - Why: `send_terminal_payload` executes blocking `write_input_for_session_with_timeout(..., 1000ms)` in the UI update path. Under daemon hiccup/backpressure, frame thread stalls, visible as delay per char.
   - Impact: degraded interactivity; repeated key presses can feel frozen.
   - Recommendation: move input write to async/background worker or non-blocking queue; keep UI thread only for enqueue + optimistic repaint state.

2. **Repaint ordering still allows up to idle-interval char visibility delay**  
   - File: `apps/chatminal-app/src/window/native_window_wezterm.rs:136`  
   - Related lines: `apps/chatminal-app/src/window/native_window_wezterm.rs:162`, `apps/chatminal-app/src/window/native_window_wezterm.rs:166`, `apps/chatminal-app/src/window/native_window_wezterm.rs:169`, `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:387`  
   - Why: frame does `refresh_cached_terminal_text()` before input handling. After sending input, a subsequent frame can clear `render_dirty` before PTY echo arrives, then scheduler drops to idle repaint (`120ms`). Echo arriving after that frame may wait until next idle tick.
   - Impact: intermittent 1-frame+ latency spikes (perceived delayed char echo).
   - Recommendation: process input before cache refresh or hold active repaint for a short grace window after successful input write / until output event observed.

### Medium
1. **Legacy duplicate-char suppression is order-dependent and may miss common key->text event ordering**  
   - File: `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:205`  
   - Related lines: `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:209`, `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:223`, `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:235`  
   - Why: suppression checks `saw_text_like_event` only for key events processed *after* text-like events. If Egui emits `Key` first then `Text`, both payloads may still be appended.
   - Impact: possible duplicate printable chars in `legacy` mode.
   - Recommendation: pre-scan frame for text-like events (or two-pass processing) before appending printable key payloads.

2. **Trailing-space fidelity change may increase repaint cost under large terminal sizes**  
   - File: `apps/chatminal-app/src/terminal_wezterm_core.rs:108`  
   - Related lines: `apps/chatminal-app/src/window/native_window_wezterm.rs:149`  
   - Why: removing `trim_end_matches(' ')` preserves right-padding spaces; UI clones full `cached_terminal_text` into a `TextEdit` every redraw.
   - Impact: higher CPU/memory churn in heavy output scenarios.
   - Recommendation: keep fidelity but consider rendering widget optimized for immutable text or avoid per-frame full-string clone.

## Positive observations
- Input batching exists (`buffered_payload`) to reduce request count per frame.
- IME dedupe integration prevents common text/commit double-send in non-legacy mode.
- Patch improves terminal fidelity for meaningful trailing spaces.

## Validation
- `cargo check --manifest-path apps/chatminal-app/Cargo.toml` => PASS
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml` => PASS (`61 passed, 0 failed`)

## Conclusion (high/critical)
- **Critical:** 0
- **High:** 2
- Patch is **not ready for sign-off** for delay/char objective until 2 high issues above are addressed.

## Unresolved questions
- None.
