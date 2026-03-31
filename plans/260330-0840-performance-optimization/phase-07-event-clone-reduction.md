# Phase 07: Event Processor Clone Reduction

## Overview
- **Priority:** LOW — CPU micro-optimization
- **Status:** pending
- **Effort:** Medium (~2-3 hours)

## Problem

`session_event_processor.rs` still clones `session_id: String` nhiều lần per PTY output event:
- clone để build runtime event broadcast
- clone để persist store updates / error paths
- clone để fanout subscribers

`output_chunk` cũng bị clone vài lần trên path này.

## Key Insights

- `session_id` là immutable once created → hợp với `Arc<str>`
- `SessionEvent` là hot transport object, nên đổi type ở đây có giá trị
- `StateInner.sessions` không bắt buộc phải đổi sang `Arc<str>` ở first pass; dùng borrowed lookup với `&str` là đủ
- Desktop execution bridge cũng đang map events bằng `String`, nên phase này phải tính cả desktop bridge
- `chunk` phức tạp hơn vì còn bị xử lý/persist theo nhiều nhánh; nên giữ optional

## Related Code Files

### Modify
- `crates/chatminal-runtime/src/state/session_event_processor.rs`
- `crates/chatminal-runtime/src/state.rs`
- `crates/chatminal-runtime/src/session.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`

### Read-only references
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-runtime/src/state/runtime_lifecycle.rs`

## Implementation Steps

### Step 1: Change `SessionEvent.session_id` to `Arc<str>`

```rust
pub enum SessionEvent {
    Output { session_id: Arc<str>, ... },
    Exited { session_id: Arc<str>, ... },
    Error  { session_id: Arc<str>, ... },
}
```

### Step 2: Update sender side to create `Arc<str>` once

Per thread:
```rust
let session_id: Arc<str> = session_id.into();
```

Per event:
```rust
session_id: Arc::clone(&session_id)
```

### Step 3: Keep `StateInner.sessions` keyed by `String`

Do not widen scope to storage-key refactor yet:
```rust
inner.sessions.get_mut(session_id.as_ref())
inner.sessions.get(session_id.as_ref())
```

### Step 4: Update event processor clone sites

Replace `session_id.clone()` with `Arc::clone(&session_id)` anywhere ownership is required.

### Step 5: Update desktop execution bridge mapping

When desktop bridge maps `SessionRuntimeEvent` into `chatminal_runtime::SessionEvent`, convert into `Arc<str>` there too.

### Step 6: Re-evaluate `chunk` separately

Only consider `Arc<str>`/`Bytes` for chunk after measuring clone cost post-`session_id` fix.

## Todo

- [ ] Change `SessionEvent` variants: `session_id: String` → `Arc<str>`
- [ ] Update sender side to create `Arc<str>` once per thread
- [ ] Keep `StateInner.sessions` keyed by `String` in first pass
- [ ] Update event processor clone sites to `Arc::clone()`
- [ ] Update desktop execution bridge mapping for new `SessionEvent` type
- [ ] Re-evaluate chunk optimization only after measurement

## Success Criteria

- `cargo test -p chatminal-runtime` — all tests pass
- `cargo check --workspace` — 0 errors
- String heap allocations for `session_id` on hot PTY event path are eliminated
- Existing session create/delete/switch behavior unchanged

## Risk Assessment

- **Medium risk:** `Arc<str>` propagation touches runtime + desktop bridge files. Mitigation: compiler-driven updates.
- **Low risk:** Keeping `StateInner.sessions` keyed by `String` avoids a broader refactor with limited incremental value.
- **Low risk:** `Arc` overhead is negligible compared to String heap allocation.

## Verify

```bash
cargo test -p chatminal-runtime
cargo check --workspace
# Optional: profile hot event path before/after
```
