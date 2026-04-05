---
title: "Single Runtime Convergence"
description: "Collapse execution architecture into chatminal-runtime as the only runtime crate, retire chatminal-host-runtime completely from active product architecture, and leave desktop as UI shell only."
status: completed
priority: P0
effort: 8-12d
branch: main
tags: [architecture, runtime, cleanup, convergence, desktop]
created: 2026-04-05
---

# Single Runtime Convergence

## Goal
Gom toàn bộ active execution architecture về một owner duy nhất để product path không còn song song `chatminal-runtime` và `chatminal-host-runtime`. Sau khi plan hoàn tất, desktop chỉ còn là UI/render/input shell; session lifecycle, PTY execution, split/join tree, persistence/restore, focus/activation đều đi qua một canonical runtime layer duy nhất.

## Product Invariants
- Chatminal là **single desktop app**.
- Active product model chỉ có **một workspace logical duy nhất**; workspace không còn được xem là execution boundary độc lập.
- User-facing hierarchy canonical chỉ là:
  - app
  - profiles
  - sessions
- Join/split chỉ là cách render/tương tác nhiều session cùng lúc trong một app, không tạo thêm execution owner mới.
- Mọi kiến trúc giữ lại vì giả định multi-window, multi-workspace runtime ownership, hoặc daemonized execution đều bị xem là debt và phải bị cắt hoặc hạ xuống test-only/compat-only nếu thật sự bắt buộc.
- End-state bắt buộc phải **xóa hẳn `chatminal-host-runtime`** khỏi active architecture; runtime crate còn lại duy nhất là `chatminal-runtime`.

## In Scope
- `crates/chatminal-runtime/*`
- `crates/chatminal-host-runtime/*`
- `apps/chatminal-desktop/src/desktop_host_runtime/*`
- `apps/chatminal-desktop/src/chatminal_runtime/*`
- `crates/chatminal-codec/*`
- `crates/chatminal-lua-bridge/*`
- active docs/README liên quan execution architecture

## Explicit Non-Goals
- Không rewrite terminal emulator/core thêm lần nữa
- Không thêm feature product/UI mới
- Không thay đổi UI/UX hiện tại của Chatminal
- Không đổi session/profile data model của user-facing product nếu không bắt buộc
- Không tối ưu performance ngoài phần cần thiết để cut seam ownership
- Không giữ compatibility abstractions chỉ để “cho đẹp” nếu product path không cần
- Không giữ open-ended abstractions cho future multi-workspace/multi-app nếu current product không dùng

## Ground Truth From Source
- `chatminal-runtime` hiện đã own canonical `session_engine/*` code dưới `crates/chatminal-runtime/src/execution/*` và production startup path không còn đi qua `DesktopRuntimeExecutionBridge`
- `apps/chatminal-desktop/src/desktop_session_host/*` là app-local facade; `desktop_host_runtime/*` đã bị retire khỏi active source path
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` không còn `desktop_runtime_host()` hay `Arc<dyn RuntimeHost>` trong active path; desktop đang gọi thẳng `DesktopSessionHost` thay vì giữ trait-object bridge
- `RuntimeHost` seam đã bị cắt khỏi active source path; desktop facade hiện gọi concrete `DesktopSessionHost` trực tiếp và `crates/chatminal-runtime/src/runtime_host.rs` đã bị xóa
- `chatminal-codec` đã nội địa hóa wire DTO (`protocol_types.rs`) và không còn depend `chatminal-host-runtime`
- `chatminal-desktop` không còn depend `chatminal-host-runtime`; active dependency graph chỉ còn `chatminal-runtime` là runtime crate
- `crates/chatminal-host-runtime/` đã bị xóa khỏi workspace active path
- `apps/chatminal-desktop/src/desktop_session_host/session_engine/mod.rs` chỉ còn thin re-export sang `chatminal-runtime::execution`, không còn implementation owner song song

## Canonical End-State
- Chỉ còn **một** canonical execution owner cho session runtime lifecycle
- Chỉ còn **một runtime crate** đại diện runtime/execution architecture trong active product path: `chatminal-runtime`
- `chatminal-runtime` là owner canonical của:
  - session lifecycle
  - PTY spawn/write/resize/kill
  - split/join execution tree
  - focus/activation state execution-side
  - restore/history/replay canonical state
- `chatminal-desktop` chỉ còn:
  - render
  - input routing
  - sidebar/modal/chrome
  - DTO/view-model glue thật sự thuộc UI
- `chatminal-host-runtime` phải bị **retire khỏi active product path**:
  - không còn là runtime crate song song
  - không còn là dependency của desktop product path
  - không còn giữ public owner vocabulary `Window` / `Tab` / `Pane` như runtime model
- Product path không còn `RuntimeExecutionAdapter`, `RuntimeSessionHandleTrait`, `RuntimeHost`, `DesktopRuntimeExecutionBridge`, `desktop_runtime_host()`, hoặc bridge tương đương làm ownership thật
- Execution mental model canonical sau cutover chỉ còn:
  - `Profile`
  - `Session`
  - `SessionExecution`
  - `TerminalInstance` / `SplitNode`
- `Workspace`, `Window`, `Tab`, `Pane` không còn là owner concepts trong active execution architecture; nếu còn thì chỉ là render/presentation detail nội bộ, không phải architectural boundary

## Hard Constraints
- Không được kết thúc plan với `chatminal-host-runtime` còn tồn tại trong active dependency graph của product path.
- Không được kết thúc plan với hai crate cùng own session execution dưới tên khác nhau.
- Không được giữ lại trait bridge/adapter chỉ để preserve layering cũ nếu caller product path duy nhất là desktop app.
- Không được để desktop app vừa own UI state vừa own duplicate execution registry.
- Không được để `chatminal-lua-bridge` kéo ngược host-runtime execution ownership quay lại active graph.
- Không được để `workspace` tiếp tục là execution abstraction trung tâm nếu product invariant chỉ có một workspace logical.
- Không được thay đổi visual layout, interaction flow, text labels, menu structure, overlay behavior, hay sidebar/session UX hiện tại trừ khi đó là bug fix bắt buộc do refactor.

## Dependency End-State
### Required target graph
```text
chatminal-desktop
  └── chatminal-runtime
        └── chatminal-store
```

### Allowed residual
```text
chatminal-desktop
  └── utility/render/input crates

chatminal-lua-bridge
  └── chatminal-runtime
```

### Forbidden end-state
```text
chatminal-desktop ──→ chatminal-runtime
        └──────────→ chatminal-host-runtime   # forbidden

chatminal-runtime ──(trait bridge)──→ owner khác # forbidden

chatminal-lua-bridge ──→ chatminal-host-runtime  # forbidden
```

## Phases
| Order | Phase | Status | Purpose |
|---|---|---|---|
| 1 | [phase-01-freeze-canonical-boundary.md](./phase-01-freeze-canonical-boundary.md) | completed | Chốt execution owner, contract đích, inventory seam, và frozen ownership map |
| 2 | [phase-02-move-execution-ownership-into-runtime.md](./phase-02-move-execution-ownership-into-runtime.md) | completed | Đưa PTY/session execution + split tree ownership vào runtime canonical layer; seam `RuntimeHost` đã bị cắt khỏi active path |
| 3 | [phase-03-collapse-desktop-bridge-and-product-wiring.md](./phase-03-collapse-desktop-bridge-and-product-wiring.md) | completed | Xóa adapter product path, rewiring desktop thành UI shell thuần |
| 4 | [phase-04-retire-host-runtime-crate.md](./phase-04-retire-host-runtime-crate.md) | completed | Xóa hẳn chatminal-host-runtime khỏi active graph |
| 5 | [phase-05-lua-bridge-docs-and-closeout.md](./phase-05-lua-bridge-docs-and-closeout.md) | completed | Sync consumer còn lại, docs, verification, deadcode prune |

## Success Criteria
- Product path chỉ còn một canonical execution owner; không còn runtime business layer gọi qua execution bridge sang owner khác
- `chatminal-host-runtime` đã bị xóa khỏi active product architecture như một runtime crate
- Desktop app không còn depend `chatminal-host-runtime`
- `chatminal-lua-bridge` không còn depend `chatminal-host-runtime`
- `chatminal-codec` không còn depend `chatminal-host-runtime`
- `RuntimeExecutionAdapter`, `RuntimeSessionHandleTrait`, `RuntimeHost`, `DesktopRuntimeExecutionBridge`, `desktop_runtime_host()`, và shim cùng vai trò đã bị xóa khỏi active path
- Session execution concepts canonical chỉ còn một mental model xuyên suốt: session runtime + terminal instance/layout tree, không còn `Session -> Runtime -> Tab -> Pane` như owner chain chồng thêm một lớp lịch sử
- `workspace` không còn là execution abstraction sống trong active product architecture; nếu còn trong store/docs thì chỉ là persistence namespace/history artifact được ghi rõ
- `Window` / `Tab` / `Pane` không còn xuất hiện như public cross-crate runtime owner vocabulary
- UI/UX behavior hiện tại giữ nguyên đối với user flow đang có: startup, sidebar, session switch, join/split, overlays, mouse/keyboard routing, context menu, restore
- `cargo check --workspace`, `cargo test --workspace --lib --bins --tests`, `cargo test -p chatminal-codec --lib -- --test-threads=1`, `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`, `make window` xanh
- Grep active scope không còn seam cũ ngoài các block historical/docs được ghi rõ

## Explicit Removal List
- `RuntimeExecutionAdapter`
- `RuntimeSessionHandleTrait`
- `RuntimeHost`
- `DesktopRuntimeExecutionBridge`
- `desktop_runtime_host()`
- `DesktopSessionHost` như execution owner cross-crate
- bridge/wrapper tương đương trong `desktop_host_runtime`
- `crates/chatminal-host-runtime` như một active runtime crate
- any active execution owner named/typed around:
  - `Window`
  - `Tab`
  - `Pane`
  - `HostRuntimeHandle`
  - `workspace_layouts()` như execution owner bridge

## Main Risks
- Cutover ownership có thể làm lệch input/focus/render synchronization nếu execution state và UI state không chuyển cùng lúc
- Nếu split/join tree migrate nửa vời, joined sessions sẽ bị đúng metadata nhưng sai pane/render behavior
- Nếu host-runtime còn bị giữ như hidden dependency ở Lua/desktop tests, phase 4 sẽ tưởng done nhưng execution still duplicated dưới một tên khác
- Đây là refactor P0 có blast radius lớn; không được gộp phase mà thiếu verify gate từng bước

## Verification Spine
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- `cargo test -p chatminal-codec --lib -- --test-threads=1`
- `cargo test -p chatminal-runtime -- --test-threads=1`
- `cargo test -p chatminal-lua-bridge -- --test-threads=1`
- `make window`
- `rg -n "RuntimeExecutionAdapter|RuntimeSessionHandleTrait|RuntimeHost|DesktopRuntimeExecutionBridge|DesktopSessionHost|desktop_runtime_host\\(|HostRuntimeHandle|host_runtime|Window::new\\(|TabId|PaneId|mux_default\\(" crates apps docs README.md`
- `rg -n "workspace_layouts\(|WorkspaceLayoutRegistry|workspace_id" crates/chatminal-runtime apps/chatminal-desktop crates/chatminal-lua-bridge`

## Done Means
Plan này chỉ được tính là hoàn tất khi **kiến trúc active path thật sự chỉ còn `chatminal-runtime` là runtime crate duy nhất**. Nếu `chatminal-host-runtime` còn nằm trong active dependency graph, hoặc vẫn còn bridge trait để runtime business layer gọi sang owner khác, thì plan chưa done dù app vẫn compile.

## Final Acceptance Gate
Chỉ được gọi plan này là `completed` khi đồng thời đúng cả 6 điều kiện:
1. `chatminal-runtime` là owner duy nhất của session execution lifecycle.
2. `chatminal-desktop` không còn cần execution bridge layer để spawn/activate/focus/resize session.
3. `chatminal-host-runtime` đã bị loại khỏi active dependency graph của desktop, lua-bridge, và codec.
4. Vocabulary public cross-crate đã collapse về single-app model: profiles + sessions + runtime-owned terminal instances.
5. Docs active scope mô tả đúng single-app/single-workspace reality, không còn hứa hẹn kiến trúc tổng quát hơn thực tế.
6. Grep residual seams sạch hoặc mọi residual đều bị khóa vào test-only/historical-only với justification explicit.
