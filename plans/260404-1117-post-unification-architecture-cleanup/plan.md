---
title: "Post-Unification Architecture Cleanup"
description: "Pragmatic next-wave cleanup plan focused on high-ROI compat, history, and active-doc debt after architecture unification."
status: pending
priority: P2
effort: 3-5d
branch: main
tags: [architecture, cleanup, compatibility, runtime, docs]
created: 2026-04-04
---

# Post-Unification Architecture Cleanup

## Goal
Làm sạch kiến trúc sau unification theo hướng practical: giảm compatibility seam còn ảnh hưởng active product path, bỏ duplicate/deadcode có ROI thật, và tránh các refactor churn-heavy không đổi behavior.

## Ground Truth
- `260401-0949-architecture-unification`: done
- `260403-1800-post-unification-followups`: done
- Product path hiện là `single-runtime desktop`
- Debt còn lại tập trung ở compatibility/private zones, không còn ở top-level architecture

## Direct Answers To Remaining Debt
1. `Mux`/compat facade trong `chatminal-host-runtime`:
   - `must-do`: cắt thêm ở active product path
   - `not goal`: xóa sạch mọi dấu vết `Mux` nếu private compat/test path vẫn cần ngắn hạn
2. Lua bridge compat path:
   - `must-do`: audit và cắt các bridge path chỉ còn phục vụ compat, nhất là path còn kéo root-window/mux semantics không còn cần cho product
3. `desktop_commands.rs` compatibility translation:
   - `must-do`: thu hẹp về đúng subset còn support; không giữ translation table rộng hơn product contract
4. Duplicate terminal layer `chatminal-terminal-core` vs `chatminal-terminal-emulator`:
   - `do-later`: chưa merge/unify ở wave này; cần decision freeze rõ ràng thay vì refactor lớn lợi ích thấp
5. Legacy/canonical dual-read history path:
   - `must-do`: steady-state runtime phải đọc canonical-only; legacy read chỉ được tồn tại như migration helper hữu hạn
6. `engine_*` lib names / Cargo aliases:
   - `do-later`: dừng ở package/path rename; chưa rename tiếp `lib.name` + aliases vì churn lớn, ROI thấp hiện tại
7. Docs/roadmap/archive:
   - `should-do`: chỉ dọn docs active scope; không rewrite archive/history docs nếu không gây hiểu nhầm active reality

## Priority Buckets

### Must-Do
- [Phase 01](./phase-01-collapse-product-compat-seams.md): collapse product-path compat seams
- [Phase 02](./phase-02-trim-lua-and-command-compat.md): trim Lua bridge + command compatibility surface
- [Phase 03](./phase-03-cut-history-dual-read.md): cut legacy/canonical dual-read in steady-state runtime

### Should-Do
- [Phase 04](./phase-04-prune-active-docs-and-deadcode.md): prune active docs + dead helpers unlocked by phases 01-03

### Do-Later / Not In This Wave
- Không unify `chatminal-terminal-core` và `chatminal-terminal-emulator` ở wave này
- Không rename tiếp `engine_*` lib names / Cargo aliases ở wave này
- Không rewrite plan archive / changelog history chỉ để đổi vocabulary

## Why This Order
1. `Mux`/compat seam trong host-runtime và desktop adapter đang là nguồn deadcode/duplicate kiến trúc rõ nhất.
2. Lua bridge và command translation là public-ish compatibility surface; nếu không siết sớm, cleanup bên dưới sẽ bị chặn.
3. History dual-read là duplicate architecture có cost dài hạn trong runtime/store/tests.
4. Docs/deadcode cleanup chỉ nên làm sau khi behavior/contract đã chốt.

## Success Criteria
- Active desktop product path không còn phụ thuộc vào `legacy_*` host wrappers hoặc `mux_default()` mặc định
- Lua bridge và command compatibility chỉ còn phần thật sự support
- Runtime steady-state không còn dual-read canonical + legacy scrollback
- Active docs phản ánh đúng reality; archive/history được giữ nguyên khi hợp lý
- `cargo check --workspace` và `cargo test --workspace --lib --bins --tests` xanh sau mỗi phase lớn

## Non-Goals
- Không đổi UI/UX feature surface
- Không cosmetic-sweep thêm cho vocabulary nếu không giảm coupling thật
- Không merge 2 terminal implementation chỉ vì “nhìn duplicate” khi contract/owner vẫn khác

## Verification Spine
- `cargo check --workspace`
- `cargo test --workspace --lib --bins --tests`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- targeted greps cho `legacy_*`, `mux_default()`, Lua compat entry points, history dual-read callsites
