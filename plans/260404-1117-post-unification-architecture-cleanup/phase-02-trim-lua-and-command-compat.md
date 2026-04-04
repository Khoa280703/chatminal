---
phase: 02
status: pending
priority: high
effort: medium
risk: medium
---

# Phase 02: Trim Lua And Command Compat

## Overview
Thu hẹp compatibility surface ở Lua bridge và desktop command translation về đúng contract product còn support.

## Why This Phase Exists
- Lua bridge vẫn còn các path/root helpers mang tính compat sâu.
- `desktop_commands.rs` vẫn là translation layer rộng hơn behavior product hiện tại.
- Nếu không siết, Phase 01 sẽ luôn bị kéo ngược về compat semantics.

## Scope
- `crates/chatminal-lua-bridge/src/lib.rs`
- `crates/chatminal-lua-bridge/src/window.rs`
- `crates/chatminal-lua-bridge/src/session.rs`
- `crates/chatminal-lua-bridge/src/leaf.rs`
- `apps/chatminal-desktop/src/desktop_commands.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- config/command tests liên quan

## Requirements
- Public bridge/command surface phải match đúng feature product đang support.
- Backward-compat nếu giữ thì phải ở parse/load boundary, không lan vào runtime dispatch logic.

## Implementation Steps
1. Audit public Lua methods và classify: active, compat-but-needed, compat-deletable.
2. Xóa hoặc deprecate các bridge path chỉ còn phục vụ `Mux`/root-window compat mà product không cần.
3. Thu hẹp `desktop_commands.rs` về subset command còn support trong session UI/product path.
4. Nếu cần compat config cũ, normalize ở config-load/translation seam thay vì giữ runtime-wide translation table.
5. Viết tests cho mixed old-config / current-product behavior để biết chính xác cái gì còn support.

## Done Criteria
- Lua bridge không còn public compat API chỉ để kéo `Mux`/root-window semantics cũ nếu product không dùng.
- `desktop_commands.rs` không còn translation cho các action product đã bỏ support.
- Hành vi backward-compat còn lại được cô lập và có test chứng minh.

## Risk / Tradeoff
- Risk: config/Lua script cũ có thể gãy nếu xóa mù.
- Tradeoff: chấp nhận giữ một lớp normalize hẹp ở load boundary; không giữ runtime compatibility vô hạn.
