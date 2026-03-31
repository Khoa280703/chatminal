# Phase 06: Workspace Dependency Cleanup

## Overview
- **Priority:** LOW — binary size + compile time
- **Status:** pending
- **Effort:** Low (~1-2 hours)

## Problem

Workspace và desktop manifest còn giữ các deps có khả năng dư thừa hoặc quá nặng:
- `reqwest` — hiện chỉ thấy ở workspace metadata
- `tokio` — hiện cũng chỉ mang tính workspace availability
- `openssl` / `chatminal-async-ossl` còn xuất hiện ở desktop Cargo.toml
- `http_req` cần verify còn dùng không

## Key Insights

- Workspace dep removal chỉ an toàn khi member manifests không còn reference
- `apps/chatminal-desktop/Cargo.toml` currently references both `async_ossl` and `openssl`
- Vì vậy cleanup phải audit direct desktop deps trước khi đụng đến crate deletion
- `cargo udeps` và `cargo tree -i` sẽ cho tín hiệu đáng tin hơn suy đoán

## Related Code Files

### Modify
- `Cargo.toml` — workspace deps
- `apps/chatminal-desktop/Cargo.toml` — direct desktop deps
- `crates/chatminal-async-ossl/Cargo.toml` — only if crate still survives

### Read-only references
- `crates/chatminal-async-ossl/src/lib.rs` — wrapper crate content

## Implementation Steps

### Step 1: Run authoritative dep audit

```bash
cargo install cargo-udeps
cargo +nightly udeps --workspace
cargo tree -p chatminal-desktop -i openssl
rg -n "async_ossl|openssl|reqwest|tokio|http_req" . -g 'Cargo.toml'
```

### Step 2: Remove obvious workspace-only metadata deps if truly unused

Candidate first:
- `reqwest`

Only after verifying no member manifest still points at it.

### Step 3: Audit desktop direct deps on `async_ossl` and `openssl`

If code references are truly zero:
1. remove `async_ossl` and `openssl` from `apps/chatminal-desktop/Cargo.toml`
2. run `cargo check -p chatminal-desktop`
3. then evaluate deleting `crates/chatminal-async-ossl/`

### Step 4: Re-evaluate OpenSSL via git2 separately

If OpenSSL remains only because of `git2`, then treat that as a separate migration task. Do not mix “delete unused dep” with “change TLS backend behavior” in one blind step.

### Step 5: Audit `http_req`

Remove only if both Rust code and manifests confirm it is dead.

## Todo

- [ ] Run `cargo udeps` for authoritative unused dep list
- [ ] Remove `reqwest` from workspace deps if truly unused
- [ ] Audit desktop direct deps on `async_ossl` and `openssl`
- [ ] Remove unused desktop direct deps before touching crate deletion
- [ ] Evaluate and potentially remove `chatminal-async-ossl` crate
- [ ] Evaluate `http_req`
- [ ] Verify `tokio` remains workspace metadata only
- [ ] Measure binary size before/after

## Success Criteria

- `cargo check --workspace` — 0 errors
- `cargo test --workspace --lib --bins --tests` — all pass
- Any removed dependency is proven unused, not assumed unused
- Binary size and/or compile graph gets smaller

## Risk Assessment

- **Low risk:** Removing truly unused deps is compiler-checked.
- **Medium risk:** OpenSSL may still be needed indirectly; do not conflate direct-dep cleanup with TLS backend migration.

## Verify

```bash
cargo check --workspace
cargo test --workspace --lib --bins --tests
# Compare binary size before/after if release build is available
```
