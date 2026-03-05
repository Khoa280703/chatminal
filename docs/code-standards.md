# Code Standards

Last updated: 2026-03-05
Scope: `apps/*` + `crates/*`.

## Principles
1. Keep daemon as single source of truth for session/profile/history state.
2. Keep protocol crate stable; breaking changes phải cập nhật đồng bộ client + daemon + docs.
3. Keep PTY hot path non-blocking; DB write đi qua batching path.
4. Keep code straightforward, avoid feature creep.

## Boundaries
- `apps/chatminald`: session manager, PTY runtime, IPC server, event publish.
- `apps/chatminal-app`: native client commands, wezterm-term state adapter, dashboard/TUI.
- `crates/chatminal-protocol`: request/response/event models.
- `crates/chatminal-store`: SQLite schema + CRUD store.

## Naming
- Rust: dùng `snake_case` cho function/field; tên type dùng kiểu CamelCase chuẩn của Rust.
- Protocol fields: giữ `snake_case` để đồng nhất serde payload.

## Runtime rules
1. Client không spawn shell trực tiếp.
2. Daemon chỉ expose local IPC endpoint.
3. Session reconnect thông qua daemon command path.
4. History retention phải đi qua store policy (`max lines`).
5. Input pipeline invariant:
   - `attach` và `window` phải dùng cùng semantic key/text mapping contract.
   - `CHATMINAL_INPUT_PIPELINE_MODE=legacy|wezterm` chỉ đổi client-side translation path, không được bypass daemon.
   - IME commit dedupe phải giữ nguyên rule: không double-send giữa text-event và ime-commit-event tương ứng.
6. Window backend invariant:
   - command public luôn đi qua `window-wezterm-gui`.
   - `CHATMINAL_WINDOW_BACKEND=wezterm-gui|legacy` chỉ đổi UI runtime path; không được bypass daemon ownership.
   - fallback/rollback phải verify bằng script migration (`phase08`).

## Validation commands
```bash
cargo check --workspace
cargo test --manifest-path crates/chatminal-protocol/Cargo.toml
cargo test --manifest-path crates/chatminal-store/Cargo.toml
cargo test --manifest-path apps/chatminald/Cargo.toml
cargo test --manifest-path apps/chatminal-app/Cargo.toml
```
