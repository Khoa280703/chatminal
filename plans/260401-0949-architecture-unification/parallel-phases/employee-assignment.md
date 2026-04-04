# Employee Assignment

> Current source of truth for the remaining closeout work of `260401-0949-architecture-unification`.
> This round assumes **4 employees + 1 lead lane**.

## Goal
Phân phối phần còn lại thành 5 lane thực dụng:
- 4 lane code độc lập tối đa
- 1 lane do lead giữ để merge/integration/plan sync/final gate

## Ground Rules
- Không overlap file giữa 4 employee lane.
- Lead lane được phép chờ merge và xử lý cross-cutting cuối; 4 employee lane thì không.
- Nếu trong lúc làm phát hiện cần chạm file ngoài ownership, không tự mở rộng scope; ghi lại để lead absorb ở merge wave.
- Không ai được tự claim plan `done`; chỉ lead chốt trạng thái sau full verify.

## Current Truth Before This Round
- Merge-wave verify hiện tại đã xanh:
  - `cargo check --workspace`
  - `cargo test --workspace --lib --bins --tests`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml`
  - `make window`
- Plan tổng vẫn chưa `done` vì còn nhóm `A. Must Close To Call Plan done` trong [merge-checklist.md](./merge-checklist.md).

## 5-Way Split

### Employee A
- Status:
  - `assigned`
- Stream:
  - `Host-Runtime Ownership And Lifecycle Final Cut`
- Ownership:
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `crates/chatminal-host-runtime/src/tab.rs`
  - `crates/chatminal-host-runtime/src/window.rs`
  - `crates/chatminal-host-runtime/src/pane.rs`
- Mission:
  - cắt nốt mutation/lifecycle paths còn đi qua compat `Mux` facade
  - đẩy ownership thật về `HostRuntimeRoot` sâu hơn nữa
  - internalize thêm public/raw `PaneId` / `TabId` ở provider side nếu nằm trong ownership này
- Must close in this lane:
  - helper mutation/focus/spawn/split/remove còn bám `Mux`
  - raw-id helper/provider surface còn public không cần thiết
- Must not touch:
  - `pty_io.rs`
  - `localpane.rs`
  - `localpane_hooks.rs`
  - `spawn_target.rs`
  - desktop/lua consumers
  - docs/plan
- Deliverable:
  - note rõ helper/path nào đã rời `Mux`
  - note rõ raw boundary nào đã internalize
- Verify:
  - `cargo check -p chatminal-host-runtime`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
  - `rg -n "with_mux\(|with_mux_strict\(|PaneId|TabId" crates/chatminal-host-runtime/src/lib.rs crates/chatminal-host-runtime/src/tab.rs crates/chatminal-host-runtime/src/window.rs crates/chatminal-host-runtime/src/pane.rs`

### Employee B
- Status:
  - `assigned`
- Stream:
  - `PTY Default Owner Final Cut`
- Ownership:
  - `crates/chatminal-host-runtime/src/pty_io.rs`
  - `crates/chatminal-host-runtime/src/localpane.rs`
  - `crates/chatminal-host-runtime/src/localpane_hooks.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
- Mission:
  - đóng triệt để compat PTY default owner khỏi Mux
  - giữ `mux_default()` chỉ như explicit compat opt-in seam
  - làm rõ owner cho output/cleanup/inline-error/localpane side effects
- Must close in this lane:
  - default owner ngầm còn quay về Mux
  - constructor/default path chưa owner-neutral
- Must not touch:
  - `lib.rs`
  - desktop shell/UI
  - lua-bridge
  - docs/plan
- Deliverable:
  - note owner mặc định mới của từng seam
  - grep note cho `mux_default()` còn lại và vì sao hợp lệ
- Verify:
  - `cargo check -p chatminal-host-runtime`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
  - `rg -n "mux_default\(" crates/chatminal-host-runtime/src/pty_io.rs crates/chatminal-host-runtime/src/localpane.rs crates/chatminal-host-runtime/src/localpane_hooks.rs crates/chatminal-host-runtime/src/spawn_target.rs`

### Employee C
- Status:
  - `assigned`
- Stream:
  - `Config Propagation Deep Sweep`
- Ownership:
  - `crates/chatminal-config/src/*`
  - `apps/chatminal-desktop/src/frontend.rs`
  - `apps/chatminal-desktop/src/main.rs`
  - `apps/chatminal-desktop/src/stats.rs`
  - `apps/chatminal-desktop/src/desktop_spawn.rs`
  - `apps/chatminal-desktop/src/desktop_commands.rs`
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - `crates/chatminal-lua-bridge/src/window.rs`
  - `crates/chatminal-lua-bridge/src/session.rs`
- Mission:
  - giảm tiếp `configuration()` singleton reads trong consumer/config paths thuộc ownership này
  - chuyển sang snapshot / explicit handle / propagated state
  - chốt phần config foundation còn thiếu mà không chạm host-runtime core
- Must close in this lane:
  - consumer-side config singleton reads trong desktop/lua/config scope này
  - helper mỏng chỉ wrap `configuration()` mà chưa thành propagated path thật
- Must not touch:
  - `crates/chatminal-host-runtime/src/*`
  - `crates/chatminal-lua-bridge/src/lib.rs`
  - `crates/chatminal-lua-bridge/src/leaf.rs`
  - docs/plan
- Deliverable:
  - grep before/after cho `configuration()` trong ownership
  - note path nào còn lại và vì sao chưa nằm trong scope này
- Verify:
  - `cargo check -p chatminal-config`
  - `cargo check -p chatminal-desktop`
  - `cargo check -p chatminal-lua-bridge`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - `rg -n "configuration\(" crates/chatminal-config/src apps/chatminal-desktop/src/frontend.rs apps/chatminal-desktop/src/main.rs apps/chatminal-desktop/src/stats.rs apps/chatminal-desktop/src/desktop_spawn.rs apps/chatminal-desktop/src/desktop_commands.rs apps/chatminal-desktop/src/chatminal_runtime/mod.rs crates/chatminal-lua-bridge/src/window.rs crates/chatminal-lua-bridge/src/session.rs`

### Employee D
- Status:
  - `assigned`
- Stream:
  - `Consumer Boundary Raw-ID Closure`
- Ownership:
  - `crates/chatminal-lua-bridge/src/lib.rs`
  - `crates/chatminal-lua-bridge/src/leaf.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - `apps/chatminal-desktop/src/overlay/copy.rs`
- Mission:
  - đóng tiếp raw/public boundary ở consumer side bằng typed helper/local adapter
  - giảm consumer-side phụ thuộc vào raw `PaneId` / `TabId` / concrete host primitives
- Must close in this lane:
  - raw boundary còn leak ở lua/desktop host adapter side thuộc ownership này
  - local wrappers còn widen back sang raw ids không cần thiết
- Must not touch:
  - `crates/chatminal-host-runtime/src/*`
  - `crates/chatminal-config/src/*`
  - session_engine
  - docs/plan
- Deliverable:
  - note rõ consumer boundary nào đã chuyển sang typed path
  - note residual raw ids còn lại nhưng nằm ở provider side để lead/A absorb
- Verify:
  - `cargo check -p chatminal-lua-bridge`
  - `cargo check -p chatminal-desktop`
  - `cargo test -p chatminal-lua-bridge`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`

### Lead (Tôi)
- Status:
  - `assigned`
- Stream:
  - `Merge, Final Grep, Scope Decision, Docs Sync`
- Ownership:
  - `plans/260401-0949-architecture-unification/*`
  - `plans/260401-0949-architecture-unification/parallel-phases/*`
  - `docs/project-changelog.md`
  - `docs/system-architecture.md`
  - `docs/codebase-summary.md`
  - any cross-cutting merge-only fix that cannot stay inside employee ownership
- Mission:
  - giữ baseline grep và verify gates
  - absorb các residual overlap giữa 4 lane
  - quyết định mục nào vào `done gate`, mục nào defer hợp lệ
  - sync docs/plan sau mỗi merge wave
  - chạy final full verify và chốt plan status
- May touch later, only after merges:
  - các file cross-cutting bị nhiều lane cùng cần nhưng không thể giao độc lập từ đầu
- Deliverable:
  - merge report cuối
  - explicit done/defer decision
  - final docs sync
- Verify:
  - `cargo check --workspace`
  - `cargo test --workspace --lib --bins --tests`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml`
  - `make window`

## Merge Order
1. Employee C
2. Employee D
3. Employee B
4. Employee A
5. Lead merge wave

## Why This Order
- `C`: mostly consumer/config side, risk thấp, dễ dọn singleton reads trước.
- `D`: consumer raw-boundary closure dựa vào API hiện có, conflict thấp hơn core host-runtime.
- `B`: PTY owner cut rủi ro lifecycle cao hơn, nên merge sau khi consumer side yên.
- `A`: ownership/core host-runtime chạm choke point nhiều nhất, merge cuối trong nhóm employee.
- `Lead`: xử lý residual overlap, grep sweep, docs, final done gate.

## Definition Of Success For This Round
- 4 employee lane nộp patch trong đúng ownership.
- Lead absorb xong residual overlap.
- Nhóm `A. Must Close To Call Plan done` trong [merge-checklist.md](./merge-checklist.md) được đóng hoặc re-scoped explicit.
- Chỉ khi đó plan tổng mới được chuyển sang `done`.
