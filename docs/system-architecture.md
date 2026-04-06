# System Architecture

Last updated: 2026-04-06

## Overview

Chatminal is a single-process desktop application with embedded runtime.

### Core Components

- `apps/desktop`: Desktop application (single-process model)
- `crates/runtime`: Session lifecycle, PTY management, persistence
- `crates/store`: SQLite database for profiles, sessions, history
- `crates/terminal-emulator`: Terminal parser, state, and input handling
- `crates/lua-bridge`: Lua scripting integration
- `crates/codec`: Data serialization

## Architecture Diagram

```
┌─────────────────────────────────────┐
│         Desktop App                  │
│  ┌─────────────────────────────────┐│
│  │  UI Layer                        ││
│  │  - Window/Shell                  ││
│  │  - Sidebar/Modals                ││
│  │  - Render/Input                  ││
│  └─────────────────────────────────┘│
│              │                       │
│  ┌─────────────────────────────────┐│
│  │  Runtime                        ││
│  │  - Session Management            ││
│  │  - PTY Spawn/Control             ││
│  │  - Event Bus                     ││
│  │  - Execution Engine              ││
│  └─────────────────────────────────┘│
│              │                       │
│  ┌─────────────────────────────────┐│
│  │  Store                          ││
│  │  - SQLite Persistence            ││
│  │  - Profiles/Sessions/History     ││
│  └─────────────────────────────────┘│
└─────────────────────────────────────┘
```

## Key Design Decisions

### Single-Process Model

- No daemon or IPC: runtime is fully embedded in the desktop process
- Session lifecycle (create, resume, close) is all in-process
- Database connection is shared via `Arc<Mutex<Connection>>`
- Background persist worker handles database writes with zero lock contention

### Session Architecture

```
app
└── profiles
    └── sessions
        ├── runtime
        ├── terminal instances
        └── render target / view binding
```

### Performance

- 3K scrollback lines per session (~22MB RAM)
- 512KB output history per session
- Non-blocking PTY hot path
- Async persist worker for database operations

## Runtime Flow

### Startup

1. Desktop app boots
2. Runtime initializes and loads persisted state from store
3. Active session is hydrated into desktop shell
4. Sidebar/render state subscribes to runtime events

### Session Activation

1. User selects session or terminal via UI
2. Desktop resolves session ID / terminal handle
3. Runtime activates/focuses the session
4. Desktop shell updates sidebar/render state

### Session Output

1. PTY output flows into runtime execution engine
2. Runtime publishes event/snapshot updates
3. Desktop materializes pane/render information
4. Terminal window and sidebar repaint

## Persistence

SQLite stores:
- Profiles
- Sessions
- Canonical scrollback/history
- Workspace layout state
- Startup recipes and lifecycle preferences

## Lua Bridge

The Lua bridge allows scripting integration:

- Query current session/terminal state
- Spawn new sessions or splits
- Activate sessions/terminals
- Read terminal metadata, lines, dimensions

## Invariants

- Single runtime crate: `runtime`
- Desktop does not own execution registry separately from runtime
- UI shell does not infer active execution outside canonical bindings
- Documentation reflects current single-app architecture
