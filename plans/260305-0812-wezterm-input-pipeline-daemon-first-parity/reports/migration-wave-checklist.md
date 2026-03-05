# Migration Wave Checklist (Phase 06)

Date: 2026-03-05  
Owner: chatminal core

## Wave Plan
1. Wave A: `wezterm` default + `legacy` kill-switch available.
2. Wave B: internal canary Linux/macOS.
3. Wave C: default-on rộng hơn sau 2 vòng gate pass liên tiếp.

## Pre-Canary
- [x] `cargo check --workspace` pass.
- [x] `cargo test --manifest-path apps/chatminal-app/Cargo.toml` pass.
- [x] `cargo test --manifest-path apps/chatminald/Cargo.toml` pass.
- [x] Phase03 fidelity required subset pass.
- [x] Phase05 soak `pr` pass.
- [x] Phase06 input/modifier wrapper report pass.

## Canary Signals
- [x] `CHATMINAL_INPUT_PIPELINE_MODE=wezterm` path verified in tests/runtime.
- [x] `CHATMINAL_INPUT_PIPELINE_MODE=legacy` path verified in tests/runtime.
- [x] Rollback procedure documented in deployment guide.

## Promote Gate
- [x] 2h soak nightly pass (Linux + macOS gate artifacts).
- [x] IME manual evidence matrix signed (`ime-vi/ja/zh`).
- [x] Release dry-run artifacts pass ngay trước cutover.

## Evidence
- `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/phase03-fidelity-matrix-20260305-v2.json`
- `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/soak-pr-20260305-v4.json`
- `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/phase06-input-modifier-ime-20260305-v2.json`
- `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/release-dry-run-20260305-v2.json`
- `scripts/soak/phase05-soak-smoke.sh` + `.github/workflows/rewrite-quality-gates.yml` nightly Linux/macOS jobs (`CHATMINAL_SOAK_DURATION_SECONDS=7200`)
- `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/ime-manual-matrix.md`
