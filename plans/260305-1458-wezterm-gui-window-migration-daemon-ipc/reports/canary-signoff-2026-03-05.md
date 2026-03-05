# Canary Sign-off (Linux) - 2026-03-05

## Scope
- Plan: `260305-1458-wezterm-gui-window-migration-daemon-ipc`
- Wave: Linux canary for `window-wezterm-gui` default path + rollback drills
- Owner: Codex session

## Evidence Executed
- `cargo check --workspace` => pass
- `make test` => pass
- `make smoke-window` => pass
- `make bench-phase02` => pass (`p95=8.688ms`, `p99=13.225ms`, `total_peak_mb=15.0`, `pass_fail_gate=true`, fail-gate `p95<=50ms`)
- `make fidelity-matrix-smoke` (strict) => pass
- `make fidelity-input-ime-smoke` (strict wrapper) => pass
- `make soak-smoke` => pass
- `make release-dry-run` => pass
- `make phase06-killswitch-verify` => pass
- `make phase08-killswitch-verify` with `CHATMINAL_PHASE08_REQUIRE_LEGACY_HEADLESS=1` => pass

## Artifacts
- `reports/fidelity-linux-phase03-fullscreen.json`
- `reports/fidelity-linux-phase06-input-ime.json`
- `reports/fidelity-linux-phase05-soak.json`
- `reports/fidelity-linux-phase05-release-dry-run.json`
- `reports/rollout-checklist.md`

## Decision
- Linux canary: **signed off** for this migration wave.
- Promotion guard: keep macOS manual smoke + IME manual sign-off as blocking item before cross-platform GA promotion.

## Unresolved Questions
1. macOS manual smoke + IME sign-off owner/date cần chốt ai chịu trách nhiệm?
