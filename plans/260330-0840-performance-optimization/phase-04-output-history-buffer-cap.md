# Phase 04: Output History Buffer Cap

## Overview
- **Priority:** MEDIUM — memory leak prevention
- **Status:** pending
- **Effort:** Low (~1-2 hours)

## Problem

`output_history: Arc<Mutex<Vec<String>>>` trong leaf_runtime.rs grows unbounded:
- Mỗi PTY output chunk append vào Vec, không bao giờ evict
- `replay_output()` (line ~216) joins all chunks: `output_history.join("")` — O(n) concatenation
- Long-running session với heavy output (e.g., `cat large_file`, build logs) → memory grows indefinitely
- Note: `live_output` trong runtime state ĐÃ có 1MB cap (`trim_live_output`), nhưng desktop-side `output_history` KHÔNG có cap

## Key Insights

- `output_history` dùng cho session restore — replay terminal output khi reconnect/switch tab
- Terminal engine đã có scrollback buffer (10,000 lines default) — `output_history` duplicate data
- `join("")` trên Vec<String> allocates new String mỗi lần replay — wasteful
- VecDeque cho O(1) drain front, Vec chỉ O(n)

## Related Code Files

### Modify
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime.rs` — output_history definition + append logic
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime_threads.rs` — hot append site in reader loop

### Read-only references
- `crates/chatminal-runtime/src/state/session_event_processor.rs` — `trim_live_output` pattern to follow
- `crates/chatminal-runtime/src/state.rs` — `max_scrollback_lines_per_session` config

## Implementation Steps

### Step 1: Switch Vec<String> → VecDeque<String> + byte tracker

```rust
use std::collections::VecDeque;

// Before
output_history: Arc<Mutex<Vec<String>>>,

// After
output_history: Arc<Mutex<OutputHistory>>,

/// Bounded output history buffer with byte tracking
struct OutputHistory {
    chunks: VecDeque<String>,
    total_bytes: usize,
}

const MAX_OUTPUT_HISTORY_BYTES: usize = 2 * 1024 * 1024; // 2MB cap
```

### Step 2: Implement OutputHistory methods

```rust
impl OutputHistory {
    fn new() -> Self {
        Self { chunks: VecDeque::new(), total_bytes: 0 }
    }

    fn push(&mut self, chunk: String) {
        self.total_bytes += chunk.len();
        self.chunks.push_back(chunk);
        self.trim();
    }

    fn trim(&mut self) {
        while self.total_bytes > MAX_OUTPUT_HISTORY_BYTES {
            if let Some(old) = self.chunks.pop_front() {
                self.total_bytes -= old.len();
            } else {
                break;
            }
        }
    }

    fn replay(&self) -> String {
        let mut buf = String::with_capacity(self.total_bytes);
        for chunk in &self.chunks {
            buf.push_str(chunk);
        }
        buf
    }

    fn clear(&mut self) {
        self.chunks.clear();
        self.total_bytes = 0;
    }
}
```

### Step 3: Update all output_history usage sites

- Initialization: `Arc::new(Mutex::new(OutputHistory::new()))`
- Append in reader loop: `history.lock().push(chunk)` (replaces `history.lock().push(chunk)` in `leaf_runtime_threads.rs`)
- Replay: `history.lock().replay()` (replaces `history.lock().join("")`)
- Initial scrollback load (line ~160-172): iterate and push each line

## Todo

- [ ] Create `OutputHistory` struct with VecDeque + byte tracking
- [ ] Replace `Vec<String>` with `OutputHistory` in leaf_runtime.rs
- [ ] Update `leaf_runtime_threads.rs` append site to use `OutputHistory::push()`
- [ ] Update `replay_output()` to use `OutputHistory::replay()`
- [ ] Update initialization with initial_scrollback

## Success Criteria

- `cargo check -p chatminal-desktop` — 0 errors
- Memory per session capped at ~3MB (1MB live_output + 2MB output_history)
- `cat /dev/urandom | head -c 10000000` → memory stays bounded
- Session restore still works (replay shows recent output)

## Risk Assessment

- **Low risk:** Oldest output trimmed → replay may miss very old output. Acceptable — terminal scrollback (10K lines) is authoritative source.
- **Low risk:** Scope is still limited to desktop session engine, but not leaf_runtime.rs alone. Mitigation: update both storage owner and reader append site together.

## Verify

```bash
cargo check -p chatminal-desktop
# Test: run `yes | head -n 1000000` in session, check memory doesn't grow past ~5MB total
# Test: switch tabs, verify session restore shows recent output
```
