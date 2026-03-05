# Tester Report - Re-verify batch mới nhất sau fix RTT deterministic shell

- Work context: `/home/khoa2807/working-sources/chatminal`
- Executed at: `2026-03-05 00:20:55 +07`
- Scope: verify only, no code change

## Test Results Overview

| Command | Result | Notes |
|---|---|---|
| `./scripts/bench/phase02-rtt-memory-gate.sh` | PASS | Hard gate PASS. RTT p99=9.309ms, total_peak_mb=10.9 |
| `./scripts/fidelity/phase03-fidelity-matrix-smoke.sh` | PASS | Matrix smoke completed. Report at `/tmp/chatminal-phase03-fidelity-matrix-report-17315.json` |
| `./scripts/fidelity/phase05-fidelity-smoke.sh` | PASS | Fidelity smoke completed. Report at `/tmp/chatminal-phase05-fidelity-report-19022.json` |
| `./scripts/soak/phase05-soak-smoke.sh` | PASS | Soak smoke completed; benchmark hard gate PASS. RTT p99=8.245ms |
| `./scripts/release/phase05-release-dry-run.sh` | PASS | Release dry-run report at `/tmp/chatminal-release-dry-run-20362/release-dry-run-report.json` |
| `cargo check --workspace` | PASS | Workspace check succeeded |
| `cargo test --manifest-path apps/chatminald/Cargo.toml` | PASS | 36 passed, 0 failed |
| `cargo test --manifest-path apps/chatminal-app/Cargo.toml` | PASS | 42 passed, 0 failed |

- Total commands: 8
- PASS: 8
- FAIL: 0

## Coverage Metrics

- Not executed in this verification batch.
- Line/branch/function coverage: N/A.

## Failed Tests

- None.

## Performance Metrics

- `phase02-rtt-memory-gate.sh`: avg=5.291ms, p50=5.004ms, p95=7.881ms, p99=9.309ms, max=9.309ms, daemon_peak=6.2MB, app_peak=4.7MB, total_peak=10.9MB.
- `phase05-soak-smoke.sh` benchmark block: avg=5.104ms, p50=4.801ms, p95=6.211ms, p99=8.245ms, max=8.245ms, daemon_peak=6.3MB, app_peak=4.9MB, total_peak=11.1MB.
- Rust unit tests runtime:
  - `chatminald`: finished in 0.72s
  - `chatminal-app`: finished in 0.00s (reported by harness)

## Build Status

- Build/check status: SUCCESS
- `cargo check --workspace`: SUCCESS
- Bench/fidelity/soak/release scripts: SUCCESS
- No warnings promoted to failure in this run.

## Critical Issues

- None blocking found.

## Recommendations

1. Keep this batch as current baseline for post-fix deterministic RTT shell verification.
2. Add coverage run (`cargo llvm-cov` or project standard coverage command) if release gate requires coverage threshold.
3. Persist `/tmp/*report*.json` artifacts into CI artifact storage for traceability.

## Next Steps

1. Run same command set in CI on clean runner to confirm environment-independent reproducibility.
2. If needed for release checklist, append this report result to `docs/release-checklist.md`.

## Unresolved Questions

- None.
