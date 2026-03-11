# Chatminal Session Execution Core Final Cutover

Status: planned
Goal: hoàn tất bước cuối để `session` trở thành execution primitive thật của Chatminal; active session execution path không còn phụ thuộc `mux/tab/pane` host để spawn, focus, split, move, close và route input. Render boundary có thể tạm giữ compatibility object nếu chưa thể bóc trong cùng plan.

## Problem Statement
- `direction-b` đã hoàn thành việc dọn app/public layer sang `session/surface/leaf`
- Nhưng active desktop/runtime path vẫn còn bridge xuống `mux::Tab` trong `EngineSurfaceAdapter`
- Vì vậy hiện trạng mới là `session facade over tab host`, chưa phải `session-native runtime`

## Target State
- `chatminal-session-runtime` là execution core duy nhất cho active desktop path
- `apps/chatminal-desktop` ở session flow chỉ nói với `SessionEngineShared` + session snapshots/events
- `chatminal_session_surface` không còn map `session <-> host Tab` trong active session flow
- `EngineSurfaceAdapter` không còn nằm trong active runtime path; nếu còn giữ chỉ ở compatibility/test slice riêng
- `chatminal-mux` không còn là execution host cho session flow; nếu còn tồn tại thì chỉ là engine/render compatibility dependency ngoài phạm vi plan này

## Phases
- Phase 01: Runtime Boundary Freeze And Inventory
- Phase 02: Session Core Commands Complete Cutover
- Phase 03: Desktop Session Host Bootstrap
- Phase 04: TermWindow Routing Migration
- Phase 05: Overlay Frontend And Action Migration
- Phase 06: Adapter Bypass And Active Path Removal
- Phase 07: Dependency Graph Prune And Hard Cleanup
- Phase 08: Verification Rollout And Post-Cutover Cleanup

## Progress
- Phase 01: pending
- Phase 02: pending
- Phase 03: pending
- Phase 04: pending
- Phase 05: pending
- Phase 06: pending
- Phase 07: pending
- Phase 08: pending

## Hard Invariants
- Không đụng `third_party/`
- Không thay terminal parser/render semantics ngoài phần host/runtime plumbing bắt buộc
- Không reintroduce public `tab/mux/pane` vocabulary vào app layer
- Không overclaim việc gỡ toàn bộ `mux` khỏi compile graph của desktop nếu desktop vẫn còn cần `mux::Pane`/notification/render compatibility
- Mỗi phase phải có grep gate + build/test gate trước khi sang phase tiếp theo

## Completion Gates
- `rg -n "EngineSurfaceAdapter|ChatminalEngineSurfaceAdapter" crates/chatminal-session-runtime apps/chatminal-desktop/src/chatminal_session_surface.rs`
  - expected: chỉ còn test/compat shims hoặc về zero sau Phase 06
- `rg -n "spawn_tab_or_window|move_pane_to_new_tab|focus_pane_and_containing_tab|get_tab\\(|get_pane\\(" crates/chatminal-session-runtime apps/chatminal-desktop/src/chatminal_session_surface.rs apps/chatminal-desktop/src/frontend.rs apps/chatminal-desktop/src/overlay apps/chatminal-desktop/src/termwindow`
  - expected: Phase 01 phải freeze inventory line-by-line cho active-session buckets; gate hoàn tất là không còn line active nào unresolved trong inventory đó. Không dùng raw grep-zero trên toàn file lớn nếu file còn chứa render/compat slices ngoài phạm vi plan
- `cargo check --workspace`
- `cargo test -p chatminal-session-runtime`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`

## Primary Files In Scope
- `crates/chatminal-session-runtime/src/*`
- `apps/chatminal-desktop/src/chatminal_session_surface.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/*`
- `apps/chatminal-desktop/src/termwindow/*`
- `apps/chatminal-desktop/src/frontend.rs`
- `apps/chatminal-desktop/src/overlay/*`
- `apps/chatminal-desktop/Cargo.toml`

## Out Of Scope
- Redesign UI
- Thay persistence model profile/session/history ngoài phần wiring bắt buộc
- Refactor toàn bộ `crates/chatminal-mux` internal engine-private cho mục đích purity thuần túy
- Gỡ toàn bộ `mux` khỏi desktop compile graph nếu phần render boundary vẫn còn dùng `mux::Pane` compatibility layer
