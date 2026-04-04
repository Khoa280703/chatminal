# Final Closeout Checklist

## Contract
Checklist này đã được hoàn tất theo source thực tế của closeout `2026-04-03`.

Execution summary nằm ở [phase-05-final-closeout.md](./phase-05-final-closeout.md).

## Audit Reset
- [x] audit dựa trên code thật, không dựa trên artifact `done` cũ
- [x] file plan/checklist đã được sync lại để không còn claim `done` sai

## Gate 1. Runtime Ownership
- [x] `initialize_host_runtime()` không còn dựng `Mux` như runtime owner mặc định
- [x] `shutdown_host_runtime()` không còn phụ thuộc `Mux::shutdown()` như owner lifecycle chính
- [x] notify path active không còn coi `Mux::notify_from_any_thread` là owner mặc định của product flow
- [x] add/remove/focus/spawn/split lifecycle không còn materialize `Mux` như owner thật trong product path

## Gate 2. PTY Default Owner
- [x] không còn hidden/default owner fallback về `mux_default()` trong product path
- [x] `mux_default()` nếu còn chỉ tồn tại như explicit compat seam/alias
- [x] desktop spawn boundary không còn chọn `mux_default()` làm default behavior
- [x] unit tests khóa semantics owner mới

## Gate 3. Raw ID Boundary
- [x] public/cross-crate API không còn raw `PaneId` / `TabId` blocker
- [x] raw ids còn lại chỉ là crate-local internals hoặc explicit wire-compat
- [x] desktop/lua consumer paths không còn widen raw ids chỉ để đi qua migrated boundary
- [x] grep audit cho `PaneId`, `TabId`, `.pane_id(`, `as u64`, `as usize` đã được ghi kết luận cuối

## Gate 4. Config Scope
- [x] `configuration(` remaining hits đã được phân loại xong
- [x] `Phase 04 Step 3/4` đã có quyết định explicit: follow-up
- [x] deferred scope đã được ghi rõ trong `plan.md` và `phase-04-config-independence.md`

## Gate 5. Rename Scope
- [x] `Phase 02` đã được tách follow-up explicit
- [x] rename/cosmetic không còn là trạng thái mơ hồ trong parent plan

## Gate 6. Status And Docs Sync
- [x] [plan.md](./plan.md) khớp source thực tế
- [x] [phase-03-kill-mux-singleton.md](./phase-03-kill-mux-singleton.md) khớp source thực tế
- [x] [phase-04-config-independence.md](./phase-04-config-independence.md) khớp source thực tế
- [x] [phase-05-final-closeout.md](./phase-05-final-closeout.md) đã hoàn tất
- [x] [parallel-phases/merge-checklist.md](./parallel-phases/merge-checklist.md) không còn claim closeout xong sớm
- [x] `docs/project-changelog.md` đã sync
- [x] `docs/system-architecture.md` đã sync
- [x] `docs/codebase-summary.md` đã sync

## Gate 7. Verification
- [x] `cargo check --workspace`
- [x] `cargo test --workspace --lib --bins --tests`
- [x] `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- [x] `make window` bounded smoke launch

## Done Rule
- [x] toàn bộ gate phía trên đã xanh
- [x] `status: done` trong [plan.md](./plan.md) đã được cập nhật sau khi verify xong
