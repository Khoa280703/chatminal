# Chatminal

Chatminal hiện chạy theo mô hình `single-runtime desktop`: cửa sổ mặc định là `Chatminal Desktop` first-party với runtime session/profile/history nhúng trực tiếp trong process GUI, có sidebar profile/session bên trái và không hiện tab bar phía trên. Desktop path dùng native API/subscription của `chatminal-runtime`; `chatminal-protocol` chỉ còn phục vụ daemon/CLI compatibility boundary.

## Runtime hiện tại
- Window client mặc định: `apps/chatminal-desktop` (`apps/chatminal-app` chỉ còn là launcher/CLI compatibility)
- GUI source entry hiện tại: `apps/chatminal-desktop/src`
- Headless mux host cho engine compatibility: binary `chatminal-mux` trong package `apps/chatminal-desktop`
- Terminal engine reference pool còn lại: `third_party/terminal-engine-reference` (reference-only; active build/runtime/workspace của Chatminal không còn phụ thuộc trực tiếp vào subtree này)
- Native vendored deps hiện tại: `vendor/terminal-deps`
- `third_party/terminal-engine-reference` hiện không còn là workspace standalone được hỗ trợ trong repo này; nó chỉ còn vai trò reference source/history
- Chatminal Desktop explicit command: `window-desktop`
- Runtime lõi: `crates/chatminal-runtime`
- Daemon compatibility host: `apps/chatminald` (bọc `chatminal-runtime` cho CLI/TUI cũ)
- Terminal core: `crates/chatminal-terminal-core`
- Shared contracts: `crates/chatminal-protocol`
- Shared persistence: `crates/chatminal-store` (SQLite)

## Cấu trúc repo
- `apps/chatminal-app/`: launcher CLI/TUI compatibility
- `apps/chatminal-desktop/`: desktop app first-party, runtime chính cho window
- `apps/chatminald/`: compatibility host local IPC
- `crates/chatminal-runtime/`: session/profile/history/explorer/runtime state dùng chung desktop + daemon
- `crates/chatminal-terminal-core/`: terminal parser/state nội bộ
- `crates/chatminal-protocol/`: request/response/event types
- `crates/chatminal-store/`: SQLite store (profiles/sessions/scrollback)
- `docs/`: tài liệu kiến trúc, roadmap, changelog

## Yêu cầu
- Rust stable (khuyến nghị >= 1.93)
- Linux/macOS
- Không yêu cầu cài terminal host ngoài máy nếu repo đã có source/vendor hiện tại
- Lần build GUI đầu tiên sẽ hydrate các C deps vendored còn thiếu vào `vendor/terminal-deps/` qua `scripts/bootstrap-terminal-vendor-deps.sh`
- Linux cần native GUI deps của desktop runtime/Wayland/X11 như thường lệ, nhưng trên host dev hiện tại `cargo check -p chatminal-desktop` đã pass sau khi dọn graph first-party và asset path

## Chạy local
CLI/TUI compatibility:
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

Để mở cửa sổ terminal mặc định:
```bash
make window
```

`make window` sẽ mở Chatminal Desktop first-party với sidebar profile/session bên trái. Luồng này không cần `chatminald`.

Hoặc gọi thẳng lệnh explicit:
```bash
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- window-desktop
```

Chạy GUI package trực tiếp:
```bash
cargo run --manifest-path apps/chatminal-desktop/Cargo.toml -- start -- chatminal-runtime proxy-desktop-session
```

Nếu muốn hydrate vendored C deps trước:
```bash
make bootstrap-terminal-deps
make verify-third-party-reference-only
```

Smoke cho Chatminal Desktop launcher:
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
CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminal-app/Cargo.toml -- dashboard-tui 120 200 120 32 20
```

Các lệnh client khác:
```bash
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- workspace
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- sessions
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- create "Dev"
cargo run --manifest-path apps/chatminal-app/Cargo.toml -- activate <session_id> 120 32 200
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
- `CHATMINAL_INPUT_PIPELINE_MODE` (`desktop` hoặc `legacy`)
- `CHATMINAL_DESKTOP_BIN` (override đường dẫn tới binary `chatminal-desktop` tương thích; nếu không set launcher sẽ build/chạy package trong workspace)
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
make verify-third-party-reference-only
cargo check -p chatminal-desktop
cargo test -p chatminal-runtime
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
