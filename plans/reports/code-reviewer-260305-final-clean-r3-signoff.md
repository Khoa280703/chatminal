## Code Review Summary

### Scope
- Files:
  - `apps/chatminal-app/src/ipc/client.rs`
  - `apps/chatminald/src/config.rs`
  - `apps/chatminal-app/src/config.rs`
  - `crates/chatminal-store/src/lib.rs`
  - `scripts/bench/phase02-rtt-memory-gate.sh`
- LOC scanned: 1,850
- Focus: final re-review after patch timeout budget + windows shell + suffix cap
- Scout findings: checked dependent flows (daemon request parser, session spawn path, transport behavior)

### Overall Assessment
- Previous blockers addressed: timeout budget now bounded end-to-end in request write path, Windows fallback shell is no longer `/bin/bash`, and pipe suffix cap is present in both app/daemon config.
- No Critical found.
- No High found.

### Critical Issues
- None.

### High Priority
- None.

### Medium Priority
1. Timeout during partial write can leave stream in protocol-desync state until reconnect.
- Evidence:
  - `apps/chatminal-app/src/ipc/client.rs:212`
  - `apps/chatminal-app/src/ipc/client.rs:222`
  - `apps/chatminal-app/src/ipc/client.rs:247`
- Dependent parser behavior:
  - `apps/chatminald/src/server.rs:98`
  - `apps/chatminald/src/server.rs:110`
  - `apps/chatminald/src/server.rs:139`
- Impact: if deadline expires after sending only part of a JSON line, daemon keeps partial bytes in pending buffer; next request may be parsed as invalid frame and cause cascading request errors on same connection.

2. Windows still trusts `SHELL` env before native fallback, without validating executable path format.
- Evidence:
  - `apps/chatminald/src/config.rs:167`
  - `apps/chatminald/src/config.rs:176`
- Dependent spawn path:
  - `apps/chatminald/src/session.rs:60`
- Impact: environment-dependent failure risk on Git Bash/Cygwin style `SHELL` values (example `/usr/bin/bash`) when running native daemon.

### Low Priority
- None.

### Edge Cases Found by Scout
- Write timeout after partial payload send can poison subsequent request parsing on same socket.
- Windows `SHELL` env may be POSIX-style path in some shells; native spawn may fail.
- Pipe suffix capping/sanitization path now bounded and deterministic.

### Positive Observations
- Request timeout budget now covers lock wait + write + flush (`apps/chatminal-app/src/ipc/client.rs:48`, `apps/chatminal-app/src/ipc/client.rs:162`, `apps/chatminal-app/src/ipc/client.rs:220`).
- Retry handling includes `WouldBlock`, `TimedOut`, `Interrupted` for both write and flush (`apps/chatminal-app/src/ipc/client.rs:233`, `apps/chatminal-app/src/ipc/client.rs:255`).
- Windows pipe suffix is sanitized + capped in both config sides (`apps/chatminal-app/src/config.rs:42`, `apps/chatminald/src/config.rs:76`).
- `CHATMINAL_DATA_DIR` resolution is aligned in app/daemon/store (`apps/chatminal-app/src/config.rs:104`, `apps/chatminald/src/config.rs:138`, `crates/chatminal-store/src/lib.rs:865`).
- Bench script now validates summary fields strictly before enforcing gates (`scripts/bench/phase02-rtt-memory-gate.sh:197`).
- Plan TODO check: no open checkbox found under `plans/260304-1442-chatminal-rewrite-production-completion/`.

### Recommended Actions
1. On write/flush timeout, mark IPC client connection as unhealthy and reconnect before next request.
2. On Windows, validate `SHELL` before use (or prefer `CHATMINAL_DEFAULT_SHELL`/`COMSPEC`/`cmd.exe` fallback sequence).
3. Add regression tests for timeout-mid-frame recovery and Windows shell resolution.

### Metrics
- Type Coverage: N/A (Rust)
- Test Coverage: not measured in % for this review
- Linting Issues: 0 observed in executed checks
- Validation commands executed:
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml --quiet` ✅ (42 passed)
  - `cargo test --manifest-path apps/chatminald/Cargo.toml --quiet` ✅ (36 passed)
  - `cargo test --manifest-path crates/chatminal-store/Cargo.toml --quiet` ✅ (7 passed)
  - `bash -n scripts/bench/phase02-rtt-memory-gate.sh` ✅

### Sign-off Conclusion
- Critical remaining: **No**
- High remaining: **No**
- Final clean sign-off status: **CLEAN (with medium-risk follow-ups)**

### Unresolved Questions
1. Windows production target có cần hỗ trợ tốt khi daemon chạy từ Git Bash/Cygwin env (`SHELL=/usr/bin/bash`) không?
