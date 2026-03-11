# Phase 07 - Dependency Graph Prune And Hard Cleanup

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/Cargo.toml
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/Cargo.toml
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/mod.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/main.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/update.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/spawn.rs

## Overview
- Priority: P1
- Current status: pending
- Brief: sau khi active path đã bỏ adapter/mux host, dọn dependency graph, wiring khởi động và code chết còn sót để repo phản ánh đúng kiến trúc mới, nhưng không overclaim việc bỏ toàn bộ `mux` khỏi desktop compile graph nếu render boundary còn cần nó.

## Key Insights
- Nếu không prune graph, repo vẫn tiếp tục gửi tín hiệu sai rằng `mux` là execution dependency của desktop
- Cleanup phase này chỉ nên xóa thứ thật sự không còn ai gọi
- Một số `mux` usage ngoài active path có thể là engine-private hoặc compatibility; cần phân tách rõ, không xóa mù

## Requirements
- Functional: build graph active desktop không cần `chatminal-mux` làm session execution host; compile dependency còn lại phải được giải thích rõ là render/compat boundary nếu chưa bỏ được
- Non-functional: không làm gãy compatibility crates không liên quan đến `make window`

## Architecture
- Desktop active session flow chỉ phụ thuộc execution path mới; dependency `mux` nếu còn phải bị thu hẹp rõ về render/notification compatibility
- Mọi helper startup/update/spawn dùng session runtime wiring thật
- Code chết do host-tab bridge bị loại bỏ hoàn toàn

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/Cargo.toml
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/Cargo.toml
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/mod.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/main.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/update.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/spawn.rs
- Delete: mọi file/helper chỉ còn phục vụ host-tab bridge nếu thật sự dead

## Implementation Steps
1. Rà dependency graph của desktop và session-runtime sau Phase 06
2. Xóa imports, helper, type alias, state cache chết
3. Gỡ dependency `chatminal-mux` khỏi session execution path; chỉ gỡ khỏi compile graph desktop nếu thật sự không còn cần ở render boundary
4. Giữ compatibility path ở crate riêng nếu bắt buộc
5. Chạy `cargo check --workspace` và grep dead code sau cleanup

## Todo List
- [ ] Dependency graph active desktop phản ánh session-native execution path
- [ ] Code chết của host-tab bridge bị xóa
- [ ] Startup/update/spawn wiring sạch lại
- [ ] Không còn dependency thừa trong `Cargo.toml`

## Success Criteria
- Active desktop runtime path không còn cần `chatminal-mux` làm session host
- `Cargo.toml` và startup wiring phù hợp kiến trúc mới; mọi `mux` dependency còn sót đều có lý do compatibility rõ ràng
- Code review grep cho host bridge trả về zero hoặc chỉ còn compat slice rõ ràng

## Risk Assessment
- Risk: compile graph vẫn kéo `chatminal-mux` qua dependency gián tiếp khó thấy
- Mitigation: xác minh bằng `cargo tree` sau khi cleanup

## Security Considerations
- Không đổi data path; chỉ dọn dependency và dead code

## Next Steps
- Phase 08 sẽ chạy verify cuối và dọn dữ liệu/app state nếu cần cho dev iteration mới
