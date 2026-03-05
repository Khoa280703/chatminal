# Phase 02 - Daemon Concurrency and Performance

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state.rs](/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state.rs)

## Overview
- Priority: P1
- Status: Completed
- Mục tiêu: giảm lock contention, kiểm soát queue/backpressure, ổn định throughput dài hạn.

## Key Insights
- `state.rs` đang là hot path lớn, lock scope rộng.
- Cần tách nhanh read/write path để tránh chặn PTY output.
- Đã có benchmark hard-gate script RTT/RSS để chặn regression sớm trước soak dài.
- Với `release` profile, benchmark gate local hiện pass mục tiêu (`p95/p99`) và hard-gates.
- Đã bắt đầu modularization bằng việc tách explorer path/state utilities ra module riêng.
- Đã tách tiếp các cụm lớn khỏi `state.rs`: session explorer handlers, runtime lifecycle publishers, session event processor, tests.
- Đã re-run benchmark gate sau batch mới nhất:
  - `p95=14.230ms`, `p99=15.011ms`
  - daemon peak RSS `6.3MB`, app peak RSS `4.7MB`, tổng `11.0MB`
  - pass target + pass hard gate.

## Requirements
- Functional:
1. Request path không block event fan-out.
2. History writer batch transaction ổn định khi output lớn.
- Non-functional:
1. Không tăng memory vô hạn khi client chậm.
2. p95 local input RTT `<= 30ms`; fail nếu p95 `> 45ms`.
3. p99 local input RTT `<= 60ms`.
4. Memory budget RSS cho daemon mục tiêu `<= 220MB`; fail nếu `> 300MB`.

## Architecture
- Tách `session_manager`, `event_hub`, `history_writer`, `workspace_state`.
- Dùng bounded channels + drop policy rõ ràng.
- Thêm metric counters nội bộ (queue depth, dropped frames, flush latency).

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/server.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/session.rs`
4. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/config.rs`
- Create:
1. `apps/chatminald/src/state/*` modules tách concern
2. `apps/chatminald/src/metrics.rs`
3. `scripts/bench/phase02-rtt-memory-gate.sh`
- Delete:
1. Legacy utility nội bộ trùng chức năng sau refactor

## Implementation Steps
1. Chia `state.rs` thành modules theo trách nhiệm.
2. Áp bounded queue + backpressure policy.
3. Tách DB writer thread và đo latency batch commit.
4. Thêm tests cho stale event, race clear-history, slow client.
5. Thêm benchmark script local.

## Todo List
- [x] Refactor state thành module nhỏ, giữ API không vỡ. (đã tách `request_handler`, `explorer_utils`, `session_explorer`, `runtime_lifecycle`, `session_event_processor`, `state/tests`)
- [x] Thêm metrics và log sampling baseline (requests/events/broadcast/drop counters).
- [x] Thêm integration test baseline cho contention/race (clear-history generation gate, workspace clear-all multi-session).
- [x] Benchmark trước/sau để xác nhận hiệu quả (script `scripts/bench/phase02-rtt-memory-gate.sh` + command `bench-rtt-wezterm`).

## Success Criteria
- Soak 24h không deadlock, memory growth bounded.
- Slow client không làm treo session output toàn cục.
- Queue pressure có metric quan sát được.
- RTT và memory đạt KPI đã chốt trong `plan.md`.

## Risk Assessment
- Risk: refactor lớn gây regression protocol.
- Mitigation: giữ contract test + golden integration tests.

## Security Considerations
- Giữ quyền socket user-only.
- Không log raw payload nhạy cảm ở mức info.

## Next Steps
- Theo dõi regression qua CI quality gates định kỳ.

## Unresolved questions
- None.
