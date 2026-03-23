# Chatminal

Chatminal hiện chạy theo mô hình `single-runtime desktop`: cửa sổ mặc định là `Chatminal Desktop` first-party với runtime session/profile/history nhúng trực tiếp trong process GUI, có sidebar profile/session bên trái. Repo hiện không còn daemon/CLI compatibility path trong workspace active; đường chạy chính thức là desktop app.

## Runtime hiện tại
- Desktop app: `apps/chatminal-desktop`
- Runtime lõi: `crates/chatminal-runtime`
- Shared persistence: `crates/chatminal-store` (SQLite)
- Terminal core: `crates/chatminal-terminal-core`
- Native vendored deps: `vendor/terminal-deps`

## Cấu trúc repo
- `apps/chatminal-desktop/`: desktop app first-party, runtime chính cho window
- `crates/chatminal-runtime/`: session/profile/history/explorer/runtime state và runtime DTO/event native
- `crates/chatminal-terminal-core/`: terminal parser/state nội bộ
- `crates/chatminal-store/`: SQLite store (profiles/sessions/scrollback)
- `docs/`: tài liệu kiến trúc, roadmap, changelog

## Yêu cầu
- Rust stable (khuyến nghị >= 1.93)
- Linux/macOS
- Lần build GUI đầu tiên sẽ hydrate các C deps vendored còn thiếu vào `vendor/terminal-deps/` qua `scripts/bootstrap-terminal-vendor-deps.sh`

## Chạy local
Mở desktop app:
```bash
make window
```

Hoặc gọi trực tiếp:
```bash
CHATMINAL_DESKTOP_SESSIONS_SIDEBAR=1 \
cargo run --manifest-path apps/chatminal-desktop/Cargo.toml -- start -- chatminal-runtime proxy-desktop-session
```

Hydrate vendor deps trước nếu cần:
```bash
make bootstrap-terminal-deps
make verify-third-party-reference-only
```

## Lệnh hỗ trợ
```bash
make clean-data
make window
make bootstrap-terminal-deps
make verify-third-party-reference-only
make check
make check-desktop
make test
```

## Biến môi trường
- `CHATMINAL_DATA_DIR`
- `CHATMINAL_PREVIEW_LINES`
- `CHATMINAL_MAX_LINES_PER_SESSION`
- `CHATMINAL_DEFAULT_SHELL`
- `CHATMINAL_DEFAULT_COLS`
- `CHATMINAL_DEFAULT_ROWS`
- `CHATMINAL_HEALTH_INTERVAL_MS`
- `CHATMINAL_INPUT_PIPELINE_MODE`

## Validate
```bash
cargo check --workspace
cargo check -p chatminal-desktop
cargo test -p chatminal-runtime
cargo test --manifest-path crates/chatminal-store/Cargo.toml
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
- [Release Checklist](./docs/release-checklist.md)
