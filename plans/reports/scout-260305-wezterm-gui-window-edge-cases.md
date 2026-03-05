# Scout Report - WezTerm GUI window edge cases

## Scope
- Work context: /home/khoa2807/working-sources/chatminal
- Changed files scoped:
  - apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs
  - apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs
  - scripts/smoke/window-wezterm-gui-smoke.sh
  - Makefile
  - README.md

## Affected dependents
- Command routing and help text:
  - apps/chatminal-app/src/main.rs
  - apps/chatminal-app/src/config.rs
- IPC behavior that new proxy depends on:
  - apps/chatminal-app/src/ipc/client.rs
  - apps/chatminal-app/src/ipc/client_runtime.rs
- Daemon request/event handlers used by proxy:
  - apps/chatminald/src/state/request_handler.rs
  - apps/chatminald/src/state/session_event_processor.rs
- CI still gating old backend only:
  - .github/workflows/rewrite-quality-gates.yml

## Data flow risks
1. UTF-8 boundary corruption risk (high)
- Proxy reads raw stdin bytes per chunk, then converts each chunk with `String::from_utf8_lossy` before `SessionInputWrite`.
- If multibyte UTF-8 char split across reads, replacement chars injected. This mutates user input.
- Source: apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:57-64,224-233.

2. Output starvation/drop under heavy input (high)
- Main loop drains entire input queue first, sends sync request per chunk, then polls events once.
- `ChatminalClient` backlog is bounded; overflow drops event frames preferentially.
- Under large paste/high output, `PtyOutput` can be delayed or dropped.
- Source: apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:76-93,112-131; apps/chatminal-app/src/ipc/client_runtime.rs:50-66.

3. First-run no-session failure path (medium)
- `make window` preflight only checks `workspace` responds.
- Launcher opens wezterm without explicit session; proxy resolves active/first session and errors if none.
- On fresh workspace with zero sessions, window opens then exits with error.
- Source: Makefile:63-67; apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:146-157.

4. CI blind spot for default path (medium)
- Default local command moved to `window-wezterm-gui`, but quality gate smoke job still runs old `window-wezterm-smoke.sh`.
- Regression in new default path can pass CI undetected.
- Source: .github/workflows/rewrite-quality-gates.yml:63-65.

## Boundary conditions
1. Exit key collision (medium)
- Proxy treats single byte `0x1d` (`Ctrl-]`) as hard exit.
- This key cannot be forwarded to applications inside terminal.
- Source: apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:78-80,220-222.

2. WezTerm binary resolution permissiveness (low)
- Resolver checks `exists()` only; may pick non-executable path then fail at spawn.
- Source: apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs:126-140.

3. Source-build fallback runtime dependency (low/medium)
- If no installed `wezterm`, fallback requires `third_party/wezterm` source + `cargo` runtime.
- Packaged runtime without cargo/toolchain will fail to launch.
- Source: apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs:45-77.

## Async races and state mutation notes
1. Session selection race (low/medium)
- Proxy picks active/first session from `WorkspaceLoad`, then activates later.
- Another client can switch/close session between those calls.
- Source: apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:146-173.

2. Daemon state mutations impacted by proxy path
- `SessionActivate` mutates active session in store.
- `SessionInputWrite` mutates runtime input queue metrics and can reject when queue full.
- `SessionEvent::Output` mutates `seq/live_output` and broadcasts `PtyOutput`.
- Source: apps/chatminald/src/state/request_handler.rs:116-152,216-247; apps/chatminald/src/state/session_event_processor.rs:24-45.

## Quick validation run
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml terminal_wezterm_gui` -> pass (7 tests)
- `bash scripts/smoke/window-wezterm-gui-smoke.sh` -> pass

## Unresolved questions
1. Should proxy preserve raw bytes end-to-end (protocol change to bytes/base64) or guarantee streaming UTF-8 boundary reconstruction before `SessionInputWrite`?
2. Should `make window` auto-create a session if workspace has none?
3. Should CI replace or add smoke gate for `window-wezterm-gui` now that it is default local entrypoint?
