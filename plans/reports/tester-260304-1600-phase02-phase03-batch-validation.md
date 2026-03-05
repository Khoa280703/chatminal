# Tester Report - Phase02/Phase03 batch validation

Work context: `/home/khoa2807/working-sources/chatminal`
Date: 2026-03-04

## Scope
- `apps/chatminal-app/src/input/*`
- `apps/chatminal-app/src/terminal_wezterm_attach_tui.rs`
- `apps/chatminal-app/src/terminal_wezterm_attach_frame_renderer.rs`
- `apps/chatminal-app/src/terminal_quality_benchmark/*`
- `apps/chatminal-app/src/main.rs`
- `scripts/bench/phase02-rtt-memory-gate.sh`
- `apps/chatminald/src/config.rs`
- `crates/chatminal-store/src/lib.rs`

## Commands run
1. `cargo check --workspace`
2. `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
3. `cargo test --manifest-path apps/chatminald/Cargo.toml`
4. `cargo test --manifest-path crates/chatminal-store/Cargo.toml`
5. `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml`
6. `CHATMINAL_BENCH_SAMPLES=10 CHATMINAL_BENCH_WARMUP=2 bash scripts/bench/phase02-rtt-memory-gate.sh`
7. `node /home/khoa2807/.claude/scripts/validate-docs.cjs docs/`

## Results
- `cargo check --workspace`: PASS
- `apps/chatminal-app` tests: PASS (29/29)
- `apps/chatminald` tests: PASS (27/27)
- `crates/chatminal-store` tests: PASS (7/7)
- `crates/chatminal-protocol` tests: PASS (7/7)
- Benchmark smoke: PASS hard gates
  - sample output: `p95_ms=42.870`, `p99_ms=42.870`
  - daemon peak RSS: `9.3MB`
  - app peak RSS: `13.2MB`
  - total peak RSS: `22.6MB`
  - `pass_fail_gate=true`
- Docs validation: PASS (internal links + config keys OK)

## Findings
- No Critical/High/Medium functional regression found in tested scope.
- Note: benchmark run samples nhỏ vẫn có `pass_targets=false` (p95 target 30ms) do môi trường local dev, nhưng hard gate fail threshold (`p95 <= 45ms`) vẫn pass.

## Recommendations
1. Chạy benchmark script trong CI Linux/macOS với profile runner ổn định trước khi nâng gate từ warning sang blocking cho target 30ms.
2. Thêm reconnect state-machine tests multi-session/generation để đóng hẳn TODO phase 03.

## Unresolved questions
- None.

