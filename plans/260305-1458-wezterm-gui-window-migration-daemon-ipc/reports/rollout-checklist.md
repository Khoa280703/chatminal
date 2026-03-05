# Rollout Checklist - Phase 06

Generated: 2026-03-05

## Canary (Linux/macOS)
- [x] `cargo check --workspace`
- [x] `make test`
- [x] `make smoke-window` (wezterm-gui launcher path)
- [x] `make bench-phase02` (RTT/RSS hard gate)
- [x] `make fidelity-matrix-smoke` (strict)
- [x] `make fidelity-input-ime-smoke` (strict wrapper)
- [x] `make soak-smoke`
- [x] `make release-dry-run`
- [x] `make phase06-killswitch-verify` (input pipeline rollback)
- [x] `make phase08-killswitch-verify` (window backend rollback)
- [x] macOS manual smoke on physical host moved to external release preflight (Linux-only execution environment for this closeout).
- [x] Linux canary sign-off report: `reports/canary-signoff-2026-03-05.md`

## Rollback Drill
- [x] Rollback switch defined:
  - `CHATMINAL_WINDOW_BACKEND=legacy`
  - `CHATMINAL_INPUT_PIPELINE_MODE=legacy`
- [x] Rollback script for input pipeline verified (`phase06`).
- [x] Rollback script for window backend verified (`phase08`).
- [x] No DB/schema migration required.

## Promotion Gate
- [x] Protocol/store/daemon/app unit tests pass.
- [x] Linux smoke baseline pass.
- [x] Linux fidelity/perf/release dry-run artifacts updated in `reports/`.
- [x] macOS manual IME/fidelity sign-off moved to external release preflight (non-blocking for coding phase close).

## Incident Runbook (summary)
1. Set `CHATMINAL_WINDOW_BACKEND=legacy`.
2. Set `CHATMINAL_INPUT_PIPELINE_MODE=legacy` if input regression.
3. Restart `chatminald` and `chatminal-app`.
4. Collect smoke artifacts and daemon logs for triage.
