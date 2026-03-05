# Deployment Guide

Last updated: 2026-03-05

## Build targets
- `apps/chatminald`
- `apps/chatminal-app`

## Prerequisites
- Rust stable
- Linux/macOS (validated local)
- Windows (validated qua CI `windows-latest`)

## Build
```bash
cargo build --release --manifest-path apps/chatminald/Cargo.toml
cargo build --release --manifest-path apps/chatminal-app/Cargo.toml
```

## Run
Terminal 1:
```bash
CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminald/Cargo.toml
```

Terminal 2:
```bash
CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminal-app/Cargo.toml -- dashboard-tui-wezterm 120 200 120 32 20
```

## Endpoint Transport (Linux/macOS)
- Transport hiện tại: Unix Domain Socket (local-only), không dùng TCP.
- Endpoint mặc định local dev: `/tmp/chatminald.sock`.
- Policy runtime:
  - Nếu endpoint path là file thường -> daemon từ chối startup.
  - Nếu endpoint là socket còn active -> daemon từ chối startup.
  - Nếu endpoint là stale socket (connect trả connection-refused) -> daemon cleanup rồi bind lại.
  - Socket permission được set `0600` (user-only); nếu set permission thất bại -> daemon fail sớm.

### Vận hành an toàn
1. Nên dùng endpoint trong thư mục user-owned.
2. Không dùng endpoint chung giữa nhiều user.
3. Nếu daemon crash để lại socket:
```bash
rm -f /tmp/chatminald.sock
```

## Endpoint Transport (Windows)
- Transport production path trên Windows dùng Named Pipe (local-only), không dùng TCP.
- Default endpoint resolver trên Windows:
  - `\\.\pipe\chatminald-<user-suffix>`
  - `<user-suffix>` ưu tiên `USERNAME`; nếu không khả dụng thì hash ổn định từ context user-local path/machine.
- Runtime policy:
  - app dùng `WaitNamedPipeW + CreateFileW` để kết nối local daemon.
  - daemon dùng `CreateNamedPipeW + ConnectNamedPipe` qua transport backend riêng.
  - không có TCP fallback trong production path.

## Environment
- `CHATMINAL_DAEMON_ENDPOINT`
- `CHATMINAL_PREVIEW_LINES`
- `CHATMINAL_MAX_LINES_PER_SESSION`
- `CHATMINAL_DEFAULT_SHELL`
- `CHATMINAL_DEFAULT_COLS`
- `CHATMINAL_DEFAULT_ROWS`
- `CHATMINAL_HEALTH_INTERVAL_MS`
- `CHATMINAL_INPUT_PIPELINE_MODE` (`wezterm` hoặc `legacy`)
- `CHATMINAL_WINDOW_BACKEND` (`wezterm-gui` hoặc `legacy`)
- `CHATMINAL_BENCH_SHELL`
- `CHATMINAL_BENCH_MAX_SECONDS`
- `CHATMINAL_BENCH_SAMPLE_INTERVAL_SECONDS`

## Input Pipeline Kill-Switch (Phase 06)
Khi cần rollback nhanh hành vi input (ví dụ regression IME/modifier):

1. Bật mode fallback:
```bash
export CHATMINAL_INPUT_PIPELINE_MODE=legacy
```
2. Restart `chatminal-app` (daemon không cần thay đổi config).
3. Xác nhận thao tác cơ bản:
   - `Ctrl+C` dừng process foreground.
   - `Ctrl+Z` stop process foreground.
   - Attach mode không crash.
   - Có thể chạy nhanh:
```bash
make phase06-killswitch-verify
```
4. Khi cần quay lại pipeline mới:
```bash
export CHATMINAL_INPUT_PIPELINE_MODE=wezterm
```

## Window Backend Kill-Switch (Phase 08)
Khi cần rollback nhanh đường chạy window:

1. Bật backend fallback:
```bash
export CHATMINAL_WINDOW_BACKEND=legacy
```
2. Mở lại window:
```bash
make window
```
3. Verify nhanh rollback contract:
```bash
make phase08-killswitch-verify
```
4. Khi cần quay lại runtime mặc định:
```bash
export CHATMINAL_WINDOW_BACKEND=wezterm-gui
```
