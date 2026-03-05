## Code Review Summary

### Scope
- Files:
  - `apps/chatminal-app/src/ipc/client.rs`
  - `apps/chatminal-app/src/config.rs`
  - `apps/chatminald/src/config.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `scripts/bench/phase02-rtt-memory-gate.sh`
  - `docs/project-changelog.md`
- LOC scanned: 1,947
- Focus: final clean re-review after latest fixes (read-only)
- Scout findings: revalidated with edge-case pass before manual review

### Overall Assessment
- Good progress; previous data-dir mismatch fix is in place (`app/daemon/store` now aligned).
- No Critical found.
- High findings still remain in request timeout semantics and Windows default shell fallback.

### Critical Issues
- None.

### High Priority
1. Request timeout budget is not enforced end-to-end in IPC write path.
- Evidence:
  - `apps/chatminal-app/src/ipc/client.rs:48`
  - `apps/chatminal-app/src/ipc/client.rs:157`
  - `apps/chatminal-app/src/ipc/client.rs:198`
  - `apps/chatminal-app/src/ipc/client.rs:203`
  - `apps/chatminal-app/src/ipc/client.rs:212`
  - `apps/chatminal-app/src/ipc/client.rs:227`
- Why high: deadline is computed before mutex lock, but lock wait is unbounded; write timeout is set once from initial remaining and not refreshed per loop, so request can exceed caller timeout under contention/backpressure.

2. Windows fallback shell is Unix path; default session spawn can fail on Windows hosts.
- Evidence:
  - `apps/chatminald/src/config.rs:159`
  - `apps/chatminald/src/config.rs:162`
- Affected flow (dependent):
  - `apps/chatminald/src/state/request_handler.rs:76`
  - `apps/chatminald/src/session.rs:60`
- Why high: if `CHATMINAL_DEFAULT_SHELL` and `SHELL` are absent (common on PowerShell/CMD), daemon falls back to `/bin/bash`, then PTY spawn fails for new/activated sessions.

### Medium Priority
1. IPC write loop does not treat `Interrupted` as retryable I/O.
- Evidence:
  - `apps/chatminal-app/src/ipc/client.rs:217`
  - `apps/chatminal-app/src/ipc/client.rs:233`
- Impact: transient signal interruption can fail request early.

2. Windows pipe suffix has no explicit length cap.
- Evidence:
  - `apps/chatminal-app/src/config.rs:57`
  - `apps/chatminal-app/src/config.rs:77`
  - `apps/chatminald/src/config.rs:91`
  - `apps/chatminald/src/config.rs:111`
- Impact: very long identity strings can exceed pipe-name practical limits on edge hosts.

### Low Priority
1. Benchmark timeout path can race when app is wrapped by `timeout` binary and script also kills wrapper PID.
- Evidence:
  - `scripts/bench/phase02-rtt-memory-gate.sh:134`
  - `scripts/bench/phase02-rtt-memory-gate.sh:141`
  - `scripts/bench/phase02-rtt-memory-gate.sh:151`
  - `scripts/bench/phase02-rtt-memory-gate.sh:153`
- Impact: low-probability cleanup/orphan sampling inconsistency.

### Edge Cases Found by Scout
- Deadline overshoot risk in `write_payload_with_deadline` under partial writes.
- Writer mutex wait not bounded by request deadline.
- Windows runtime depends on env override for shell portability.
- Process-tree RSS sampling around timeout wrapper has race edge.

### Positive Observations
- `CHATMINAL_DATA_DIR` relative-path normalization is now consistent in `app/daemon/store` (`apps/chatminal-app/src/config.rs:89`, `apps/chatminald/src/config.rs:123`, `crates/chatminal-store/src/lib.rs:865`).
- Benchmark gate hardens summary parsing and checks required fields before enforcing thresholds (`scripts/bench/phase02-rtt-memory-gate.sh:197`).
- Changelog reflects latest hardening batch in focused areas (`docs/project-changelog.md:116`).
- Plan TODO check: no open checkbox found under `plans/260304-1442-chatminal-rewrite-production-completion/`.

### Recommended Actions
1. Make request deadline truly end-to-end: include lock-acquire budget + refresh `set_write_timeout` per iteration based on remaining time.
2. Add Windows-specific default shell fallback (`pwsh.exe`/`powershell.exe`/`cmd.exe` strategy) when env vars are missing.
3. Treat `ErrorKind::Interrupted` as retryable in write/flush loops.
4. Add max length cap (or stable hashed cap) for Windows pipe suffix.

### Metrics
- Type Coverage: N/A (Rust)
- Test Coverage: not measured in % for this review
- Linting Issues: 0 observed in executed checks
- Validation commands executed:
  - `cargo check --workspace` ✅
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml config::tests` ✅
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml client_tests` ✅
  - `cargo test --manifest-path apps/chatminald/Cargo.toml config::tests` ✅
  - `cargo test --manifest-path crates/chatminal-store/Cargo.toml` ✅
  - `bash -n scripts/bench/phase02-rtt-memory-gate.sh` ✅

### Sign-off Conclusion
- Critical remaining: **No**
- High remaining: **Yes (2)**
- Final clean sign-off status: **NOT CLEAN**

### Unresolved Questions
1. Is Windows runtime expected to be production-ready now, or still best-effort pre-GA?
