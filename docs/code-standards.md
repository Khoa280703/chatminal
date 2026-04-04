# Code Standards

Last updated: 2026-04-04 (single terminal architecture)
Scope: `apps/chatminal-desktop` + `crates/*`.

## Principles
1. Keep desktop as single source of truth for session/profile/history state (single-process).
2. Keep protocol crate stable (if protocols re-emerge in future); breaking changes phải cập nhật đồng bộ + docs.
3. Keep PTY hot path non-blocking; DB write đi qua async persist worker (background thread, zero lock contention).
4. Keep code straightforward, avoid feature creep.

## Boundaries
- `apps/chatminal-desktop`: desktop app, session manager, PTY runtime, GUI shell, native API.
- `crates/chatminal-runtime`: embedded orchestrator, session state, startup recipes, persist worker, store facade.
- `crates/chatminal-store`: SQLite schema + CRUD store.
- `crates/chatminal-terminal-emulator`: terminal parser/state/input core canonical cho active product path.
- `crates/chatminal-host-runtime`: private engine primitives (Mux/Tab/Pane, not public API).

## Naming
- Rust: dùng `snake_case` cho function/field; tên type dùng kiểu CamelCase chuẩn của Rust.
- Protocol fields: giữ `snake_case` để đồng nhất serde payload.

## Runtime rules
1. Desktop app không spawn shell bên ngoài process; PTY tạo thông qua LeafRuntime.
2. Session lifecycle hoàn toàn nằm trong desktop process (no IPC).
3. History retention phải đi qua store policy (`max lines`, output history cap 512KB).
4. Persist writes phải đi qua background worker thread (zero lock contention on hot path).
5. Startup recipes invariant:
   - Per-session registry, persisted in SQLite.
   - Executed before first prompt (run/type/enter/wait/wait-for sequence).
   - Available for inspection via native API.
6. Desktop invariant:
   - Single Window, single Tab, multiple Panes mapped to sessions.
   - Session owns Pane via `session_id → Arc<ChatminalSessionPane>` lookup.
   - No public host Tab/Pane surface (private adapter zone only).

## Validation commands
```bash
cargo check --workspace
cargo check -p chatminal-desktop
cargo test --manifest-path crates/chatminal-runtime/Cargo.toml -- --test-threads=1
cargo test --manifest-path crates/chatminal-store/Cargo.toml
cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1
```
