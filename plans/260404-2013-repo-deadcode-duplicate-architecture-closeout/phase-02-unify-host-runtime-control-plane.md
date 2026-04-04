---
phase: 02
status: completed
priority: high
effort: medium
risk: medium
---

# Phase 02: Unify Host Runtime Control Plane

## Overview
Chốt `HostRuntimeHandle` làm public control-plane canonical trong `chatminal-host-runtime`, rồi xử lý luôn các seam phụ cùng boundary: free-function wrappers dư, `compat_default()` family, và orphan public wrappers.

## Findings Covered
- Finding 2: dual public control-plane API
- Finding 5: `compat_default()` family semantically empty
- Finding 6: orphan public wrappers `register_runtime_client(...)` / `replace_active_identity(...)`

## Scope
- `crates/chatminal-host-runtime/src/lib.rs`
- `crates/chatminal-host-runtime/src/spawn_target.rs`
- `crates/chatminal-host-runtime/src/pty_io.rs`
- `crates/chatminal-host-runtime/src/localpane_hooks.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- tests liên quan host runtime bootstrap/control plane

## Requirements
- Caller mới phải có đúng một public style để register client, replace identity, subscribe notifications, set active workspace
- Free-function surface nếu còn giữ chỉ được là private/test-only compat seam hoặc deprecated transitional shim có removal plan rõ
- `compat_default()` chỉ được giữ nếu semantic khác `host_default()`; nếu giống hệt thì phải xóa hoặc hạ scope

## Architecture
- `HostRuntimeHandle` là owner-facing API cho bootstrap/control metadata
- `HostRuntimeRoot` vẫn là internal owner
- Desktop bootstrap/subscription path bám cùng một canonical handle/boundary story
- Hooks defaults phân biệt theo behavior thật, không phân biệt theo naming lịch sử

## Related Code Files
- Modify:
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
  - `crates/chatminal-host-runtime/src/pty_io.rs`
  - `crates/chatminal-host-runtime/src/localpane_hooks.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Delete:
  - wrappers public không còn caller/không còn semantic value

## Implementation Steps
1. Audit toàn bộ caller của `HostRuntimeHandle` methods và free-function wrappers.
2. Chuyển desktop subscription path sang canonical control-plane style giống bootstrap path.
3. Thu hẹp hoặc xóa `register_runtime_client(...)` và `replace_active_identity(...)` nếu product path không còn dùng.
4. Quyết định fate của `subscribe_runtime_notifications(...)`: giữ dưới canonical handle, hay explicit compat seam hẹp có documentation rõ.
5. Audit `compat_default()` family; nếu không khác `host_default()`, xóa hoặc hạ scope về test-only helper.
6. Cập nhật tests để bám boundary mới, không assert song song hai API styles cho cùng contract.

## Todo List
- [x] Audit caller map cho handle methods vs free functions
- [x] Cut desktop bridge/subscription sang canonical path
- [x] Remove or contain orphan public wrappers
- [x] Remove or contain semantically-empty `compat_default()` helpers
- [x] Update host-runtime tests theo boundary mới

## Success Criteria
- `chatminal-host-runtime` còn một public control-plane story canonical
- Product path không còn vừa bootstrap bằng handle vừa subscribe/identity bằng free-function API cũ
- `compat_default()` không còn là duplicate API surface vô nghĩa
- `register_runtime_client(...)` và `replace_active_identity(...)` không còn public debt vô chủ

## Risk Assessment
- Risk: gãy tests/compat callers ngầm ngoài product path
- Mitigation: grep callers toàn workspace trước; compat nếu cần thì hạ scope rõ ràng thay vì giữ public chung

## Security Considerations
- Control-plane cleanup không đổi auth model, nhưng tránh split-brain ownership khi future callers chọn nhầm API style

## Next Steps
- Sau phase này docs ownership/control-plane mới đủ ổn để sync ở phase 04
