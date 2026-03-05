# tester-260305-hard-cutover-wezterm-gui

- Date: 2026-03-05
- Work context: `/home/khoa2807/working-sources/chatminal`
- Scope verified:
  - `apps/chatminal-app/src/main.rs`
  - `apps/chatminal-app/src/config.rs`
  - `apps/chatminal-app/src/input/mod.rs`
  - `apps/chatminal-app/src/input/terminal_input_event.rs`
  - `apps/chatminal-app/src/input/terminal_input_shortcut_filter.rs`
  - `apps/chatminal-app/src/input/ime_commit_deduper.rs`
  - `Makefile`
  - `README.md`
  - docs/plans liên quan cutover

## Sequential Steps
1. Read `README.md` + `Makefile` để xác nhận command/gate hiện hành cho WezTerm GUI cutover.
2. Run compile gate: `cargo check --workspace`.
3. Run unit/integration gate: `make test` + focused `cargo test --manifest-path apps/chatminal-app/Cargo.toml`.
4. Run runtime smoke gates: `make smoke-window`, `make phase06-killswitch-verify`, `make fidelity-input-ime-smoke`.
5. Run extra regression smoke cho command surface: so sánh launcher args giữa `window-wezterm-gui` và `window-wezterm-gui-proxy` bằng mock `CHATMINAL_WEZTERM_BIN`.

## Test Results Overview
- Total Rust tests run: `119`
- Passed: `119`
- Failed: `0`
- Ignored/Skipped (Rust tests): `0`
- Smoke scripts: `3/3 PASS`
- Fidelity matrix checks (phase03, strict): `15 checks` (`9 pass`, `6 skip optional tools`, `0 fail`, `required_skip_count=0`)

### Command Results
| Command | Result | Duration | Notes |
|---|---|---:|---|
| `cargo check --workspace` | PASS | `0:00.31` | No compile errors |
| `make test` | PASS | `0:01.28` | Protocol/store/daemon/app suites pass |
| `cargo test --manifest-path apps/chatminal-app/Cargo.toml` | PASS | `0:00.29` | 65 tests pass, gồm `config` + `input/*` scope |
| `make smoke-window` | PASS | `0:01.31` | `window-wezterm-gui smoke passed` |
| `make phase06-killswitch-verify` | PASS | `0:14.78` | `wezterm_exit=124 legacy_exit=124` + ready banner |
| `make fidelity-input-ime-smoke` | PASS | `0:11.66` | strict matrix pass + phase06 report pass |

Artifacts from run:
- `/tmp/chatminal-phase03-fidelity-matrix-report-4045273.json`
- `/tmp/chatminal-phase06-input-modifier-ime-report-4045273.json`

## Coverage Metrics
- Coverage tool status: `N/A`
- `cargo llvm-cov --version` failed: `no such command: llvm-cov`.
- Line/branch/function coverage %: chưa generate trong môi trường hiện tại.

## Failed Tests
- None.

## Performance Metrics
- Tổng thời gian cho batch check/test/smoke chính: ~`00:29.63` (sum theo wall-clock command).
- Slowest check: `make phase06-killswitch-verify` (`14.78s`).
- No hang/timeout regression observed in executed smoke gates.

## Build Status
- Build/check status: PASS.
- Warnings blocking build: none observed in executed commands.

## Critical Issues
1. `Medium` - rollback command surface chưa tách runtime thực tế.
   - Evidence code: `apps/chatminal-app/src/main.rs:86-90` đều gọi chung `run_window_wezterm_gui`.
   - Evidence code: `apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs:94-100` luôn emit `proxy-wezterm-session`.
   - Evidence runtime smoke: compare test cho thấy launcher args giống hệt:
     - `window-wezterm-gui args: start -- ... proxy-wezterm-session <session_id>`
     - `window-wezterm-gui-proxy args: start -- ... proxy-wezterm-session <session_id>`
     - `COMMAND_ARGS_EQUAL=true`
   - Impact: `window-wezterm-gui-proxy` hiện là alias, rollback path không độc lập về behavior.

2. `Low` - docs/plan inconsistency sau hard-cutover.
   - Evidence: `plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md:41` và `:68` còn ghi fallback `make window-legacy`.
   - Hiện trạng command thực tế: `Makefile` dùng `window-proxy`, không còn target `window-legacy`.

## Recommendations
1. Nếu mục tiêu batch là embedded default runtime: tách handler `window-wezterm-gui` khỏi proxy path ngay trong code (không dùng chung launcher args).
2. Nếu hiện tại chủ đích vẫn proxy-first: đổi tên/ghi rõ `window-wezterm-gui-proxy` là alias tạm, tránh hiểu nhầm rollback độc lập.
3. Sync lại `plan.md` cutover để bỏ hoàn toàn reference `window-legacy`.
4. Thêm smoke CI riêng xác minh `window-wezterm-gui` và `window-wezterm-gui-proxy` phải khác behavior (khi embedded cutover hoàn tất).

## Next Steps
1. Quyết định contract chính thức cho `window-wezterm-gui` trong batch này: embedded hay proxy-first.
2. Chốt command semantics rồi cập nhật test oracle + smoke assertions theo semantics đó.
3. Bật coverage tool (`cargo-llvm-cov`) nếu muốn gate coverage định lượng cho `input/*`.

## Unresolved Questions
1. `window-wezterm-gui` ở batch hard-cutover này expected phải chạy embedded runtime chưa, hay vẫn cho phép proxy-first?
2. Nếu vẫn proxy-first tạm thời, có muốn giữ `window-wezterm-gui-proxy` như alias hay remove để tránh duplicate command surface?
