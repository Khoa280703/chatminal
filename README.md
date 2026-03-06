# Chatminal

Chatminal hiện dùng kiến trúc native Rust với terminal core nội bộ.

## Runtime hiện tại
- Native client: `apps/chatminal-app` (dùng `chatminal-terminal-core` để giữ terminal state)
- Daemon: `apps/chatminald` (quản lý session/profile/PTY/history)
- Terminal core: `crates/chatminal-terminal-core`
- Shared contracts: `crates/chatminal-protocol`
- Shared persistence: `crates/chatminal-store` (SQLite)

## Cấu trúc repo
- `apps/chatminal-app/`: native client CLI/TUI/window
- `apps/chatminald/`: daemon local IPC + PTY runtime
- `crates/chatminal-terminal-core/`: terminal parser/state nội bộ
- `crates/chatminal-protocol/`: request/response/event types
- `crates/chatminal-store/`: SQLite store (profiles/sessions/scrollback)
- `docs/`: tài liệu kiến trúc, roadmap, changelog

## Yêu cầu
- Rust stable (khuyến nghị >= 1.93)
- Linux/macOS
- Không yêu cầu cài WezTerm runtime.

## Chạy local
Nhanh nhất (khuyến nghị):
```bash
make daemon
```

Mở terminal thứ 2:
```bash
make dashboard
```

Để tương tác gõ lệnh trực tiếp (interactive):
```bash
make attach
```
Thoát attach bằng `F10`.

Để mở app cửa sổ native (phase 01):
```bash
make window
```

`make window` mở native window của Chatminal.

Smoke cho native window (headless với xvfb):
```bash
make smoke-window
```

Benchmark nhanh RTT:
```bash
make bench-rtt
```

Hard gate Phase 02 (RTT + RSS):
```bash
make bench-phase02
```
Smoke nhanh (không fail cứng):
```bash
CHATMINAL_BENCH_ENFORCE_HARD_GATE=0 make bench-phase02
```

Phase 05 fidelity smoke (JSON report):
```bash
make fidelity-smoke
```

Phase 03 fidelity matrix smoke (JSON report):
```bash
make fidelity-matrix-smoke
```
Relaxed mode:
```bash
make fidelity-matrix-smoke-relaxed
```

Phase 06 input/modifier smoke + IME manual-gate report:
```bash
make fidelity-input-ime-smoke
```

Phase 05 soak smoke (JSON report):
```bash
make soak-smoke
```
Nightly soak 2h (stability gate, tách riêng RTT hard-gate):
```bash
CHATMINAL_SOAK_MODE=nightly CHATMINAL_SOAK_DURATION_SECONDS=7200 CHATMINAL_SOAK_REQUIRE_BENCH_HARD_GATE=0 make soak-smoke
```

Phase 05 release dry-run (artifacts + checksum + smoke):
```bash
make release-dry-run
```

Lệnh tắt khác:
```bash
make daemon-reset
make workspace
make sessions
make create NAME="Dev"
make activate SESSION_ID="<session_id>"
make attach SESSION_ID="<session_id>"
make window
make smoke-window
make bench-rtt
make bench-phase02
make fidelity-smoke
make fidelity-matrix-smoke
make fidelity-matrix-smoke-relaxed
make fidelity-input-ime-smoke
make phase06-killswitch-verify
make phase08-killswitch-verify
make soak-smoke
make release-dry-run
```

Nếu muốn chạy thủ công:

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
- `CHATMINAL_DATA_DIR`
- `CHATMINAL_PREVIEW_LINES`
- `CHATMINAL_MAX_LINES_PER_SESSION`
- `CHATMINAL_DEFAULT_SHELL`
- `CHATMINAL_DEFAULT_COLS`
- `CHATMINAL_DEFAULT_ROWS`
- `CHATMINAL_HEALTH_INTERVAL_MS`
- `CHATMINAL_INPUT_PIPELINE_MODE` (`wezterm` hoặc `legacy`)
- `CHATMINAL_WINDOW_BACKEND` (`legacy`)
- `CHATMINAL_BENCH_ENFORCE_HARD_GATE` (script `bench-phase02`, mặc định `1`)
- `CHATMINAL_BENCH_PROFILE` (script `bench-phase02`, `release` hoặc `dev`, mặc định `release`)
- `CHATMINAL_BENCH_SHELL` (script `bench-phase02`, mặc định `/bin/sh` để đo RTT ổn định)
- `CHATMINAL_BENCH_MAX_SECONDS` (giới hạn thời gian benchmark phase02, mặc định `180`)
- `CHATMINAL_BENCH_SAMPLE_INTERVAL_SECONDS` (chu kỳ lấy mẫu RSS phase02, mặc định `0.02`)
- `CHATMINAL_SOAK_MODE` (`pr` hoặc `nightly`, mặc định `pr`)
- `CHATMINAL_SOAK_DURATION_SECONDS` (nightly soak duration, mặc định `7200`)
- `CHATMINAL_SOAK_PR_ITERATIONS` (số vòng cho mode `pr`, mặc định `2`)
- `CHATMINAL_SOAK_WARMUP_ITERATIONS` (số vòng warmup bỏ khỏi gate; mặc định `1` cho `pr/nightly`)
- `CHATMINAL_SOAK_BENCH_SAMPLES` / `CHATMINAL_SOAK_BENCH_WARMUP` / `CHATMINAL_SOAK_BENCH_TIMEOUT_MS`
- `CHATMINAL_SOAK_BENCH_SHELL` (mặc định `/bin/sh`)
- `CHATMINAL_SOAK_REQUIRE_BENCH_HARD_GATE` (`0|1`, mặc định `0`)

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
- [Release Checklist](./docs/release-checklist.md)
