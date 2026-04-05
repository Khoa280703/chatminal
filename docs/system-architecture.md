# System Architecture

Last updated: 2026-04-05

## Current Product Shape
- Chatminal là một desktop app duy nhất: `apps/chatminal-desktop`
- Runtime canonical duy nhất: `crates/chatminal-runtime`
- Persistence canonical: `crates/chatminal-store`
- Terminal domain canonical: `crates/chatminal-terminal-emulator` (`engine_term`)
- Lua bridge canonical: `crates/chatminal-lua-bridge`
- Codec canonical: `crates/chatminal-codec`

`crates/chatminal-host-runtime` đã bị xóa khỏi workspace active path. `desktop_host_runtime` cũng đã bị retire và thay bằng `desktop_session_host` như app-local host facade.

## Canonical Ownership
### `chatminal-runtime`
Owner của các concern sau:
- session lifecycle
- session/profile/workspace persistence contract
- PTY spawn, write, resize, kill
- runtime IDs, terminal instance IDs, render-target IDs
- execution event bus
- session execution engine dưới `src/execution/*`
- history/restore/startup recipe/state reconciliation

### `chatminal-desktop`
Owner của các concern sau:
- window bootstrap
- sidebar, modal, overlay, palette, context menu
- render/input shell
- mapping UI state sang runtime state
- session view layout/presentation trong cửa sổ desktop

### `desktop_session_host`
Đây là app-local facade, không phải runtime owner thứ hai.
Nó chỉ làm các việc:
- nối `chatminal-desktop` với `chatminal-runtime`
- giữ window-local pane/session handle registry cho render/input
- expose overlay-facing adapter types để shell hiện tại tiếp tục chạy

## Dependency Graph
```text
chatminal-desktop
├── chatminal-runtime
├── chatminal-lua-bridge
└── utility/render/input crates

chatminal-lua-bridge
└── chatminal-runtime

chatminal-runtime
└── chatminal-store
```

Forbidden architecture đã bị loại khỏi active path:
- `chatminal-desktop -> chatminal-host-runtime`
- `chatminal-lua-bridge -> chatminal-host-runtime`
- `chatminal-codec -> chatminal-host-runtime`
- runtime ownership qua `RuntimeHost` / `RuntimeExecutionAdapter`

## Execution Model
User-facing model canonical:
```text
app
└── profiles
    └── sessions
        ├── runtime
        ├── terminal instances
        └── render target / view binding
```

Internal execution model canonical:
- `RuntimeId`
- `TerminalInstanceId`
- `SessionRenderTargetId`
- `SessionViewId`
- `SessionGroupId`
- `WorkspaceLayoutState`

`Window`, `Tab`, `Pane` không còn là cross-crate execution owner vocabulary. Nếu còn xuất hiện, đó chỉ là UI/render terminology hoặc local adapter detail.

## Runtime Flow
### Startup
1. Desktop app boot
2. `desktop_session_host` khởi tạo local host facade
3. `chatminal-runtime` load persisted state từ store
4. active session/runtime được hydrate vào desktop shell
5. sidebar/render state subscribe runtime events

### Session activation
1. UI chọn session hoặc terminal
2. desktop facade resolve `session_id` / `terminal_handle`
3. `chatminal-runtime` activate/focus runtime canonical
4. desktop shell cập nhật sidebar/render state theo snapshot mới

### Session output
1. PTY output vào runtime execution engine
2. runtime publish event/snapshot update
3. desktop host bridge materialize pane/render info
4. termwindow/sidebar repaint

## Persistence Model
SQLite (`chatminal-store`) giữ:
- profiles
- sessions
- canonical scrollback/history
- workspace layout state
- startup recipe / lifecycle preferences

Runtime load/restore không depend vào host-runtime cũ.

## Lua Bridge Contract
`chatminal-lua-bridge` nói chuyện với `chatminal-runtime` qua backend được cài bởi desktop app.

Supported responsibilities:
- query session/window/pane hiện tại
- spawn session root/split theo product semantics hiện tại
- activate session/terminal
- đọc terminal metadata / lines / dimensions

Unsupported in single-runtime Chatminal:
- legacy session zoom
- legacy pane rotation

Các API compatibility nếu còn tồn tại phải trả lỗi `unsupported`, không được giả vờ “session không tồn tại”.

## Invariants
- Chỉ có một runtime crate active: `chatminal-runtime`
- Desktop không được own execution registry song song với runtime
- UI shell không được tự suy luận active execution owner ngoài snapshot/runtime bindings canonical
- Docs phải mô tả đúng single-app reality hiện tại, không giữ ground truth lịch sử lẫn với current state

## Verification Snapshot
Đã verify trên source hiện tại:
- `cargo check --workspace`
- `cargo test -p chatminal-runtime -- --test-threads=1`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- `cargo tree -p chatminal-desktop -e normal` không còn `chatminal-host-runtime`

## Residual Compatibility Notes
- `desktop_session_host/session_engine/mod.rs` chỉ còn là thin re-export sang `chatminal-runtime::execution`; không còn implementation ownership song song.
- Một số naming legacy như `host_*` vẫn có thể còn trong app-local facade. Chúng không còn biểu thị runtime owner thứ hai; chỉ là rename debt nhỏ nếu còn sót.
