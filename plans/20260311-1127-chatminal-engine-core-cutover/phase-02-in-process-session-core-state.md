# Phase 02 - In-Process Session Core State

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_core_state.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_surface.rs

## Overview
- Priority: P0
- Status: completed
- Brief: dựng state/store nội bộ cho surface/leaf/layout/process registry để phase sau gắn PTY runtime thật mà không phải mượn `mux` làm source of truth

## Key Insights
- Trước khi thay execution core thật cần một state store nội bộ ổn định cho session engine mới
- `SessionCoreState` phải quản reverse-index `session_id -> surface_id` và snapshot runtime của từng leaf
- Phase này an toàn nếu chỉ thêm store bootstrap, chưa cắm vào runtime path hiện tại

## Requirements
- Functional: có state store cho surface/leaf/process metadata và sync layout cơ bản
- Non-functional: không đổi behavior runtime hiện tại; tests hiện có phải pass

## Architecture
- `SessionCoreState` là source of truth tương lai cho session engine in-process
- `SurfaceRuntimeState` giữ mapping session/surface/layout/active leaf
- `LeafRuntimeState` + `LeafProcessState` là slot metadata cho PTY/process layer Phase 03

## Related Code Files
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_core_state.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs

## Current Progress
- Đã tạo `SessionCoreState`, `SurfaceRuntimeState`, `LeafRuntimeState`, `LeafProcessState`
- Đã có API bootstrap: `register_surface`, `remove_surface`, `surface_id_for_session`, `sync_surface_layout`, `set_leaf_process`
- Đã có unit tests cho reverse-index và layout pruning
- Đã cắm `SessionCoreState` vào `SessionEngine` runtime path qua `StatefulSessionEngine` và shared per-window state trong desktop helper; hiện mới là state song song, chưa thay execution core

## Todo List
- [x] Add in-process surface/leaf/process state store
- [x] Add unit tests for registry/layout sync basics
- [x] Wire `SessionCoreState` into session engine runtime path
- [x] Introduce real leaf runtime handles in Phase 03 bootstrap slice

## Success Criteria
- Có state store độc lập khỏi `mux` cho session engine mới
- Store sync được layout và leaf/process metadata cơ bản
- Không có regression ở desktop/session-runtime tests

## Next Steps
- Phase 04 sẽ dùng state store này làm metadata/source of truth khi cắt command path khỏi `mux`
