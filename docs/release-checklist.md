# Release Checklist

Last updated: 2026-03-05

## Scope
- Runtime: `apps/chatminald`, `apps/chatminal-app`, `crates/chatminal-protocol`, `crates/chatminal-store`
- Release order:
  1. Linux + macOS
  2. Windows (sau khi Named Pipe + matrix pass)

## Auto Gates (must pass)
1. Compile + tests
```bash
cargo check --workspace
cargo test --manifest-path apps/chatminald/Cargo.toml
cargo test --manifest-path apps/chatminal-app/Cargo.toml
```
2. RTT + memory hard gate
```bash
bash scripts/bench/phase02-rtt-memory-gate.sh
```
3. Docs validation
```bash
node "$HOME/.claude/scripts/validate-docs.cjs" docs/
```

## Manual Gates (must pass)
0. Kill-switch sanity (runtime rollback path):
```bash
make phase06-killswitch-verify
```
   - attach path phải khởi động được ở cả `wezterm` và `legacy`, không crash ngay.
   - sau kiểm tra, trả về mode mặc định:
```bash
export CHATMINAL_INPUT_PIPELINE_MODE=wezterm
```

1. Fidelity matrix: [terminal-fidelity-matrix.md](./terminal-fidelity-matrix.md)
2. Fidelity matrix auto smoke (phase 03, strict mặc định):
```bash
CHATMINAL_FIDELITY_STRICT=1 bash scripts/fidelity/phase03-fidelity-matrix-smoke.sh
```
   - Dùng `CHATMINAL_FIDELITY_STRICT=0` cho mode relaxed khi chỉ cần signal warning.
   - Script tự detect `timeout`/`gtimeout` để chạy ổn trên Linux/macOS.
   - Strict mode yêu cầu các case trong `CHATMINAL_FIDELITY_REQUIRED_CASES` không được skip.
   - Required PR subset hiện tại: `bash-prompt,ctrl-c,ctrl-c-burst,ctrl-z,unicode,stress-paste,resize,reconnect`.
   - CI Linux+macOS override `CHATMINAL_FIDELITY_REQUIRED_CASES` lên full required matrix (bao gồm vim/nvim/tmux/htop/btop/fzf).
   - IME (`ime-vi`, `ime-ja`, `ime-zh`) vẫn là manual evidence gate trước release, chưa khóa bằng auto smoke.
3. Phase-06 modifier/input smoke + IME manual gate report:
```bash
bash scripts/fidelity/phase06-input-modifier-ime-smoke.sh
```
   - Script sẽ fail nếu phase03 strict matrix fail.
   - Report JSON đánh dấu rõ `ime_manual_evidence_required=true`; có thể bật hard-fail manual gate bằng `CHATMINAL_PHASE06_REQUIRE_MANUAL_IME_SIGNOFF=1`.
4. Window smoke (headless):
```bash
bash scripts/smoke/window-wezterm-smoke.sh
```
5. Fidelity smoke report (machine-readable):
```bash
bash scripts/fidelity/phase05-fidelity-smoke.sh
```
6. Soak smoke report (machine-readable):
```bash
bash scripts/soak/phase05-soak-smoke.sh
```
   - Mode mặc định `pr` chạy 2 vòng ngắn (1 warmup + 1 evaluated).
   - Promote gate cần chạy nightly 2h:
```bash
CHATMINAL_SOAK_MODE=nightly CHATMINAL_SOAK_DURATION_SECONDS=7200 CHATMINAL_SOAK_REQUIRE_BENCH_HARD_GATE=0 bash scripts/soak/phase05-soak-smoke.sh
```
   - Lý do `CHATMINAL_SOAK_REQUIRE_BENCH_HARD_GATE=0`: soak gate tập trung crash/freeze/input-loss; RTT hard-gate đã được chặn riêng ở bước benchmark phase-02.
7. Release dry-run artifacts:
```bash
bash scripts/release/phase05-release-dry-run.sh
```
   - Script tự chọn checksum command theo OS (`sha256sum` hoặc `shasum -a 256`).
   - Script luôn phát hành JSON report cả khi fail sớm; nếu đường dẫn report không ghi được sẽ fallback sang `/tmp/chatminal-release-dry-run-report-<RUN_ID>.json`.

## KPI Thresholds
1. Latency
- Target: `p95 <= 30ms`, `p99 <= 60ms`
- Hard fail: `p95 > 50ms`
2. Memory (RSS)
- `chatminald` target `<= 120MB`, hard fail `> 160MB`
- `chatminal-app` target `<= 180MB`, hard fail `> 220MB`
- Total target `<= 300MB`, hard fail `> 350MB`

## Cutover Criteria
1. Auto gates pass 2 vòng liên tiếp.
2. Manual gates pass đầy đủ, không còn bug P0/P1 open.
3. Deployment guide chạy lại thành công trên Linux/macOS.
4. Changelog + roadmap đã sync với trạng thái code thực tế.

## Rollback Plan
1. Không overwrite bản stable trước khi smoke pass.
2. Giữ artifact/tag release trước đó để rollback ngay.
3. Nếu phát hiện regression P0/P1 trong 24h đầu:
   - Revert tag phát hành mới.
   - Phát hành lại artifact stable gần nhất.
   - Freeze feature, chỉ cho phép bugfix.

## Artifacts and Reports
- Fidelity matrix smoke report mặc định: `/tmp/chatminal-phase03-fidelity-matrix-report-$$.json`
- Fidelity report JSON mặc định: `/tmp/chatminal-phase05-fidelity-report-$$.json`
- Soak report JSON mặc định: `/tmp/chatminal-phase05-soak-report-$$.json`
- Release dry-run report mặc định: `/tmp/chatminal-release-dry-run-$$/release-dry-run-report.json`
- Release dry-run report fallback khi report path không ghi được: `/tmp/chatminal-release-dry-run-report-<RUN_ID>.json`
