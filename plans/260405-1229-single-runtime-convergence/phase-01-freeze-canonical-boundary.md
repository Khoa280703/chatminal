---
phase: 01
status: completed
priority: critical
effort: medium
risk: medium
---

# Phase 01: Freeze Canonical Boundary

## Context Links
- [plan.md](./plan.md)
- [runtime_bridge.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state/runtime_bridge.rs)
- [execution_bridge.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs)
- [chatminal-host-runtime/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)

## Overview
- Priority: P0
- Current status: completed
- Mục tiêu: đóng băng architecture target và inventory toàn bộ ownership seam phải cắt trước khi move code.

## Key Insights
- Nếu không chốt rõ “owner nào giữ execution thật”, các phase sau sẽ chỉ là rename hoặc move file cơ học.
- `chatminal-host-runtime` đang lẫn execution concern và UI-host concern; phải tách trước khi cut.
- Không có một seam mà có ít nhất hai seam song song phải cắt: `RuntimeExecutionAdapter` và `RuntimeHost`.
- `chatminal-codec` vẫn depend `host_runtime`, nên retire crate không thể chỉ nhìn desktop + lua-bridge.
- Done của phase này là có bản đồ ownership sạch, không phải code chạy tốt hơn ngay.

## Requirements
- Chốt canonical owner là `chatminal-runtime` execution layer.
- Lập inventory tất cả public/private seam còn khiến runtime depend sang owner khác.
- Phân loại module `keep in runtime`, `keep in desktop`, `retire`, `utility-only`.
- Chốt rõ `workspace` không còn là execution boundary active; chỉ còn app -> profiles -> sessions là product hierarchy canonical.

## Architecture
- Execution domain canonical:
  - PTY/session lifecycle
  - split tree / joined layout execution-side
  - terminal instance registry
  - focus/activation execution-side
  - runtime/client contract cho desktop và lua consumers
- Desktop-only domain:
  - render surface
  - UI hit-test/layout/sidebar/modal
  - window shell thật sự thuộc presentation
- Host-runtime residual domain:
  - end-state của plan này là **không còn active host-runtime domain**
  - nếu còn utility nào sống sót thì phải được re-home vào `chatminal-runtime` hoặc desktop UI modules với ownership mới rõ ràng

## Mandatory Output Of This Phase
- Một bảng ownership map cấp module:
  - source module
  - target home
  - owner sau cutover
  - reason keep/delete
- Một bảng forbidden residuals:
  - type names
  - functions
  - Cargo deps
  - docs vocabulary
- Một statement explicit rằng end-state product chỉ còn single-app/single-workspace logical model.

## Frozen Ownership Chain Today
1. `RuntimeState`
2. `RuntimeExecutionAdapter` (`crates/chatminal-runtime/src/state/runtime_bridge.rs`)
3. `DesktopRuntimeExecutionBridge` (`apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`)
4. `SessionEngineShared` / `StatefulSessionEngine` (`apps/chatminal-desktop/src/desktop_host_runtime/session_engine/*`)
5. `RuntimeHost` (`crates/chatminal-runtime/src/runtime_host.rs`)
6. `DesktopSessionHost` (`apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`)
7. `host_runtime::*` (`crates/chatminal-host-runtime/src/*`)
8. `Window` / `Tab` / `Pane` / `LocalPane`

Chuỗi này là điều plan phải xóa. End-state chỉ được còn `RuntimeState` + runtime-native execution modules + desktop presentation adapters mỏng.

## Frozen Ownership Map
| Source | Current role | Target home after cutover | Decision | Why |
|---|---|---|---|---|
| `crates/chatminal-runtime/src/state/runtime_bridge.rs` | Trait bridge từ runtime sang desktop-owned execution | runtime-native execution modules trong `crates/chatminal-runtime/src/` | delete/replace | runtime không được depend vào owner khác |
| `crates/chatminal-runtime/src/runtime_host.rs` | Trait bridge thứ hai từ desktop facade vào execution owner | collapse vào runtime API trực tiếp | delete/replace | cross-crate execution trait này giữ dual ownership |
| `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs` | Concrete adapter cho `RuntimeExecutionAdapter` | none | delete | owner execution phải vào runtime |
| `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/*` | Execution registry, runtime tree, PTY-facing session engine | `crates/chatminal-runtime/src/execution/*` | migrate | đây là execution owner thật hiện tại |
| `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` | `RuntimeHost` impl + pane/render sync + host-runtime glue | split: execution bits -> runtime, render glue -> desktop | split | file này đang trộn owner execution với presentation |
| `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs` | UI-facing pane adapter cho termwindow/render/input | desktop shell/presentation | keep + re-home nếu cần | đây là presentation adapter hợp lệ nếu không own lifecycle |
| `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` | bootstrap, overlay aliases, host-runtime re-export, embedded runtime holder | split: runtime bootstrap -> runtime crate, overlay UI aliases -> desktop | split | module này đang là facade lai, gây hiểu sai ownership |
| `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` | desktop-facing runtime facade nhưng còn expose host/session engine types | desktop client/view-model glue tối thiểu | shrink | phải thôi export `DesktopSessionHost`, `SessionEngineShared`, `desktop_runtime_host()` |
| `crates/chatminal-host-runtime/src/localpane.rs` | PTY + terminal leaf implementation | runtime execution leaf module | migrate | đây là execution primitive thật |
| `crates/chatminal-host-runtime/src/localpane_hooks.rs` | side effects cho input/output/alert/cleanup | runtime events + desktop subscribers, không giữ module cũ | delete | module này chỉ gói callback bag của host-runtime cũ |
| `crates/chatminal-host-runtime/src/pty_io.rs` | PTY reader/output dispatch/cleanup | runtime execution IO module | migrate | đây là runtime execution concern |
| `crates/chatminal-host-runtime/src/pane.rs` | abstract terminal surface trait + search/selection/render helpers | desktop presentation surface hoặc runtime/internal surface tùy caller | split | trait này hiện bị dùng lẫn cho execution và render |
| `crates/chatminal-host-runtime/src/renderable.rs` | terminal snapshot DTOs (`StableCursorPosition`, `RenderableDimensions`) + terminal helpers | runtime/shared terminal DTO layer | migrate | đây là canonical terminal-state contract, không nên ở host-runtime |
| `crates/chatminal-host-runtime/src/tab.rs` | split tree + active pane + runtime entry container | runtime execution layout tree | migrate then rename | logic split/join là execution canonical, nhưng tên `Tab` phải biến mất |
| `crates/chatminal-host-runtime/src/window.rs` | single root window mux container | delete or collapse into desktop-only root render scope store | retire | product chỉ có single app window; không cần owner `Window` riêng |
| `crates/chatminal-host-runtime/src/spawn_target.rs` | spawn backend + split source | runtime execution spawn module | split | spawn belongs to runtime; host-specific wrappers phải mất |
| `crates/chatminal-host-runtime/src/termwiztermtab.rs` | overlay terminal/view utility | desktop render shell | migrate | utility render, không phải execution owner |
| `crates/chatminal-host-runtime/src/client.rs` / `activity.rs` | host identity/activity state | desktop-only shell / lua compat DTO nếu còn cần | migrate or delete | không được kéo execution ownership đi theo |
| `crates/chatminal-lua-bridge/src/lib.rs` | Lua API đang bind thẳng `host_runtime::{Pane, SplitRequest, RuntimeEntryInfo,...}` | runtime canonical API + desktop-neutral DTOs | rewire | lua không được depend host-runtime |
| `crates/chatminal-codec/src/lib.rs` | codec types đang import `host_runtime::{client, renderable, tab}` | runtime/shared terminal DTOs or local codec DTOs | rewire | codec trong active graph không được giữ host-runtime dep |

## Frozen Consumer Contracts
- Desktop sau cutover chỉ được gọi runtime qua:
  - `RuntimeState`
  - runtime client APIs/DTOs
  - render snapshot / terminal binding DTOs không expose owner `Window` / `Tab` / `Pane`
- Lua bridge sau cutover chỉ được thấy:
  - session/profile/runtime ids
  - split/layout DTO canonical của runtime
  - terminal handle/binding DTO canonical
- Codec sau cutover chỉ được serialize:
  - runtime-neutral DTOs
  - render snapshot / cursor / split info đã re-home
  - không import `host_runtime::*`

## Forbidden Residuals
| Kind | Residual | Rule |
|---|---|---|
| Trait seam | `RuntimeExecutionAdapter`, `RuntimeSessionHandleTrait`, `RuntimeHost` | phải xóa khỏi active path |
| Desktop facade | `DesktopRuntimeExecutionBridge`, `desktop_runtime_host()`, `DesktopSessionHost` as public owner | phải xóa hoặc hạ thành desktop-local presentation helper |
| Cargo dep | `chatminal-host-runtime` trong `chatminal-desktop`, `chatminal-lua-bridge`, `chatminal-codec` | phải biến mất |
| Vocabulary | `Window`, `Tab`, `Pane`, `HostRuntimeHandle`, `RuntimeEntryInfo` như public owner terms | phải biến mất khỏi cross-crate active contract |
| API smell | `SessionEngineShared`, `StatefulSessionEngine` lộ ra ngoài runtime crate | phải bị internalize hoặc xóa |

## Cutover Verify Gates
- `cargo metadata --format-version 1 --no-deps` không còn dependency `chatminal-host-runtime` trong `chatminal-desktop`, `chatminal-lua-bridge`, `chatminal-codec`
- `rg -n "RuntimeExecutionAdapter|RuntimeSessionHandleTrait|RuntimeHost|DesktopRuntimeExecutionBridge|DesktopSessionHost|desktop_runtime_host\\(" crates apps`
- `rg -n "host_runtime::" crates apps`
- `rg -n "SessionEngineShared|StatefulSessionEngine" apps/chatminal-desktop crates/chatminal-runtime crates/chatminal-lua-bridge`
- active docs không còn mô tả execution owner bằng `Window` / `Tab` / `Pane`

## Concrete Cut Points
1. Cắt trait seam tại `crates/chatminal-runtime/src/state/runtime_bridge.rs` và `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`.
2. Thay `RuntimeHandle = Arc<Mutex<dyn RuntimeSessionHandleTrait>>` bằng runtime-owned concrete handle.
3. Cắt seam `RuntimeHost` tại `crates/chatminal-runtime/src/runtime_host.rs` và `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`.
4. Xóa `session_tab_shim` / `ensure_mux_tab_shim` trong `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`; đây là compat path giữ owner chain `Session -> runtime_id -> host tab shim`.
5. Xóa bootstrap qua `HostRuntimeRoot` ở `crates/chatminal-host-runtime/src/lib.rs` và `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`.
6. Xóa proxy seam `apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs`.
7. Hợp nhất `WorkspaceLayoutRegistry`; không được để một registry sống ở runtime bridge và một registry sống trong session engine.

## Related Code Files
- Modify: `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`
- Audit: `crates/chatminal-host-runtime/src/*`
- Audit: `apps/chatminal-desktop/src/desktop_host_runtime/*`
- Audit: `crates/chatminal-codec/*`
- Audit: `crates/chatminal-lua-bridge/*`

## Implementation Steps
1. Inventory toàn bộ trait/bridge/owner chain hiện tại.
2. Ghi rõ canonical execution owner và DTO/boundary đích.
3. Phân loại từng module của `chatminal-host-runtime`: migrate / utility-only / delete.
4. Chốt consumer contract mới cho desktop, lua-bridge, và codec.
5. Viết migration checklist cho phase 2-5.

## Todo List
- [x] Liệt kê full ownership chain hiện tại
- [x] Map trait/seam cần xóa
- [x] Chốt canonical owner/module map
- [x] Chốt consumer contract mới
- [x] Định nghĩa cutover verify gates

## Success Criteria
- Có module map rõ ràng: cái gì về runtime, cái gì ở desktop, cái gì retire.
- Không còn ambiguity về owner của PTY/session execution.
- Có grep target cụ thể cho seams phải biến mất ở end-state.
- Có danh sách explicit những thứ **không được phép** còn sống sau plan closeout.

## Risk Assessment
- Nếu phân loại sai `host-runtime`, phase 2 sẽ nhét cả UI-host concern vào runtime.
- Nếu giữ contract mơ hồ, phase 3/4 sẽ mắc lại bridge dưới tên mới.

## Security Considerations
- Không đổi capability security surface ở phase này.
- Chỉ chốt ownership boundaries và migration rules.

## Next Steps
- Phase 02 chỉ bắt đầu khi canonical boundary và module map đã được review xong.
