# System Architecture

Last updated: 2026-03-05

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
6. Khi input queue đầy, daemon áp dụng backpressure policy:
   - text input thường: drop ngay + trả lỗi rõ cho client
   - control input (Ctrl+C, Ctrl+Z, phím delete): retry bounded trước khi drop
   - metrics ghi lại qua `input_queue_full_total`, `input_retry_total`, `input_drop_total`

## Main components
- Client: command bridge + internal terminal core pane state + TUI/dashboard + native window runtime `window`.
- Daemon: request parser, session lifecycle, persistence, health events + runtime metrics instrumentation.
- Input backpressure runtime: request ghi input của session vẫn giữ daemon-first invariant, có ưu tiên control-key path và telemetry counters để phục vụ soak/incident analysis.
- Transport layer:
  - daemon: `transport/unix.rs` (UDS) + `transport/windows.rs` (Named Pipe)
  - client: `ipc/transport/unix.rs` (UDS) + `ipc/transport/windows.rs` (Named Pipe)
  - abstraction boundary giữ frame loop không phụ thuộc platform.
- Shared protocol/store crates: contract và storage reuse cho cả hai app.
- Perf gates: `bench-rtt-wezterm` command + `scripts/bench/phase02-rtt-memory-gate.sh` để đo RTT/RSS theo KPI.

## Data model
- Tables: `profiles`, `sessions`, `scrollback`, `app_state`, `session_explorer_state`.
- Active key: `active_profile_id`, `active_session_id:{profile_id}`.
- Runtime data directory hỗ trợ override bằng `CHATMINAL_DATA_DIR` (daemon/app/store).
