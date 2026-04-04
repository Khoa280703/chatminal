# Parallel Completion Plan

## Goal
Chia phần còn lại của `260401-0949-architecture-unification` thành các phase tuần tự; trong mỗi phase, các lane có file ownership tách biệt để nhiều người có thể làm cùng lúc mà không đè nhau.

## Rules
- Không có 2 lane trong cùng phase cùng sửa một file.
- Chỉ mở phase kế tiếp khi verification gate của phase hiện tại xanh.
- Nếu một lane cần đổi public contract, contract đó phải được chốt ở phase foundation trước.
- Phase cuối cùng mới làm rename/cosmetic sweep diện rộng.

## Critical Path
`Phase 01 -> Phase 02 -> Phase 03 -> Phase 04 -> Phase 05 -> Phase 06 -> Phase 07 -> Phase 08 -> Phase 09 -> Phase 10`

## Fastest Parallelizable Waves
- `Phase 01`: 2 lane song song
- `Phase 03`: 3 lane song song
- `Phase 04`: 2 lane song song
- `Phase 06`: 2 lane song song
- `Phase 07`: 2 lane song song
- `Phase 09`: 2 lane song song

## Phase List
1. [Phase 01](./phase-01-regression-and-contract-lock.md): khóa regression và contract lệch
2. [Phase 02](./phase-02-host-runtime-api-foundation.md): dựng host-runtime API foundation
3. [Phase 03](./phase-03-consumer-cutover-parallel.md): cutover consumer song song
4. [Phase 04](./phase-04-ui-and-desktop-facade-parallel.md): cleanup UI/facade song song
5. [Phase 05](./phase-05-control-plane-foundation.md): control-plane/singleton foundation
6. [Phase 06](./phase-06-runtimehost-wiring-parallel.md): wiring explicit RuntimeHost song song
7. [Phase 07](./phase-07-pty-owner-migration-parallel.md): PTY owner migration song song
8. [Phase 08](./phase-08-config-foundation.md): config foundation
9. [Phase 09](./phase-09-config-propagation-parallel.md): config propagation song song
10. [Phase 10](./phase-10-final-rename-and-sweep.md): rename/cosmetic/final sweep

## Staffing Aid
- [Workload Ranking](./workload-ranking.md): xếp hạng độ nặng/rủi ro/headcount của từng phase và lane để chia người
- [Macro 4-Phase Staffing](./macro-4-phase-staffing.md): gom 10 phase thành 4 macro-phase để quản lý 4 nhân sự
- [Employee Assignment](./employee-assignment.md): phân công trực tiếp cho team 4 người theo wave/lane/ownership
- [Independent Employee Split](./independent-employee-split.md): chia lại để 4 người làm độc lập trên branch riêng, ghép sau
- [Merge Checklist](./merge-checklist.md): checklist ghép 4 stream độc lập + integration backlog

## Ownership Map Summary
- `crates/chatminal-host-runtime/src/*`: foundation, control-plane, PTY compat, config-host-runtime
- `crates/chatminal-lua-bridge/src/*`: Lua bridge cutover, Lua RuntimeHost wiring
- `apps/chatminal-desktop/src/desktop_host_runtime/*`: desktop host adapter cutover, RuntimeHost wiring
- `apps/chatminal-desktop/src/desktop_termwindow_*`, `apps/chatminal-desktop/src/termwindow/*`, `apps/chatminal-desktop/src/overlay/*`: UI/internal typed-handle cleanup
- `apps/chatminal-desktop/src/frontend.rs`, `apps/chatminal-desktop/src/main.rs`, `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`, `apps/chatminal-desktop/src/desktop_commands.rs`, `apps/chatminal-desktop/src/desktop_spawn.rs`: desktop facade/config propagation
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/*`: session-native PTY ownership
- `crates/chatminal-config/src/*`: config foundation and dead-field sweep

## Exit Condition For 100%
- Không còn `Arc<Tab>` / `PaneId` / `TabId` ở public cross-crate boundary mục tiêu
- `static MUX` không còn là ownership root
- PTY cleanup/output lifecycle owner cuối chuyển khỏi Mux-based default semantics
- `configuration()` không còn là singleton đọc rải rác ở path mục tiêu đã scope
- Phase 02 rename/cosmetic chạy sau cùng khi kiến trúc functional đã đóng
