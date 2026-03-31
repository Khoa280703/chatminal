# Phase 03: Session Thread Reduction

## Overview
- **Priority:** HIGH — scaling bottleneck
- **Status:** pending
- **Effort:** High (~4-5 hours)

## Problem

Production desktop path still runs 3 threads per active session:
1. reader loop
2. writer loop
3. waiter loop polling `try_wait()` every 120ms

The expensive part is the standalone waiter thread waking constantly while idle.

## Key Insights

- Production path is desktop leaf runtime in `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/**`
- `crates/chatminal-runtime/src/session.rs` is currently `#[cfg(test)]`; optimizing it alone gives no desktop benefit
- Reader thread is legitimate because it blocks on PTY read
- Writer path is acceptable
- `spawn_waiter_loop()` is pure polling overhead on idle desktop sessions
- Desktop leaf runtime currently does not carry `health_interval_ms`, so first pass should not assume config plumbing already exists there

## Related Code Files

### Modify
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime_threads.rs` — production reader/waiter loops
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime.rs` — spawn wiring if signatures change
- `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs` — only if event plumbing changes

### Read-only references
- `crates/chatminal-portable-pty/src/` — `MasterPty::as_raw_fd`
- `crates/chatminal-runtime/src/state.rs` — downstream consumer only

## Implementation Steps

### Step 1: Retarget the phase to production desktop code

Do not start in `crates/chatminal-runtime/src/session.rs`.

### Step 2: Merge desktop reader + waiter into one thread on supported unix path

Wrap the existing desktop reader body with readiness polling:
```rust
thread::spawn(move || {
    let poll_interval = Duration::from_millis(500);

    loop {
        match poll_read_with_timeout(raw_fd, poll_interval) {
            PollResult::Ready => {
                // existing reader logic:
                // - read chunk
                // - prompt de-dupe
                // - terminal.advance_bytes
                // - io_terminal.advance_bytes
                // - output_history.push
                // - send TerminalInstanceRuntimeEvent::Output
            }
            PollResult::Timeout => {}
            PollResult::Error(err) => break,
        }

        if let Ok(Some(status)) = child.try_wait() {
            // send exited event and break
        }
    }
});
```

### Step 3: Implement unix-only `poll_read_with_timeout`

Use `nix::poll` with `MasterPty::as_raw_fd()` when available.

Fallback:
- unix backend without raw fd: keep old waiter thread
- non-unix: keep old waiter thread

### Step 4: Raise desktop poll interval from 120ms to 500ms

This keeps exit detection under ~0.5s while cutting idle wakeups substantially.

### Step 5: Keep writer path unchanged

Do not widen scope unless profiling later proves writer contention.

## Todo

- [ ] Move Phase 03 implementation target to desktop leaf runtime files
- [ ] Create `PollResult` enum and unix `poll_read_with_timeout`
- [ ] Merge reader + waiter in `leaf_runtime_threads.rs`
- [ ] Preserve prompt de-dupe/history/terminal update behavior inside merged loop
- [ ] Raise desktop poll interval from 120ms to 500ms
- [ ] Keep old waiter path as fallback where polling is unavailable

## Success Criteria

- `cargo check -p chatminal-desktop` — 0 errors
- Desktop leaf runtime uses 2 threads per session on supported unix path
- Child exit detected within 500ms
- PTY output latency unchanged
- `ps -M <pid>` shows reduced desktop thread count with 10+ sessions

## Risk Assessment

- **High risk:** Editing wrong target file gives zero production benefit. Mitigation: only modify desktop leaf runtime path first.
- **Medium risk:** PTY fd may not support polling on every backend. Mitigation: unix fast path with fallback to existing waiter loop.
- **Medium risk:** Merged loop must preserve prompt de-dupe and output history behavior. Mitigation: move existing reader body intact, only wrap readiness around it.

## Verify

```bash
cargo check -p chatminal-desktop
# Stress test: open 20 sessions, run `ps -M $(pgrep chatminal-desktop)` to count threads
# Verify: shell exit detected promptly
# Verify: PTY output latency normal
```
