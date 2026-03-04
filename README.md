# Chatminal

Chatminal hiện dùng kiến trúc native Rust theo hướng WezTerm-first.

## Runtime hiện tại
- Native client: `apps/chatminal-app` (dùng `wezterm-term` để giữ terminal state)
- Daemon: `apps/chatminald` (quản lý session/profile/PTY/history)
- Shared contracts: `crates/chatminal-protocol`
- Shared persistence: `crates/chatminal-store` (SQLite)

## Cấu trúc repo
- `apps/chatminal-app/`: native client CLI/TUI cho runtime WezTerm
- `apps/chatminald/`: daemon local IPC + PTY runtime
- `crates/chatminal-protocol/`: request/response/event types
- `crates/chatminal-store/`: SQLite store (profiles/sessions/scrollback)
- `docs/`: tài liệu kiến trúc, roadmap, changelog

## Yêu cầu
- Rust stable (khuyến nghị >= 1.93)
- Linux/macOS

## Chạy local
Terminal 1:
```bash
CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminald/Cargo.toml
```

Terminal 2:
```bash
CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminal-app/Cargo.toml -- dashboard-tui-wezterm 120 200 120 32 20
```

Các lệnh client khác:
```bash
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- workspace
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- sessions
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- create "Dev"
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- activate-wezterm <session_id> 120 32 200
```

## Biến môi trường
- `CHATMINAL_DAEMON_ENDPOINT`
- `CHATMINAL_PREVIEW_LINES`
- `CHATMINAL_MAX_LINES_PER_SESSION`
- `CHATMINAL_DEFAULT_COLS`
- `CHATMINAL_DEFAULT_ROWS`
- `CHATMINAL_HEALTH_INTERVAL_MS`

## Validate
```bash
cargo check --workspace
cargo test --manifest-path crates/chatminal-protocol/Cargo.toml
cargo test --manifest-path crates/chatminal-store/Cargo.toml
cargo test --manifest-path apps/chatminald/Cargo.toml
cargo test --manifest-path apps/chatminal-app/Cargo.toml
```

## Tài liệu
- [Docs Index](./docs/index.md)
- [System Architecture](./docs/system-architecture.md)
- [Codebase Summary](./docs/codebase-summary.md)
- [Code Standards](./docs/code-standards.md)
- [Deployment Guide](./docs/deployment-guide.md)
- [Project Roadmap](./docs/project-roadmap.md)
- [Development Roadmap](./docs/development-roadmap.md)
- [Project Changelog](./docs/project-changelog.md)
