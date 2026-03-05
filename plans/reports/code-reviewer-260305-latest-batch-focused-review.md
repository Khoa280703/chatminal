## Code Review Summary

### Scope
- Files:
  - `apps/chatminal-app/src/ipc/client.rs`
  - `apps/chatminald/src/config.rs`
  - `scripts/bench/phase02-rtt-memory-gate.sh`
  - `.env.example`
- Focus: latest working-tree batch (no code edits), edge-case scout + manual review.
- Validation run:
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml --quiet` (pass)
  - `cargo test --manifest-path apps/chatminald/Cargo.toml --quiet` (pass)
  - `bash -n scripts/bench/phase02-rtt-memory-gate.sh` (pass)

### Overall Assessment
- Critical: none.
- High: found.
- Main risk cluster: timeout semantics in IPC client, Windows endpoint safety defaults, and fail-open behavior in bench gate parsing.

### Critical Issues
- None.

### High Priority
1. Request timeout is not enforced during blocking write on non-Windows path; caller can hang beyond requested timeout.
- Impact: request timeout contract can be violated; CLI/TUI call can stall under backpressure or unresponsive daemon.
- References:
  - `apps/chatminal-app/src/ipc/client.rs:79`
  - `apps/chatminal-app/src/ipc/client.rs:91`
  - `apps/chatminal-app/src/ipc/client.rs:233`
  - `apps/chatminal-app/src/ipc/client.rs:239`

2. Windows default pipe endpoint can collapse to global fallback (`\\.\pipe\chatminald`) when `USERNAME` sanitizes to empty; increases collision/isolation risk.
- Impact: possible cross-user endpoint collision on Windows hosts with non-ASCII/empty usernames.
- References:
  - `apps/chatminald/src/config.rs:71`
  - `apps/chatminald/src/config.rs:86`
  - `apps/chatminald/src/config.rs:82`

3. Phase02 hard gate can fail-open when RTT field parse is empty/malformed.
- Impact: false PASS possible if `p95_ms` missing/non-numeric (evaluates like `0` in awk compare path).
- References:
  - `scripts/bench/phase02-rtt-memory-gate.sh:137`
  - `scripts/bench/phase02-rtt-memory-gate.sh:150`

### Medium Priority
1. Deterministic shell fix (`CHATMINAL_BENCH_SHELL` + `CHATMINAL_DEFAULT_SHELL` override in bench script) has side effects.
- Confirmed side effects:
  - Gate now measures `/bin/sh` behavior, not user default shell behavior.
  - `/bin/sh` implementation differs by distro (dash/bash/etc), so cross-runner comparability still imperfect.
  - If shell path invalid/missing, benchmark fails at runtime.
- References:
  - `scripts/bench/phase02-rtt-memory-gate.sh:19`
  - `scripts/bench/phase02-rtt-memory-gate.sh:80`
  - `.env.example:14`

2. `.env.example` currently embeds Unix-specific defaults that can override new platform-safe resolution and break Windows runs if sourced directly.
- Impact: forcing `CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock` + `/bin/bash` can break native Windows setup.
- References:
  - `.env.example:2`
  - `.env.example:8`
  - `apps/chatminald/src/config.rs:56`
  - `apps/chatminald/src/config.rs:113`

### Low Priority
- None đáng kể trong scope review.

### Edge Cases Found by Scout
- IPC backpressure + blocking write path can violate timeout expectation.
- Windows identity sanitization edge case (`USERNAME` non-ASCII) collapses endpoint namespace.
- Bench output schema drift can bypass gate due parse-fallback behavior.

### Positive Observations
- Dedicated Windows write queue in client reduces concurrent write contention risk.
- Config now supports `CHATMINAL_DATA_DIR` override cleanly.
- Script cleanup and hard/soft gate flow is clear and diagnosable via logs.

### Recommended Actions
1. Enforce write-time bounded behavior in client write path (or explicit cancellation strategy) so `request(timeout)` cannot block indefinitely.
2. Harden Windows endpoint suffix generation (SID/hash fallback) instead of global fallback when username sanitization is empty.
3. Make phase02 gate fail-closed when RTT fields are missing/invalid.
4. Document deterministic shell tradeoff in README + keep explicit per-environment shell pin in CI for reproducibility.
5. Split `.env.example` by platform guidance or avoid forcing Unix-only endpoint/shell defaults.

### Metrics
- Type/build check: pass on tested Linux environment.
- Tests run in scope: 78 tests passed (42 app + 36 daemon).
- Linting: not run (no linter output collected in this review).

### Unresolved Questions
1. Windows target expectation: native shell path required, or WSL/Git-Bash acceptable default?
2. Gate policy: should missing RTT summary keys be hard failure by default?
