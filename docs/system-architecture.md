# System Architecture

Last updated: 2026-03-04

## Topology
```text
chatminal-app (native client)
  -> local IPC (UDS / Named Pipe)
chatminald (daemon)
  -> portable-pty sessions
  -> sqlite store (profiles/sessions/scrollback)
```

## Runtime flow
1. Client connect daemon endpoint.
2. Client gọi `workspace_load` để hydrate profiles/sessions.
3. Client activate session để daemon attach/spawn PTY.
4. Client gửi input/resize; daemon trả event output/exited/error.
5. Daemon batch persist scrollback vào SQLite.

## Main components
- Client: command bridge + wezterm-term pane state + TUI/dashboard rendering.
- Daemon: request parser, session lifecycle, persistence, health events.
- Shared protocol/store crates: contract và storage reuse cho cả hai app.

## Data model
- Tables: `profiles`, `sessions`, `scrollback`, `app_state`, `session_explorer_state`.
- Active key: `active_profile_id`, `active_session_id:{profile_id}`.
