# code-reviewer-260305-wezterm-gui-rereview-r1

Date: 2026-03-05
Reviewer: code-reviewer
Work context: `/home/khoa2807/working-sources/chatminal`
Scope:
- `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs`
- `apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs`
- `scripts/smoke/window-wezterm-gui-smoke.sh`
- `Makefile`
- `README.md`

## Edge-case scouting (pre-review)
- Re-checked command wiring + IPC backlog behavior from dependents:
  - `apps/chatminal-app/src/main.rs`
  - `apps/chatminal-app/src/ipc/client.rs`
  - `apps/chatminal-app/src/ipc/client_runtime.rs`
- Re-ran focused validation:
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml terminal_wezterm_gui`
  - `bash scripts/smoke/window-wezterm-gui-smoke.sh`

## Remaining findings

### High
- None.

### Medium
1. Event fidelity can still degrade under sustained high throughput (input burst + heavy output).
- Evidence:
  - Proxy still sends synchronous `SessionInputWrite` per decoded chunk in hot loop: `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:88`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:96`.
  - Event polling is interleaved and budgeted, but finite (`INPUT_DRAIN_BUDGET=32`, `EVENT_DRAIN_BUDGET=128`): `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:14`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:15`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:80`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:123`.
  - Client backlog explicitly drops event frames when full to preserve responses: `apps/chatminal-app/src/ipc/client_runtime.rs:50`.
- Impact: in extreme workloads, `PtyOutput` frames may still be delayed/dropped.
- Recommendation: move input writes to async/batched path (reduce request-per-chunk pressure) and/or add stronger backpressure/fair scheduling policy that prevents event loss.

### Low
1. WezTerm binary resolver accepts any existing path, not explicitly executable file.
- Evidence: `apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs:130`, `apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs:139`.
- Impact: invalid override/path entry fails later at spawn with less actionable error.
- Recommendation: validate `is_file()` and executable bit (Unix) before selecting candidate.

## Conclusion
- High findings from prior review are fixed.
- Remaining risk: 1 medium + 1 low.

## Unresolved questions
- None.
