## Code Review Summary

### Scope
- Files:
  - `apps/chatminald/src/transport/windows.rs`
  - `apps/chatminal-app/src/ipc/transport/windows.rs`
  - `apps/chatminal-app/src/ipc/client.rs`
- Goal: re-check follow-up fixes for prior Critical/High findings (`endpoint disappearance`, `PIPE_NOWAIT/read behavior`, `timeout contract`).

### Verdict
- Critical: **none**.
- High: **still present** (2 items below).

### High Priority Findings
1. Residual endpoint handoff race on daemon listener.
- Evidence: `apps/chatminald/src/transport/windows.rs:90`, `apps/chatminald/src/transport/windows.rs:92`, `apps/chatminald/src/transport/windows.rs:94`.
- Why still high: on successful connect, listener `take()`s current pipe instance and returns stream before creating next listening instance. In that short window, client side can observe endpoint missing and fail connect.

2. Client connect path is single-shot around Wait/Create handoff.
- Evidence: `apps/chatminal-app/src/ipc/transport/windows.rs:63`, `apps/chatminal-app/src/ipc/transport/windows.rs:66`, `apps/chatminal-app/src/ipc/transport/windows.rs:77`, `apps/chatminal-app/src/ipc/transport/windows.rs:88`.
- Why still high: `WaitNamedPipeW` + `CreateFileW` has no bounded retry loop for transient handoff failures (`ERROR_FILE_NOT_FOUND`/busy race), so intermittent connection failures remain user-visible.

### Medium Priority Findings
1. Timeout contract remains best-effort, not equivalent behavior across transports.
- Evidence: `apps/chatminal-app/src/ipc/transport/windows.rs:22`, `apps/chatminal-app/src/ipc/transport/windows.rs:29`, `apps/chatminal-app/src/ipc/client.rs:38`, `apps/chatminal-app/src/ipc/client.rs:43`, `apps/chatminal-app/src/ipc/client.rs:72`.
- Note: follow-up fixed startup failure by tolerating `Unsupported`, but write operations can still block beyond request timeout intent.

### Resolved From Previous Review
1. Prior `PIPE_NOWAIT` read-disconnect risk on connected stream is addressed.
- Evidence: `apps/chatminald/src/transport/windows.rs:32`, `apps/chatminald/src/transport/windows.rs:34` (connected handle switched to `PIPE_WAIT` before stream handoff).

2. Prior severe disappearance pattern (drop-every-poll) is improved.
- Evidence: `apps/chatminald/src/transport/windows.rs:77`, `apps/chatminald/src/transport/windows.rs:78`, `apps/chatminald/src/transport/windows.rs:89` (pending instance retained across non-connected polls).

### Unresolved Questions
1. Should daemon pre-create next pending pipe before returning accepted stream to remove the handoff gap entirely?
2. Should client retry `CreateFileW` within the existing 3s budget before surfacing connect failure?
