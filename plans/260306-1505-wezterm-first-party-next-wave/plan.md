## WezTerm First-Party Next Wave

Status: completed
Priority: high
Scope: chỉ xử lý frontier gần nhất của migration

### Context
- GUI source entry đã nằm ở `apps/chatminal-wezterm-gui/src`.
- Một nhóm helper/foundation/runtime crates lớn đã bóc sang `crates/chatminal-*`, gồm cả `termwiz`, `mux`, `config`, `wezterm-term`, `wezterm-ssh`.
- Shadow manifest/subtree copies trong `crates/` đã được dọn để Cargo/IDE không mở nhầm crate cũ.
- Root workspace đã không còn third-party path dep trực tiếp; frontier còn lại là cleanup để `third_party/wezterm` thành reference-only rõ ràng hơn.
- `cargo check -p chatminal-wezterm-gui` đã pass lại trên host Linux hiện tại sau khi sửa wrapper IME X11 và asset path first-party.
- `cargo test -p chatminal-wezterm-font` đã pass sau khi vá bootstrap libpng vendored (`pngsimd.c`).

### TODO
- [x] Sửa asset/data path trong `crates/chatminal-termwiz-funcs/src/lib.rs` để không còn phụ thuộc layout cũ của `third_party/wezterm/lua-api-crates`.
- [x] Mang theo `xterm-256color` data vào vùng first-party của `chatminal-termwiz-funcs` rồi cập nhật `include_bytes!`.
- [x] Re-run verify tối thiểu sau fix path:
  - `cargo fmt --all`
  - `cargo check -p chatminal-wezterm-gui`
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
  - `cargo test --manifest-path apps/chatminald/Cargo.toml`
- [x] Update docs ngắn cho wave vừa verify:
  - `docs/development-roadmap.md`
  - `docs/project-changelog.md`
- [x] Bóc thêm batch foundation/parser crates ít rủi ro:
  1. `async_ossl`
  2. `wezterm-config-derive`
  3. `wezterm-dynamic-derive`
  4. `bintree`
  5. `vtparse`
  6. `wezterm-bidi`
- [x] Giữ tương thích crate import cũ bằng `lib.name` cho các package đã rename nơi cần thiết.
- [x] Bóc thêm batch color/char/input foundation crates:
  1. `wezterm-color-types`
  2. `wezterm-char-props`
  3. `wezterm-input-types`
- [x] Verify các crate feature-gated theo cấu hình đúng nơi cần thiết (`chatminal-wezterm-color-types --features std,use_serde`).
- [x] Bóc thêm batch terminal data-path crates:
  1. `wezterm-escape-parser`
  2. `wezterm-cell`
- [x] Sửa feature propagation `chatminal-wezterm-cell/use_serde -> wezterm-escape-parser/use_serde`.
- [x] Bóc thêm batch runtime utility crates:
  1. `wezterm-uds`
  2. `wezterm-toast-notification`
  3. `wezterm-dynamic`
- [x] Bóc thêm batch PTY/surface crates:
  1. `portable-pty`
  2. `wezterm-surface`
- [x] Verify batch PTY/surface:
  - `cargo test -p chatminal-portable-pty`
  - `cargo test -p chatminal-wezterm-surface --features std,appdata`
- [x] Ghi nhận `wezterm-ssh` đã first-party và verify package-level sạch:
  - `cargo test -p chatminal-wezterm-ssh`
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
  - `cargo test --manifest-path apps/chatminald/Cargo.toml`
  - `cargo check -p chatminal-wezterm-gui` vẫn chỉ dừng ở blocker native host `xcb-util.pc`
- [x] Bóc `wezterm-term` sang first-party:
  - copy crate sang `crates/chatminal-wezterm-term`
  - đổi `package.name` và giữ `lib.name = "wezterm_term"` để tương thích import cũ
  - remap ở root `Cargo.toml` và `third_party/wezterm/Cargo.toml`
  - mang `termwiz/data/wezterm` sang `crates/chatminal-wezterm-term/data/wezterm`
  - cập nhật `include_bytes!` để không còn phụ thuộc layout subtree cũ
- [x] Verify `wezterm-term`:
  - `cargo check -p chatminal-wezterm-term`
  - `cargo test -p chatminal-wezterm-term`
- [x] Bóc `config` sang first-party:
  - copy crate sang `crates/chatminal-config`
  - đổi `package.name` và giữ `lib.name = "config"` để tương thích import cũ
  - remap ở root `Cargo.toml` và `third_party/wezterm/Cargo.toml`
- [x] Verify `config`:
  - `cargo check -p chatminal-config`
  - `cargo test -p chatminal-config`
- [x] Bóc `mux` sang first-party:
  - copy crate sang `crates/chatminal-mux`
  - đổi `package.name` và giữ `lib.name = "mux"` để tương thích import cũ
  - remap ở root `Cargo.toml` và `third_party/wezterm/Cargo.toml`
  - bật `chrono/clock` cục bộ trong crate để giữ `Utc::now()` hoạt động dưới root workspace feature set hiện tại
- [x] Verify `mux`:
  - `cargo check -p chatminal-mux`
  - `cargo test -p chatminal-mux`
- [x] Bóc `termwiz` sang first-party:
  - copy crate sang `crates/chatminal-termwiz`
  - đổi `package.name` và giữ `lib.name = "termwiz"` để tương thích import cũ
  - remap ở root `Cargo.toml` và `third_party/wezterm/Cargo.toml`
- [x] Verify `termwiz`:
  - `cargo check -p chatminal-termwiz`
  - `cargo test -p chatminal-termwiz`
- [x] Dọn shadow manifests/subtree copies còn sót trong `crates/`:
  - `chatminal-mux/mux`
  - `chatminal-config/derive`
  - `chatminal-filedescriptor/filedescriptor`
  - `chatminal-ratelim/ratelim`
  - `chatminal-wezterm-client/wezterm-client`
  - `chatminal-wezterm-gui-subcommands/wezterm-gui-subcommands`
- [x] Bóc `wezterm-mux-server-impl` sang first-party:
  - copy crate sang `crates/chatminal-wezterm-mux-server-impl`
  - đổi `package.name` và giữ `lib.name = "wezterm_mux_server_impl"` để tương thích import cũ
  - remap ở root `Cargo.toml` và `third_party/wezterm/Cargo.toml`
- [x] Verify `wezterm-mux-server-impl`:
  - `cargo check -p chatminal-wezterm-mux-server-impl`
  - `cargo test -p chatminal-wezterm-mux-server-impl`
- [x] Bóc `window` sang first-party:
  - copy crate sang `crates/chatminal-window`
  - đổi `package.name` và giữ `lib.name = "window"` để tương thích import cũ
  - remap ở root `Cargo.toml` và `third_party/wezterm/Cargo.toml`
- [x] Verify `window` theo downstream graph:
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
  - `cargo test --manifest-path apps/chatminald/Cargo.toml`
  - `cargo check -p chatminal-wezterm-gui` vẫn chỉ dừng ở `xcb-util.pc`
- [x] Enforce reference-only guard:
  - thêm `scripts/verify-third-party-wezterm-reference-only.sh`
  - thêm `make verify-third-party-reference-only`
  - đưa guard này vào `make check`
- [x] Nếu verify không phát sinh blocker graph mới, bóc wave kế tiếp theo thứ tự:
  1. dọn tiếp metadata/tài liệu thừa trong `third_party/wezterm` nếu cần
  2. bỏ dependency/feature dormant `xcb-imdkit` khỏi nhánh mặc định của active workspace
- [x] Với mỗi crate trong wave trên:
  - copy sang `crates/chatminal-*`
  - đổi `package.name`
  - thêm vào root workspace `members`
  - remap ở root `Cargo.toml`
  - remap ở `third_party/wezterm/Cargo.toml`
  - chỉ update docs sau khi cả wave verify xong
- [x] Nếu wave pass, update ngắn:
  - `README.md`
  - `docs/development-roadmap.md`
  - `docs/project-changelog.md`

### Success Criteria
- `chatminal-termwiz-funcs` không còn hardcode path theo subtree cũ.
- `cargo metadata`/`cargo check -p chatminal-wezterm-gui` không fail vì path crate/data cũ.
- Các crate parser/foundation mới bóc vẫn pass test dưới crate name tương thích cũ.
- Các crate color/char/input mới bóc vẫn pass test dưới crate name tương thích cũ.
- Các crate terminal data-path/runtime-utility mới bóc vẫn pass test dưới crate name tương thích cũ.
- Các crate `portable-pty`, `wezterm-surface`, `wezterm-ssh` chạy verify package-level sạch dưới package first-party.
- `chatminal-wezterm-term` chạy verify package-level sạch và không còn phụ thuộc `include_bytes!` theo relative path cũ.
- `chatminal-config` chạy verify package-level sạch và không còn phụ thuộc crate path từ `third_party/wezterm/config`.
- `chatminal-mux` chạy verify package-level sạch dưới package first-party.
- `chatminal-termwiz` chạy verify package-level sạch dưới package first-party.
- `chatminal-wezterm-mux-server-impl` chạy verify package-level sạch dưới package first-party.
- `window` đã chạy qua downstream graph first-party và root workspace không còn path dep trực tiếp vào `third_party/wezterm/*`.
- Có guard tự động fail nếu active build/runtime quay lại dùng `third_party/wezterm` trực tiếp.
- Không còn shadow `Cargo.toml` lồng trong `crates/` gây workspace mismatch/IDE footgun.
- `xcb-imdkit` không còn tồn tại trong manifest/lockfile active của root workspace.
- Nếu còn fail build GUI ở wave sau, blocker phải là issue mới thật sự của batch đó, không phải graph/path mismatch hay asset path sót từ layout `third_party/wezterm`.

### Risk
- `third_party/wezterm` vẫn còn giá trị tham chiếu source cho việc đối chiếu upstream; nếu cleanup quá mạnh tay dễ làm mất mốc so sánh cho các wave refactor sau.
- Guard reference-only hiện chỉ khóa active build/runtime surfaces; nó không chặn các tài liệu lịch sử hay file note vẫn nhắc tới `third_party/wezterm`.
- `wezterm-color-types` có API test phụ thuộc feature `std`; verify package-level về sau cần dùng đúng feature set để tránh false-alarm.
- `wezterm-cell` có đường serde phụ thuộc feature từ `wezterm-escape-parser`; cần giữ propagation này khi refactor tiếp để tránh regression ẩn.
- `wezterm-surface` có test snapshot gắn với debug output hiện tại của `Line`; nếu upstream/stdlib đổi formatter, snapshot có thể lệch dù runtime logic không gãy.
- `config` có nhiều module lớn và dùng `mlua`/filesystem watcher; dù package-level test đã pass, các wave kế tiếp vẫn nên giữ regression app/daemon sau mỗi lần remap.
- `mux` hiện cần `chrono/clock` cục bộ vì root workspace đang dùng `chrono` không có default features; nếu crate khác bắt đầu dùng `Utc::now()` thì có thể lặp lại pattern này.
- Build mặc định của Chatminal hiện không có X11 IME path; nếu cần khôi phục về sau thì phải re-introduce dependency/implementation có chủ đích, thay vì trông vào dormant feature cũ.

### Next Step
- Wave này đã hoàn tất: graph/path, asset path, lockfile active, smoke, và test downstream đều sạch trên host Linux dev; phần còn lại chỉ là theo dõi CI matrix và manual host-specific preflight ngoài coding wave này.
