# System Architecture

Last updated: 2026-03-01

## Overview
Chatminal is a single-process desktop app with event-driven UI state and per-session PTY worker threads.
The terminal runtime/parser is backed by `wezterm-term`; UI renders immutable `TerminalGrid` snapshots.

```text
+----------------------+         +---------------------------+
| Iced App Runtime     |<------->| SessionManager            |
| - AppState::update   | events  | - create/close/resize     |
| - AppState::view     |         | - send_input              |
| - subscriptions      |         | - shell validation        |
+----------+-----------+         +-------------+-------------+
           ^                                     |
           | SessionEvent via mpsc               |
           |                                     v
+----------+-------------------------------------+-----------+
| PTY Worker Threads (per session)                           |
| - reader: bytes -> wezterm advance_bytes -> snapshot grid  |
| - writer: input bytes channel -> PTY writer                |
+----------+-------------------------------------+-----------+
           |
           v
+----------------------+ 
| TerminalGrid         |
| - primary/alternate  |
| - scrollback deque   |
| - cursor style/pos   |
+----------------------+
```

## Components

### 1. Bootstrap
Files: `src/main.rs`, `src/config.rs`
- Initializes logger and ignores broken-pipe signal.
- Loads optional config from `~/.config/chatminal/config.toml`.
- Normalizes runtime numeric config values before app boot.

### 2. App State Machine
File: `src/app.rs`
- Owns active session ID, session grids, per-session scroll offsets.
- Maintains runtime cols/rows and cell metrics.
- Routes UI events + session events through `Message`.
- Uses `terminal_generation` to invalidate canvas cache.

### 3. Session Orchestration
Files: `src/session/manager.rs`, `src/session/mod.rs`
- Creates PTY, spawns shell process, and starts worker threads.
- Preserves stable session order in `IndexMap`.
- Applies shell allowlist validation and input-size bounds.
- Exposes APIs for create/close/resize/send input.

### 4. Terminal Runtime
Files: `src/session/pty_worker.rs`, `src/session/grid.rs`
- Reader thread feeds PTY bytes to `Terminal::advance_bytes`.
- Builds `TerminalGrid` snapshot from wezterm screen lines.
- Snapshot range is bounded to `scrollback window + visible window`.
- Computes `lines_added` from stable-row delta to preserve user scroll position.
- Maps cursor shape/visibility to `CursorStyle::{Block, Underline, Bar, Hidden}`.

### 5. UI Layer
Files: `src/ui/*`
- Sidebar for session listing/actions.
- Input mapper converts keyboard events to PTY byte sequences.
- Terminal canvas draws grid cells, background, underline, and cursor overlays.
- Color rendering uses `CellColor` emitted from runtime snapshots.

## PTY Output Flow
1. Reader thread reads bytes from PTY master.
2. `PtyEngine::advance_bytes()` forwards bytes to wezterm terminal state.
3. `snapshot_grid` captures scrollback + visible range into `TerminalGrid`.
4. Worker emits `SessionEvent::Update` via `try_send`.
5. If queue is full, worker keeps state dirty and retries latest snapshot later.
6. On EOF/read error, worker flushes once then spawns a sender thread.
7. Sender thread calls `blocking_send(SessionEvent::Exited(...))`.
8. App subscription maps events to `Message::TerminalUpdated` / `Message::SessionExited`.
9. App updates grid + scroll offsets and increments generation for redraw.

## Input Flow
1. Keyboard events enter `AppState::handle_event()`.
2. App-level shortcuts handled first (Alt+N, Alt+W, Shift+PageUp/PageDown).
3. Remaining keys go through `key_to_bytes`.
4. Bytes are enqueued with `SessionManager::send_input()`.
5. Writer thread writes and flushes bytes to PTY.

## Scrollback and Viewport Model
- `TerminalGrid` keeps primary grid, alternate grid, and scrollback deque.
- Alternate screen disables scrollback view (`offset = 0`).
- On new updates while user is scrolled up, offset is shifted by `lines_added` and clamped.
- Renderer builds visible rows from scrollback + active cells using current offset.

## Concurrency Model
- Main thread: Iced runtime + app state updates.
- Per session:
  - 1 blocking PTY reader thread
  - 1 blocking PTY writer thread
- Channels:
  - session event channel capacity: `64`
  - per-session input channel capacity: `16`

## Error and Shutdown Behavior
- Reader EOF/read error triggers `SessionEvent::Exited` through spawned sender thread.
- `Message::SessionExited` is transformed into `Message::CloseSession`.
- Close flow kills child process, drops handles, joins reader/writer threads.
- Channel full/closed and PTY/shell errors are surfaced via `SessionError` and logged.

## Security and Safety Controls
1. Ignore broken-pipe signal to avoid process-level termination.
2. Validate shell path against `/etc/shells` and executable permissions.
3. Reject oversized PTY input payloads.
4. Clamp PTY resize dimensions and config-driven numeric values.
5. Keep queue boundaries explicit to avoid unbounded memory growth.

## Current Gaps
1. No integration test suite for full lifecycle under sustained output.
2. No metrics/telemetry for render latency and queue pressure.
3. Linux/Unix assumptions still embedded in shell and signal handling paths.
