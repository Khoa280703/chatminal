## Code Review Summary

### Scope
- Files:
  - `apps/chatminal-app/src/ipc/client.rs`
  - `apps/chatminal-app/src/ipc/transport/windows.rs`
  - `apps/chatminal-app/src/config.rs`
  - `apps/chatminald/src/config.rs`
  - `scripts/bench/phase02-rtt-memory-gate.sh`
  - `docs/project-changelog.md`
- LOC: 1,146
- Focus: latest changes in listed files + direct dependents for edge-case validation
- Scout findings: validated data-dir path split risk, benchmark timeout boundary, Windows pipe suffix collision edge case

### Overall Assessment
Batch này có nhiều hardening tốt (request write deadline, Windows named pipe retry/mode, benchmark gate parser). Tuy nhiên còn 1 regression mức High liên quan `CHATMINAL_DATA_DIR` làm endpoint path và DB path lệch nhau khi dùng relative path. Final sign-off chưa clean.

### Critical Issues
- None.

### High Priority
1. Relative `CHATMINAL_DATA_DIR` bị resolve không đồng nhất giữa app/daemon và store.
- `apps/chatminal-app/src/config.rs:81` + `apps/chatminal-app/src/config.rs:86`: relative path được normalize dưới `$HOME` (hoặc CWD fallback).
- `apps/chatminald/src/config.rs:115` + `apps/chatminald/src/config.rs:120`: daemon endpoint cũng normalize tương tự.
- Dependent mismatch: `crates/chatminal-store/src/lib.rs:865` + `crates/chatminal-store/src/lib.rs:869` giữ nguyên relative path thô.
- Impact: daemon lắng nghe endpoint ở `$HOME/<relative>/chatminald-*.sock` nhưng DB lại mở ở `<cwd>/<relative>/chatminald.db`; có thể gây split state, sai data location, hoặc permission lỗi tùy CWD service.
- Repro đã xác nhận: log runtime cho thấy DB `relative-review/chatminald.db` nhưng endpoint `/home/khoa2807/relative-review/chatminald-linux.sock`.
- Recommended fix: dùng chung helper normalize data-dir cho app/daemon/store (cùng policy absolute/relative/home fallback).

### Medium Priority
1. `CHATMINAL_BENCH_MAX_SECONDS` không được enforce nếu máy thiếu `timeout`/`gtimeout`.
- `scripts/bench/phase02-rtt-memory-gate.sh:125`-`scripts/bench/phase02-rtt-memory-gate.sh:140`
- Impact: bench có thể chạy vô hạn trong local env thiếu coreutils timeout, trái kỳ vọng “outer benchmark timeout”.
- Recommended fix: fail-fast khi không tìm thấy timeout binary (hoặc fallback watchdog bằng background killer).

2. Windows default pipe suffix có thể rơi về `default-user` khi username/hostname sanitize rỗng.
- `apps/chatminal-app/src/config.rs:57`-`apps/chatminal-app/src/config.rs:70`
- `apps/chatminald/src/config.rs:91`-`apps/chatminald/src/config.rs:104`
- Impact: collision endpoint giữa nhiều runtime “non-ASCII identity sanitized-empty” (edge case), giảm tính user-scope isolation.
- Recommended fix: derive suffix từ stable user SID/hash thô (không phụ thuộc sanitize rỗng), thêm length cap.

### Low Priority
1. Changelog có 2 mô tả dễ gây hiểu nhầm trạng thái hiện tại của request write-path.
- `docs/project-changelog.md:110` mô tả writer queue.
- `docs/project-changelog.md:120` mô tả direct writer lock.
- Impact: reader có thể hiểu sai implementation đang active nếu không đọc theo timeline.
- Recommended fix: thêm câu “writer queue was replaced by direct writer lock in post-review hardening”.

### Edge Cases Found by Scout
- Mutex wait trước write chưa nằm trong write deadline budget (`apps/chatminal-app/src/ipc/client.rs:157`); đây là residual risk concurrency, chưa thấy regression mới trực tiếp trong batch này.
- `request()` và `recv_event()` chia sẻ `frames_rx` mutex (`apps/chatminal-app/src/ipc/client.rs:137`), có thể tăng contention nếu caller dùng đồng thời.

### Positive Observations
- `apps/chatminal-app/src/ipc/client.rs` write loop xử lý partial write + retry `WouldBlock`/`TimedOut` rõ ràng.
- `apps/chatminal-app/src/ipc/transport/windows.rs` có endpoint validation + connect retry window bounded.
- `scripts/bench/phase02-rtt-memory-gate.sh` validate summary fields chặt hơn, giảm false-pass.
- Plan TODO check: `plans/260304-1442-chatminal-rewrite-production-completion/plan.md` TODO đều đã `[x]`.

### Recommended Actions
1. Fix High trước: đồng bộ normalize policy cho `CHATMINAL_DATA_DIR` giữa app/daemon/store.
2. Enforce timeout binary policy trong benchmark gate để `CHATMINAL_BENCH_MAX_SECONDS` luôn có hiệu lực.
3. Hardening suffix identity cho Windows named pipe để tránh fallback collision.
4. Clarify changelog wording cho write-path chronology.

### Metrics
- Type Coverage: N/A (Rust project)
- Test Coverage: không đo coverage % trong lượt review này
- Linting Issues: không chạy full lint
- Validation executed:
  - `bash -n scripts/bench/phase02-rtt-memory-gate.sh` ✅
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml concurrent_requests_receive_correct_response_variant` ✅
  - `cargo test --manifest-path apps/chatminald/Cargo.toml config::tests` ✅
  - `cargo check --manifest-path apps/chatminal-app/Cargo.toml --target x86_64-pc-windows-gnu` ✅
  - `cargo check --manifest-path apps/chatminald/Cargo.toml --target x86_64-pc-windows-gnu` ❌ (missing host toolchain `x86_64-w64-mingw32-gcc`, environment issue)

### Sign-off Conclusion
- Critical: **0**
- High: **1 (open)**
- Final sign-off status: **NOT CLEAN (blocked by High issue)**

### Unresolved Questions
1. Có chốt policy chính thức cho `CHATMINAL_DATA_DIR` relative path là “always under `$HOME`” cho toàn bộ app/daemon/store không?
2. Với benchmark gate local, có muốn coi thiếu `timeout`/`gtimeout` là hard error luôn không?
