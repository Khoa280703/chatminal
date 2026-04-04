# Independent Employee Split

## Goal
Chia lại công việc để 4 người có thể code **độc lập thật** trên branch/worktree riêng, không phải đợi nhau mở contract giữa chừng. Bước ghép và cleanup cross-cutting sẽ làm sau.

## Hard Truth
- Không thể chia toàn bộ `260401-0949-architecture-unification` thành 4 nhánh **vừa độc lập hoàn toàn vừa hoàn thành 100% plan**.
- Nếu ép độc lập tuyệt đối, phải chấp nhận:
  - mỗi stream chỉ làm phần **self-contained**
  - các nhát phá contract/final cleanup sẽ dồn về bước merge/integration sau

## Split Principle
- Không overlap file giữa 4 employee.
- Không stream nào được yêu cầu stream khác mở API mới trước rồi mới làm được.
- Mỗi stream chỉ dùng **contract hiện có** hoặc thêm **compat helper cục bộ trong chính ownership của mình**.
- Không stream nào được claim “done phase” nếu phần đó vẫn phụ thuộc merge của stream khác.

## Recommended Setup
- Mỗi employee dùng 1 worktree/branch riêng.
- Không cherry-pick chéo giữa các employee trong lúc đang làm.
- Chỉ merge về nhánh tích hợp khi stream đã:
  - build trong scope của nó
  - có note rõ changed files
  - có note rõ compat assumptions

## Current Status
- `Employee A`: completed
- `Employee B`: completed
- `Employee C`: completed
- `Employee D`: completed
- Verification run after all 4 streams landed together:
  - `cargo check -p chatminal-host-runtime`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
  - `cargo check -p chatminal-lua-bridge`
  - `cargo test -p chatminal-lua-bridge`
  - `cargo check -p chatminal-desktop`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - `cargo test --workspace --lib --bins --tests`
  - `make window`
- Result:
  - host-runtime: `23/23` tests pass
  - lua-bridge: `5/5` tests pass
  - desktop: `91/91` tests pass
  - workspace `--lib --bins --tests`: pass
  - desktop smoke launch: pass
- Remaining work moved out of employee streams:
  - only `Integration Backlog`

## Employee Split

### Employee A: Host Runtime Boundary Additions
- Goal:
  - thêm/nắn **boundary helpers mới** ở host-runtime mà không phá caller hiện tại
  - chuẩn bị vật liệu cho merge step sau, nhưng không ép B/C/D phải đợi
- Ownership:
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `crates/chatminal-host-runtime/src/pane.rs`
  - `crates/chatminal-host-runtime/src/tab.rs`
  - `crates/chatminal-host-runtime/src/window.rs`
  - `crates/chatminal-host-runtime/src/client.rs`
- Allowed:
  - thêm DTO/helper mới
  - thêm typed-wrapper helper mới
  - narrow visibility nếu không làm gãy compile ngoài ownership này
  - thêm tests cho helper mới
- Not Allowed:
  - không đổi `spawn_target.rs`, `pty_io.rs`, `localpane*.rs`
  - không đổi desktop/lua callsites
  - không cố bỏ `static MUX` trong stream này
  - không xóa compat API mà B/C đang còn dùng
- Definition of Done:
  - có thêm boundary API mới usable ở merge step
  - không làm hỏng caller hiện tại
  - compile/test host-runtime xanh
- Why independent:
  - A chỉ làm additive hardening ở host-runtime core; B/C/D không cần chờ A để tiếp tục branch của họ

### Employee B: Desktop Shell Local Cleanup
- Goal:
  - dọn desktop/UI/facade theo hướng local-first, typed-first, nhưng chỉ dùng API hiện có
  - giảm raw-id và singleton scatter ở desktop shell mà không đòi host-runtime đổi tiếp
- Ownership:
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_commands.rs`
  - `apps/chatminal-desktop/src/desktop_spawn.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_*`
  - `apps/chatminal-desktop/src/termwindow/*`
  - `apps/chatminal-desktop/src/overlay/*`
  - `apps/chatminal-desktop/src/frontend.rs`
  - `apps/chatminal-desktop/src/main.rs`
  - `apps/chatminal-desktop/src/stats.rs`
- Allowed:
  - thay raw `u64` nội bộ desktop bằng wrapper/helper cục bộ
  - gom fallback logic quanh `desktop_session_host()`
  - snapshot config/read local hơn ở desktop side
  - thêm desktop tests/smoke helpers
- Not Allowed:
  - không sửa `session_engine/*`
  - không sửa `crates/chatminal-host-runtime/src/*`
  - không sửa `crates/chatminal-lua-bridge/src/*`
- Definition of Done:
  - desktop shell bớt raw-id hơn
  - fallback/session-host logic gọn hơn
  - compile/test desktop xanh trong scope này
- Why independent:
  - B chỉ refactor caller-side bằng contract đang có; không cần chờ A/C/D

### Employee C: Lua Bridge Self-Contained Decoupling
- Goal:
  - tiếp tục gom toàn bộ Lua bridge qua `LuaBridgeHost`/local helpers
  - bỏ phụ thuộc trực tiếp vào raw tuple/guard style trong chính crate lua-bridge
- Ownership:
  - `crates/chatminal-lua-bridge/src/lib.rs`
  - `crates/chatminal-lua-bridge/src/window.rs`
  - `crates/chatminal-lua-bridge/src/leaf.rs`
  - `crates/chatminal-lua-bridge/src/session.rs`
- Allowed:
  - thay direct lookup bằng local host wrapper
  - đổi internal flow sang typed terminal handle nhiều hơn
  - giữ compat methods nếu cần (`pane_id()`, etc.) nhưng không lan raw shape mới ra ngoài
  - thêm lua-bridge tests
- Not Allowed:
  - không yêu cầu host-runtime mở API mới rồi mới làm
  - không sửa desktop files
  - không sửa PTY/session-engine
- Definition of Done:
  - lua-bridge dùng local host adapter nhất quán hơn
  - ít direct dependency vào raw tab/pane/tab guard hơn
  - compile/test lua-bridge xanh
- Why independent:
  - C tự chứa trong một crate; merge sau mới tận dụng thêm API mới từ A nếu cần

### Employee D: PTY And Session-Engine Ownership Isolation
- Goal:
  - tách hidden fallback/default owner trong PTY + session-engine path
  - đẩy lifecycle ownership rõ hơn ở hook bundle/session-engine side
- Ownership:
  - `crates/chatminal-host-runtime/src/pty_io.rs`
  - `crates/chatminal-host-runtime/src/localpane.rs`
  - `crates/chatminal-host-runtime/src/localpane_hooks.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/*`
- Allowed:
  - đổi default constructor sang explicit hook bundle/no-op semantics
  - chuyển cleanup/output/error owner rõ hơn ở PTY path
  - cải thiện session-engine exit/input/process-metadata lifecycle
  - thêm PTY/session-engine tests
- Not Allowed:
  - không sửa `crates/chatminal-host-runtime/src/lib.rs`
  - không sửa desktop shell/UI files ngoài `session_engine/*`
  - không đụng lua-bridge
- Definition of Done:
  - hidden fallback giảm rõ
  - lifecycle tests xanh
  - compile/test host-runtime + session_engine xanh
- Why independent:
  - D giữ trọn PTY/session-engine seam; không cần chờ contract từ A/B/C nếu không tự mở rộng scope

## Integration Backlog
- Các việc này **không giao độc lập cho 4 employee ở round hiện tại**; giữ lại cho merge step:
  - bỏ thật `runtime_entry_by_runtime_id(...) -> Arc<Tab>`
  - bỏ thật `runtime_entry_by_session_id(...) -> Arc<Tab>`
  - bỏ `static MUX` làm ownership root
  - đóng triệt để compat PTY default owner khỏi Mux
  - unify config foundation/propagation toàn stack
  - rename/cosmetic/final grep sweep

## Merge Order
1. Merge `Employee C`
2. Merge `Employee B`
3. Merge `Employee D`
4. Merge `Employee A`
5. Chạy integration backlog

## Why This Merge Order
- `C` độc lập nhất, conflict thấp nhất.
- `B` caller-side nhiều nhưng mostly desktop-local.
- `D` rủi ro cao hơn, nên merge khi desktop/Lua caller-side đã yên hơn.
- `A` cố ý để cuối vì đây là stream dễ chạm boundary/choke point nhất; additive helper merge sau sẽ ít chặn người khác hơn.

## What This Fixes Compared To Old Split
- Không còn mô hình “mọi người đợi A mở contract”.
- Không còn giao fake-parallel phase mà thực chất bị block bởi foundation.
- Mỗi employee có một cụm file đủ lớn để làm thật, nhưng không đụng ownership của người khác.

## What This Does Not Promise
- Không hứa rằng merge cuối sẽ trivial.
- Không hứa rằng 4 stream này tự động làm plan lên 100% ngay khi xong riêng lẻ.
- Nó chỉ tối ưu cho:
  - tốc độ code song song
  - ít đợi nhau
  - merge có kiểm soát ở bước sau
