# Phase 04 - Daemon Contract and Runtime Stability

## Context Links
- [plan.md](./plan.md)
- [phase-02-linux-macos-shared-input-translation-layer.md](./phase-02-linux-macos-shared-input-translation-layer.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/request_handler.rs](/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/request_handler.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminald/src/session.rs](/home/khoa2807/working-sources/chatminal/apps/chatminald/src/session.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/session_event_processor.rs](/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/session_event_processor.rs)
- [/home/khoa2807/working-sources/chatminal/crates/chatminal-protocol/src/lib.rs](/home/khoa2807/working-sources/chatminal/crates/chatminal-protocol/src/lib.rs)

## Overview
- Priority: P1
- Status: Completed
- Effort: 6d
- Brief: giữ daemon-first invariant, cải thiện ổn định queue/backpressure để input mới không tạo regression.

## Key Insights
- `SessionRuntime::write_input` đang `try_send`; khi queue đầy có thể rớt input theo burst.
- Input pipeline mới tăng fidelity => tần suất events cao hơn, cần backpressure strategy rõ.
- Nên tránh breaking protocol nếu chưa cần thiết.

## Requirements
- Functional:
1. Daemon vẫn là điểm vào duy nhất cho input terminal.
2. Không mất input trong burst ngắn hợp lệ (Ctrl+C phải đi qua ngay).
3. Backpressure behavior determinist: trả lỗi rõ hoặc retry có giới hạn.
- Non-functional:
1. Không tăng lock contention đáng kể.
2. Protocol backward-compatible cho client hiện tại.

## Architecture
- Keep contract ổn định:
1. `Request::SessionInputWrite` giữ nguyên.
2. Nếu cần metadata, thêm field optional theo kiểu backward-compatible.
- Runtime queue policy:
1. tách control-priority writes (interrupt/control) và bulk text writes.
2. bounded retry với timeout ngắn thay vì fail tức thì trên burst.
3. metrics mới: `input_queue_full_total`, `input_retry_total`, `input_drop_total`.
- State update safety:
1. giữ generation guard.
2. đảm bảo seq/status update không block hot path quá lâu.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/session.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/request_handler.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/metrics.rs`
4. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/tests.rs`
5. `/home/khoa2807/working-sources/chatminal/crates/chatminal-protocol/src/lib.rs` (chỉ khi cần optional metadata)
- Create:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/input_backpressure.rs`
- Delete:
1. None

## Implementation Steps
1. Đo behavior queue full hiện tại bằng stress test input burst.
2. Thiết kế retry/priority policy nhỏ gọn (KISS, không thêm actor framework mới).
3. Triển khai metrics và error taxonomy rõ cho input write failures.
4. Bổ sung daemon tests cho burst + queue full + control key priority.
5. Re-check RTT/RSS sau thay đổi.

## Todo List
- [x] Có regression test cho tình huống queue full.
- [x] Có kiểm chứng Ctrl+C không bị drop dưới burst.
- [x] Cập nhật docs kiến trúc về backpressure policy.
- [x] Bật metrics log để phục vụ phase soak.

## Success Criteria
- Không thấy input loss ở test burst chuẩn.
- Control keys critical (Ctrl+C) có tỷ lệ thành công 100% trong test suite.
- Không breaking change với client/daemon cũ trong cùng branch.

## Risk Assessment
- Risk: retry policy làm tăng latency tail.
- Mitigation: đặt retry budget nhỏ + telemetry p95/p99 theo gate.
- Risk: thêm logic ưu tiên gây bug starvation text input.
- Mitigation: dùng fairness window và stress tests dài.

## Security Considerations
- Không tăng attack surface IPC (vẫn local-only, payload size guard giữ nguyên).
- Tránh log full input payload khi report queue pressure.

## Next Steps
- Sang Phase 05 để đóng gate tự động + matrix bắt buộc.

## Decisions Locked
1. Không thêm request-level idempotency token ở wave đầu; giữ benchmark markers và metrics hiện tại để giảm độ phức tạp.
