---
phase: 01
status: done
priority: medium
effort: large
risk: medium
---

# Phase 01: Config Ownership Completion

## Scope
- `260401-0949-architecture-unification` Phase 04 Step 3
- `260401-0949-architecture-unification` Phase 04 Step 4

## Goal
- Tách `Config` thành section rõ ràng nếu vẫn còn giá trị kiến trúc.
- Thay `configuration()` singleton reads còn lại bằng propagation/config ownership explicit end-to-end.

## Key Files
- `crates/chatminal-config/src/*`
- `crates/chatminal-terminal-font/src/*`
- `crates/chatminal-ratelim/src/*`
- `crates/chatminal-time-funcs/src/*`
- `crates/chatminal-window/src/*`
- `crates/chatminal-host-runtime/src/*`
- `crates/chatminal-lua-bridge/src/*`
- `apps/chatminal-desktop/src/*`

## Reality Check
- Scope thực tế rộng hơn plan closeout cũ: `configuration()` product/foundation reads còn nằm ở `chatminal-terminal-font`, `chatminal-window`, `chatminal-ratelim`, `chatminal-time-funcs`.
- `apps/chatminal-desktop/src/shapecache.rs` hiện chỉ còn test code.
- `crates/chatminal-config/src/*` vẫn giữ singleton compatibility surface; phase này tập trung cắt product/background paths trước, không bắt buộc xóa toàn bộ compat API.
- Code reality đã được sync: product-path `configuration()` reads đã được kéo về explicit ownership ở các crate liên quan.

## Parallel Lanes
1. `engine-font`
   - bỏ singleton reads trong FreeType/Harfbuzz path
   - ưu tiên reuse `ConfigHandle` đã có sẵn trong `FontConfiguration`
2. `window`
   - inject `ConfigHandle` vào connection/window/frame state
   - bỏ singleton reads trong platform window bootstrap và IME/window decoration branches
3. `background helpers`
   - `chatminal-ratelim` và `chatminal-time-funcs`
   - chỉ nhận explicit provider/handle, không tự chạm singleton trong hot path mới
4. `desktop test cleanup`
   - dọn `shapecache.rs` test singleton reads và comment stale references

## Exit Criteria Clarification
- `configuration()` có thể còn trong:
  - `chatminal-config` compat layer
  - tests
  - comments/docs
- Không còn singleton read trong runtime/product/foundation execution path của desktop app.

## Done Criteria
- runtime/product paths không còn phụ thuộc `configuration()` singleton
- propagation xuống PTY/parser path được inject rõ ràng
- build/test/smoke xanh toàn workspace
