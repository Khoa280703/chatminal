# Independent Verification Report - Latest Batch

Date: 2026-03-05
Work context: /home/khoa2807/working-sources/chatminal

## Test Results Overview
- Command 1: `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
  - Run #1: FAILED (61 total, 60 passed, 1 failed, 0 skipped)
  - Run #2 (recheck): PASSED (61 total, 61 passed, 0 failed)
  - Flaky check: `concurrent_requests_receive_correct_response_variant` passed 30/30 isolated reruns.
- Command 2: `bash scripts/migration/phase06-killswitch-verify.sh` PASSED
- Command 3: `bash scripts/bench/phase02-rtt-memory-gate.sh` PASSED

## Coverage Metrics
- Not generated in this verification run (N/A line/branch/function coverage).

## Failed Tests
- Intermittent failure observed on Run #1:
  - Test: `ipc::client::client_tests::concurrent_requests_receive_correct_response_variant`
  - Panic site: `apps/chatminal-app/src/ipc/client_tests.rs:74`
  - Error: `session request: "daemon stream disconnected"`

## Root Cause (Most Likely)
- Race window in concurrent request handling when stream closes near-simultaneous with response routing:
  - `apps/chatminal-app/src/ipc/client_tests.rs:46-64`: mock server writes 2 responses then drops stream immediately.
  - `apps/chatminal-app/src/ipc/client.rs:76-83`: on `Disconnected`, request returns error after only one backlog recheck.
  - `apps/chatminal-app/src/ipc/client.rs:96-100`: mismatched response is pushed to shared backlog, but another thread can hit disconnect path before that push becomes visible.

## File/Line Fix Hints
1. `apps/chatminal-app/src/ipc/client_tests.rs:46-64`
- Keep mock server stream alive until both client requests definitely consume responses (barrier/ack/channel), reducing false disconnect race in test.

2. `apps/chatminal-app/src/ipc/client.rs:76-83,139-150`
- On `RecvTimeoutError::Disconnected`, attempt a short bounded drain/backlog reconciliation loop before failing hard.

3. `apps/chatminal-app/src/ipc/client.rs:96-100`
- Consider serial response dispatcher model (single consumer + per-request wait map), avoid cross-thread backlog race.

## Performance Metrics
- `cargo test` runtime: 0.26s (run #1), 0.27s (run #2)
- `phase06-killswitch-verify.sh`: 8.71s
- `phase02-rtt-memory-gate.sh`: 1.19s
  - RTT: samples=80 warmup=15 avg=5.879ms p50=5.693ms p95=9.194ms p99=9.433ms max=9.433ms
  - Memory peak: daemon=8.1MB app=6.8MB total=14.9MB

## Build Status
- Build/test compile phases succeeded for both `dev` and `release` profiles in executed scripts.
- No blocking build warning surfaced in command outputs.

## Critical Issues
- Non-deterministic IPC test behavior observed once in required full-suite run.
- Not reproduced in isolated reruns, but still regression-risk indicator (flaky race potential).

## Recommendations
1. Treat this as flaky-risk, not hard functional regression yet.
2. Stabilize concurrent IPC test with deterministic synchronization in mock server.
3. Harden client disconnect handling path to reduce false negatives under close timing.
4. Add CI stress target for this test (e.g., looped full-suite subset) to catch recurrence.

## Next Steps
1. Patch test synchronization at `client_tests.rs` first (fast, low risk).
2. Add bounded disconnect-drain safeguard in `client.rs`.
3. Re-run full batch verification after patch.

## Unresolved Questions
- Should intermittent failure in this gate be blocking for merge, or classified as non-blocking flaky until reproduced in CI?
