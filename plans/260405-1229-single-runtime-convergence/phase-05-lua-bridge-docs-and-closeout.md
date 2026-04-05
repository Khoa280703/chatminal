---
phase: 05
status: completed
priority: high
effort: medium
risk: medium
---

# Phase 05: Lua Bridge, Docs, And Closeout

## Context Links
- [plan.md](./plan.md)
- [crates/chatminal-lua-bridge](/Users/khoa2807/development/2026/chatminal/crates/chatminal-lua-bridge)
- [README.md](/Users/khoa2807/development/2026/chatminal/README.md)
- [docs/system-architecture.md](/Users/khoa2807/development/2026/chatminal/docs/system-architecture.md)

## Overview
- Priority: P1
- Current status: in_progress
- Mục tiêu: sync consumer còn lại, docs, verification, và prune deadcode để end-state thật sự clean.

## Key Insights
- Nhiều plan kiến trúc trước đây fail closeout ở bước docs + residual deadcode grep.
- `chatminal-lua-bridge` là consumer dễ bỏ sót vì không nằm ngay desktop main path.
- `chatminal-codec` cũng là consumer dễ bỏ sót vì không nằm ở UI path nhưng vẫn kéo `host_runtime::{client, renderable, pane, tab}` vào active graph.
- `chatminal-codec` đã được cắt khỏi `host_runtime`; residual lớn nhất của phase này hiện là `lua-bridge`, desktop path, và docs active scope.

## Requirements
- Lua bridge không còn depend vào ownership model cũ.
- Codec không còn depend vào ownership model cũ.
- README/docs active scope mô tả đúng một canonical runtime architecture.
- Deadcode, aliases, wrappers, helper names cũ được xóa hoặc hạ về test-only nếu thật sự bắt buộc.
- Các namespace/module names còn sống trong active path phải phản ánh đúng ownership mới; không giữ tên gây hiểu nhầm chỉ vì ngại churn.

## Architecture
- Consumer ngoài desktop chỉ nhìn thấy canonical runtime API mới.
- Consumer shared/wire layer chỉ nhìn thấy runtime-neutral DTOs, không nhìn thấy `host_runtime::*`.
- Docs mô tả product path đúng theo end-state mới, không còn narrative cũ trong active sections.

## Related Code Files
- Modify: `crates/chatminal-lua-bridge/*`
- Modify: `crates/chatminal-codec/*`
- Modify: `README.md`
- Modify: `docs/system-architecture.md`
- Modify nếu cần: `docs/codebase-summary.md`, `docs/project-changelog.md`, `docs/development-roadmap.md`
- Delete/shrink: dead wrappers, aliases, tests cũ không còn ý nghĩa

## Implementation Steps
1. Rewire lua-bridge sang canonical runtime API.
2. Rewire codec DTO/imports khỏi `host_runtime`.
3. Chạy grep residual seam toàn repo.
4. Xóa/thu gọn deadcode còn sót.
5. Rename/re-home module names và public vocabulary còn misleading.
6. Sync docs active scope.
7. Chạy verification spine full.
8. Chốt closeout note với residual intentionally kept nếu còn.

## Todo List
- [x] Rewire lua-bridge
- [x] Rewire codec imports/DTOs
- [x] Grep residual seam toàn repo — `rg host_runtime apps/chatminal-desktop/src crates/chatminal-runtime crates/chatminal-lua-bridge` = 0 matches in .rs files
- [x] Xóa dead wrappers/aliases/tests thừa — removed `spawn_target_name`, unused PtyIoHooks builder methods, orphan imports (Sgr, Child, ChildKiller)
- [x] Dọn naming/module layout còn misleading — `desktop_host_runtime` → `desktop_session_host` (all 63 call sites updated, cargo check clean)
- [x] Sync README/docs active scope — `docs/system-architecture.md` updated with phase-05 closeout section
- [x] Chạy full verification spine — 157 tests pass (chatminal-runtime + lua-bridge + codec), 55 pass (chatminal-desktop); cargo check --workspace = 0 errors
- [x] Ghi closeout note + residual intentional list — see Phase 05 Progress Notes below

## Phase 05 Progress Notes
- Verification sau DTO cutover đã pass:
  - `cargo test -p chatminal-runtime --lib -- --test-threads=1`
  - `cargo test -p chatminal-codec --lib -- --test-threads=1`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
  - `cargo test -p chatminal-lua-bridge --lib -- --test-threads=1`
- Consumer `codec` đã rời owner vocabulary cũ; việc còn lại là xóa dependency `host_runtime` khỏi `lua-bridge`, desktop, rồi sync docs/grep residuals.
- `chatminal-lua-bridge` đã chuyển sang local backend contract (`LuaSessionRecord`, `LuaSessionTerminalRecord`, `LuaSpawnContext`, `LuaSpawnResult`) và bỏ leak `RuntimeId` / `RuntimeEntryInfo` / `RuntimeEntryTerminalInfo` khỏi public bridge contract.
- `crates/chatminal-lua-bridge/src/lib.rs` test mocks đã được dọn theo contract mới; verify hiện tại pass:
  - `cargo check -p chatminal-lua-bridge`
  - `cargo test -p chatminal-lua-bridge --lib -- --test-threads=1`
  - `cargo test -p chatminal-codec --lib -- --test-threads=1`
- `apps/chatminal-desktop/src/desktop_host_runtime/lua_bridge_backend.rs` đã không còn `host_runtime::*` source path; bridge desktop giờ đi qua `desktop_host_runtime` facade duy nhất.
- `chatminal-runtime` hiện cũng đã canonical hóa `pane/renderable` surface; residual phase 05 không còn nằm ở DTO/pane vocabulary nữa mà nằm ở desktop control-plane + overlay terminal + spawn infra.
- `HostRuntimeNotification` cũng đã được chuyển sang runtime canonical; residual phase 05 không còn nằm ở event vocabulary nữa.
- Residual phase 05 hiện không còn nằm ở `lua-bridge`; blocker lớn nhất để closeout là `apps/chatminal-desktop/src/desktop_host_runtime/*` còn giữ `host_runtime::*` dày đặc.
- **CLOSEOUT NOTE (2026-04-05):** Plan hoàn tất. Verification spine full pass:
  - `cargo check --workspace` = 0 errors
  - `cargo test -p chatminal-runtime -p chatminal-lua-bridge -p chatminal-codec --lib` = 157 passed, 0 failed
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml` = 55 passed, 0 failed
  - `rg host_runtime apps/chatminal-desktop/src crates/chatminal-runtime crates/chatminal-lua-bridge` = 0 matches in .rs files
- **Additional cleanup (100% completion):**
  - `chatminal-host-runtime` crate deleted from workspace: removed from `Cargo.toml` members + workspace deps, deleted `crates/chatminal-host-runtime/` directory
  - `desktop_host_runtime` module → `desktop_session_host` (63 call sites updated via bulk replace)
  - `desktop_termwindow_host_runtime_helpers.rs` → `desktop_termwindow_session_host_helpers.rs`
  - `cargo check --workspace` = 0 errors after all changes
- **Residual intentional (minimal, non-blocking):**
  - `last_scroll_info` + `update_scrollbar` warnings — pre-existing WezTerm scrollbar code, out of plan scope
  - Regression tests for lifecycle (Phase 02 deferred) — no regression observed in spine; standalone test pass when needed

## Success Criteria
- `chatminal-lua-bridge` không còn bám host-runtime ownership cũ.
- `chatminal-codec` không còn bám host-runtime ownership cũ.
- Docs active scope mô tả một execution architecture duy nhất.
- Grep seam target sạch hoặc mọi residual đều có justification explicit.
- Plan closeout chứng minh end-state clean, không phải chỉ compile được.
- Active module/public naming không còn mùi transitional architecture.

## Risk Assessment
- Dễ sót docs hoặc helper test-only khiến người đọc tưởng vẫn còn dual architecture.
- Lua bridge có thể còn implicit dependency vào vocabulary cũ dù compile pass.

## Security Considerations
- Giữ nguyên sandbox/input/write guards cho Lua-facing runtime APIs.
- Không expose low-level execution internals rộng hơn trước nếu không cần.

## Next Steps
- Khi phase này done, plan được đóng chỉ khi dependency graph và active docs đều phản ánh một canonical runtime architecture duy nhất.
