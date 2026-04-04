# Phase 04 - Verify Closeout And Doc Sync

## Context Links
- [plan.md](/Users/khoa2807/development/2026/chatminal/plans/260404-1532-merge-terminal-core-and-emulator/plan.md)
- [docs/system-architecture.md](/Users/khoa2807/development/2026/chatminal/docs/system-architecture.md)
- [docs/codebase-summary.md](/Users/khoa2807/development/2026/chatminal/docs/codebase-summary.md)

## Overview
- Priority: P1
- Status: done
- Brief: prove plan thật sự closed, không còn “merge trên giấy”.

## Key Insights
- Merge terminal layer rất dễ thất bại theo kiểu compile pass nhưng docs, tests, hoặc naming seam vẫn mô tả reality cũ.
- Closeout tốt phải tách rõ cái gì đã xong kiến trúc và cái gì chỉ còn là naming debt không làm duplicate behavior nữa.

## Requirements
- Run full verification spine.
- Sync docs active scope.
- Chốt residual debt có còn lại hay không, nhất là `engine_term` lib name.

## Architecture
- Architecture done-state:
  - one active terminal layer
  - no product-path dependency on `chatminal-terminal-core`
  - no steady-state type translation between “core size” và “io size”
- no adapter crate/song song terminal contract under a softer name
- Residual acceptable debt:
  - `engine_term` import name có thể được giữ tạm nếu chỉ là naming alias, không còn là second architecture layer.

## Related Code Files
- Modify: `docs/project-changelog.md`
- Modify: `docs/system-architecture.md`
- Modify: `docs/codebase-summary.md`
- Modify: `README.md`
- Optional follow-up only if needed: package/lib naming docs for `engine_term`

## Implementation Steps
1. Chạy verification commands đầy đủ.
2. Smoke `make window` để chắc desktop path vẫn boot.
3. Grep cuối cùng cho `chatminal_terminal_core` và dual-layer wording.
4. Update docs/changelog với kết luận closeout.
5. Nếu còn naming debt `engine_term`, ghi rõ là naming-only debt, không phải architecture debt.

## Todo List
- [x] `cargo check --workspace`
- [x] `cargo test --workspace --lib --bins --tests`
- [x] `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- [x] `make window` smoke
- [x] Active docs sync
- [x] Closeout note rõ residual naming debt

## Success Criteria
- Verification spine xanh.
- Docs active scope không còn nhắc tới two-terminal-layer reality.
- Kết luận kiến trúc đủ mạnh để nói plan hoàn thành 100%.

## Risk Assessment
- Risk: docs nói “done” nhưng source vẫn còn residual dependency.
- Mitigation: closeout chỉ dựa trên grep/cargo tree/check thực tế, không dựa trên assumption.

## Security Considerations
- Không có security change trực tiếp.
- Phải giữ nguyên expected terminal behavior khi smoke launch và interaction cơ bản.

## Next Steps
- Nếu muốn đẹp thêm sau closeout: plan riêng cho `engine_term` lib-name rename hoặc API polish, không gộp vào plan merge này nếu không phải blocker.
