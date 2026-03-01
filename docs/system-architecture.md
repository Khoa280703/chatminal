# System Architecture

Last updated: 2026-03-01

## Architecture Overview
Chatminal uses a single-process desktop architecture with event-driven UI state and per-session PTY worker threads.

```text
+-------------------------+        +--------------------------+
| Iced Runtime            |        | SessionManager           |
| - AppState::update      |<------>| - sessions: IndexMap     |
| - AppState::view        |        | - create/close/resize    |
| - subscription streams  |        | - send_input             |
+-----------+-------------+        +------------+-------------+
            ^                                   |
            | SessionEvent via tokio mpsc       |
            |                                   v
+-----------+-----------------------------------+-------------+
| PTY Worker Threads (per session)                            |
| - reader thread: read PTY -> vte parser -> TerminalGrid     |
| - writer thread: input channel -> PTY writer                |
+-----------+-----------------------------------+-------------+
            |
            v
+-------------------------+
| TerminalGrid            |
| - primary buffer        |
| - alternate buffer      |
| - scrollback ring       |
+-------------------------+
```

## Component Responsibilities

### 1. Bootstrap Layer
Files: `src/main.rs`, `src/config.rs`
- Initializes logger and signal handling.
- Loads normalized config from `~/.config/chatminal/config.toml`.
- Clamps config bounds at load path (`scrollback_lines`, `font_size`, `sidebar_width`).
- Starts Iced application loop.

### 2. Application State Layer
File: `src/app.rs`
- Owns user-visible state:
  - active session
  - per-session terminal snapshot
  - per-session scroll offset
  - terminal metrics (cols, rows, font/cell)
- Handles all `Message` variants from UI and PTY events.
- Coordinates session lifecycle and terminal viewport behavior.
- Derives cell metrics from runtime `font_size` via `metrics_for_font` and uses them in resize math.

### 3. Session Orchestration Layer
Files: `src/session/manager.rs`, `src/session/mod.rs`
- Creates PTY pair and child shell.
- Starts and joins reader/writer threads.
- Enforces shell validation and input size limits.
- Exposes session ordering and metadata for sidebar.
- Maps bounded input queue failures to explicit `SessionError::ChannelFull` / `SessionError::ChannelClosed`.

### 4. Terminal Emulation Layer
Files: `src/session/grid.rs`, `src/session/pty_worker.rs`
- Parses terminal control sequences through `vte::Parser`.
- Applies cursor, erase, SGR, alternate buffer semantics to `TerminalGrid`.
- Emits immutable snapshots (`Arc<TerminalGrid>`) for UI rendering.
- Implements reverse index (`ESC M`) semantics:
  - top row -> `scroll_down(1)`
  - otherwise -> cursor row decrements by one
- Handles update queue backpressure by retrying latest dirty snapshot after `TrySendError::Full`.

### 5. Presentation Layer
Files: `src/ui/sidebar.rs`, `src/ui/input_handler.rs`, `src/ui/terminal_pane.rs`, `src/ui/color_palette.rs`, `src/ui/theme.rs`
- Sidebar: session list and session commands.
- Input mapping: translates keyboard events to PTY byte sequences.
- Terminal canvas: draws background, cell glyphs, style, cursor overlay, and handles wheel scroll.

## Data Flow

### Session Create Flow
1. User action (`+ New Session` or `Alt+N`) emits `Message::NewSession`.
2. `AppState::create_new_session()` calls `SessionManager::create_session()`.
3. Manager resolves shell, opens PTY, spawns child and threads.
4. New session id is marked active and viewport reset.

### PTY Output Flow
1. Reader thread reads bytes from PTY master reader.
2. `vte::Parser` drives `PtyPerformer` mutations on `TerminalGrid`.
3. Performer sends `SessionEvent::Update` with grid snapshot + `lines_added` using `try_send`.
4. If update queue is full, performer keeps state dirty and retries on next flush.
5. On EOF/read error, reader emits `SessionEvent::Exited` with `blocking_send`.
6. Subscription converts event to `Message::TerminalUpdated` / `Message::SessionExited`.
7. `AppState::update()` merges snapshot and adjusts scroll offset.
8. Terminal canvas redraws when generation changes.

### Input Flow
1. Keyboard event reaches `AppState::handle_event()`.
2. `key_to_bytes` converts key/modifier to PTY bytes.
3. `SessionManager::send_input()` pushes bytes to per-session input channel.
4. Writer thread flushes bytes to PTY writer.

## Concurrency Model
- Main thread: Iced update/view/subscription orchestration.
- Per session:
  - 1 reader thread (blocking read + parse)
  - 1 writer thread (blocking channel recv + write)
- Cross-thread communication:
  - `tokio::sync::mpsc` for session events and input payloads.
  - Event queue size: `64` (app boot channel).
  - Per-session input queue size: `16`.
- Shared UI data model for terminal snapshots uses `Arc<TerminalGrid>`.

## Error and Shutdown Behavior
- PTY read EOF/error emits `SessionEvent::Exited` via blocking send.
- `Message::SessionExited` triggers `Message::CloseSession`.
- Close path kills child, drops channels/PTY handles, joins worker threads.
- Oversized input or closed/full channel errors return `SessionError` and are logged.
- Update snapshots are coalesced under pressure (latest state retries when queue is full).

## Security-Relevant Controls
1. Process-level broken-pipe signal is ignored to prevent hard exit on broken PTY writes.
2. Shell path must pass filesystem checks and allowlist in `/etc/shells`.
3. Input payload size is bounded before enqueue.
4. PTY dimensions are clamped to valid `u16` range.
5. User-configurable numeric UI/runtime values are clamped to safe ranges before use.

## Current Architecture Gaps
1. No persistence boundary (all session/grid state in memory only).
2. No explicit telemetry/metrics pipeline for render/update latency.
3. No abstraction for non-Unix shell resolution and PTY semantics.
