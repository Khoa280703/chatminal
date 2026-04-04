# Remaining 4-Way Completion Split

## Goal
Chia **phần còn lại** của `260401-0949-architecture-unification` thành 4 stream đủ độc lập để đẩy nhanh nhất có thể, rồi ghép ở một merge wave cuối.

## Why This Split
- Phần còn lại không còn phù hợp để chia theo phase lớn.
- Nếu chia theo ownership/file cluster, 4 người có thể làm song song thật.
- Chỗ còn phải ghép nhau chỉ còn ở merge wave cuối, không nên bắt từng employee tự xử lý integration cross-cutting.

## Hard Rule
- Không overlap file giữa 4 employee.
- Không stream nào được yêu cầu stream khác mở API trước rồi mới làm.
- Nếu cần compat helper, helper đó phải nằm trong ownership của chính stream đó.
- Lead giữ quyền merge/final cleanup/docs sync cuối.

## Stream A: Host-Runtime Mutation/Lifecycle Cut
- Goal:
  - cắt tiếp mutation/lifecycle helpers còn bám compat `Mux` facade
  - đẩy ownership thật sâu hơn vào `HostRuntimeRoot`
- Ownership:
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `crates/chatminal-host-runtime/src/tab.rs`
  - `crates/chatminal-host-runtime/src/window.rs`
  - `crates/chatminal-host-runtime/src/pane.rs`
- Main tasks:
  - dọn các helper `add/remove/focus/spawn/split` còn dựng qua `Mux`
  - giảm tiếp compat wrapper names nếu không phá caller hiện tại
  - giữ `Mux` chỉ như compat shell thật mỏng
- Not allowed:
  - không sửa `pty_io.rs`
  - không sửa `localpane*.rs`
  - không sửa `spawn_target.rs`
  - không sửa desktop/lua callsites
- Done when:
  - host-runtime compile/test xanh
  - mutation/lifecycle cluster trong ownership này mỏng hơn rõ rệt
  - không mở thêm public raw-id leak mới

## Stream B: PTY Default Owner Final Cut
- Goal:
  - đóng triệt để `mux_default()`/default cleanup owner ở compat PTY path
  - làm rõ owner của output/cleanup/inline-error/localpane side effects
- Ownership:
  - `crates/chatminal-host-runtime/src/pty_io.rs`
  - `crates/chatminal-host-runtime/src/localpane.rs`
  - `crates/chatminal-host-runtime/src/localpane_hooks.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
- Main tasks:
  - chuyển default owner khỏi Mux semantics ở PTY/localpane/spawn boundary
  - làm explicit bundle owner cho fallback path
  - giữ backward behavior bằng explicit compat seams, không bằng hidden fallback
- Not allowed:
  - không sửa `lib.rs`
  - không sửa desktop shell/UI
  - không sửa lua-bridge
- Done when:
  - host-runtime tests xanh
  - grep/inspection cho thấy default owner không còn ẩn sau `mux_default()` ở seam mục tiêu
  - residual Mux dependency ở PTY path chỉ còn nếu được gọi explicit

## Stream C: Config Final Integration
- Goal:
  - giảm tiếp `configuration()` singleton reads trên path mục tiêu của plan
  - hoàn tất phần config foundation/propagation còn lại
- Ownership:
  - `crates/chatminal-config/src/*`
  - `apps/chatminal-desktop/src/frontend.rs`
  - `apps/chatminal-desktop/src/main.rs`
  - `apps/chatminal-desktop/src/stats.rs`
  - `apps/chatminal-desktop/src/desktop_spawn.rs`
  - `apps/chatminal-desktop/src/desktop_commands.rs`
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Main tasks:
  - quét các path còn singleton config read trong phạm vi ownership này
  - đẩy sang config snapshot / explicit handle / propagated state
  - chốt helper foundation còn thiếu ở `chatminal-config`
- Not allowed:
  - không sửa `host-runtime/src/lib.rs`
  - không sửa `pty_io.rs`, `localpane*.rs`, `spawn_target.rs`
  - không sửa lua-bridge
- Done when:
  - `configuration()` trong ownership này giảm tiếp rõ rệt
  - desktop compile/test xanh
  - không sinh thêm config flow vòng vo

## Stream D: Lua Bridge + Final Public Boundary Cleanup
- Goal:
  - chốt phần bridge/public boundary còn lại để Phase 03/04 không bị treo bởi compat leak
- Ownership:
  - `crates/chatminal-lua-bridge/src/lib.rs`
  - `crates/chatminal-lua-bridge/src/window.rs`
  - `crates/chatminal-lua-bridge/src/session.rs`
  - `crates/chatminal-lua-bridge/src/leaf.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- Main tasks:
  - dọn tiếp bridge compat path còn dựa vào concrete host primitives rộng hơn cần thiết
  - thu hẹp cross-crate/public boundary còn lại ở desktop-host/lua slices này
  - dùng helper hiện có; nếu thiếu thì localize compat logic trong ownership
- Not allowed:
  - không sửa `host-runtime/src/lib.rs`
  - không sửa `pty_io.rs` / `localpane*.rs` / `spawn_target.rs`
  - không sửa `termwindow/*` rộng hơn ngoài 2 file ownership trên
- Done when:
  - lua-bridge compile/test xanh
  - desktop compile/test xanh
  - bridge/session_host không mở thêm phụ thuộc rộng vào host internals

## Merge Wave: Lead Only
- Ownership:
  - docs
  - plan sync
  - final grep sweep
  - cross-stream conflict resolution
- Tasks:
  - merge 4 streams
  - chạy full verify
  - absorb integration backlog còn lại
  - chạy rename/cosmetic sweep cuối nếu chưa xong
  - sync:
    - `plan.md`
    - `phase-03-kill-mux-singleton.md`
    - `merge-checklist.md`
    - `docs/system-architecture.md`
    - `docs/codebase-summary.md`
    - `docs/project-changelog.md`

## Recommended Merge Order
1. Stream D
2. Stream C
3. Stream B
4. Stream A
5. Merge wave

## Why This Order
- `D` và `C` ít rủi ro ownership lõi hơn, dễ merge trước.
- `B` có rủi ro PTY lifecycle cao hơn, nên merge sau khi caller-side đã yên.
- `A` chạm lõi host-runtime nhiều nhất, nên merge cuối trong 4 stream để resolve theo trạng thái mới nhất.

## Fastest Staffing Advice
- Nếu chỉ có 2 người:
  - Person 1: A
  - Person 2: B+C+D
- Nếu có 3 người:
  - Person 1: A
  - Person 2: B
  - Person 3: C+D
- Nếu có 4 người:
  - mỗi người 1 stream
- Nếu có hơn 4 người:
  - vẫn chỉ nên có 4 implementation streams
  - người dư làm tester/reviewer/merge support, không mở stream code thứ 5

## Done Condition For This Split
- 4 streams đều xong trong ownership của mình
- merge wave absorb xong phần cross-cutting
- full verify xanh
- khi đó plan tổng mới có thể gọi là `100%`
