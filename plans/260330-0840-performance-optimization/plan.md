# Chatminal Performance Optimization Plan

## Context

Chatminal Desktop (WezTerm fork, single-process GUI + runtime) cần tối ưu: low RAM, smooth render, scale nhiều sessions không lag. Audit phát hiện nhiều điểm nóng ở render pipeline, memory, threading, deps.

**Corrections from audit + code review:**
- VSync đã có (wgpu Fifo mode) → frame rate cap OK
- `live_output` đã có 1MB cap → không unbounded
- `reqwest`/`tokio` không trong desktop Cargo.toml → workspace metadata only
- Phase 03 phải tối ưu production desktop session engine (`apps/chatminal-desktop/src/desktop_host_runtime/session_engine/**`), không phải `crates/chatminal-runtime/src/session.rs` vì file đó đang là `#[cfg(test)]`
- Sidebar đã có `snapshot.version` / `ChatminalSidebar::version()` → Phase 01 nên dùng versioned subtree cache, không thêm dirty-flag tổng quát trong `TermWindow`
- Phase 02 phải ưu tiên clone nóng thực sự (`cluster.clone()`), không lặp lại tối ưu composing path vốn đã là conditional
- Phase 05 chỉ giữ các tối ưu draw path khả thi với API hiện tại; `UniformBuilder` chưa có `.set(...)`
- Phase 06 phải audit cả direct deps ở `apps/chatminal-desktop/Cargo.toml`, không chỉ workspace root

**Hard boundaries:** Không touch `crates/chatminal-terminal-core/**`

## Phases

| # | Phase | Priority | Status | File |
|---|-------|----------|--------|------|
| 01 | Sidebar Render Cache | HIGH | completed | [phase-01](phase-01-sidebar-render-cache.md) |
| 02 | Line Clone Elimination | HIGH | completed | [phase-02](phase-02-line-clone-elimination.md) |
| 03 | Session Thread Reduction | HIGH | completed | [phase-03](phase-03-session-thread-reduction.md) |
| 04 | Output History Buffer Cap | MEDIUM | completed | [phase-04](phase-04-output-history-buffer-cap.md) |
| 05 | GPU State Reuse | MEDIUM | completed | [phase-05](phase-05-gpu-state-reuse.md) |
| 06 | Workspace Dependency Cleanup | LOW | completed | [phase-06](phase-06-workspace-dep-cleanup.md) |
| 07 | Event Processor Clone Reduction | LOW | completed | [phase-07](phase-07-event-clone-reduction.md) |

## Dependencies

```text
Phase 01 + 02 (parallel) → biggest render perf win
Phase 03 + 04 (parallel) → scaling + memory
Phase 05 → after baseline measurement; independent với sidebar
Phase 06 + 07 → independent cleanup, anytime
```

## Expected Total Impact

Targets bên dưới là mục tiêu sau khi hoàn thành đúng production path, không phải guarantee cứng từ từng phase đơn lẻ.

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| RAM per idle session | ~4-5MB | ~2-3MB | -40-50% |
| CPU idle (10 sessions) | constant wakeups | event-driven | -75% |
| Threads per session | 3 | 2 | -33% |
| Sidebar allocs/frame (idle) | 20+ | 0 (cached) | -100% |
| Line render clones/frame | 2000+ | <100 | -95% |
| Binary size | baseline | -2-4MB | -5-10% |

## Final Verification

```bash
cargo check --workspace
cargo check -p chatminal-desktop
cargo test --workspace --lib --bins --tests
cargo test -p chatminal-runtime
# Manual: open 20 sessions, check Activity Monitor (threads + RAM)
# Manual: open 20 sessions, verify desktop leaf runtime thread count actually drops
# Manual: scroll fast, verify smooth rendering
# Manual: idle 5min, check CPU near 0%
```
