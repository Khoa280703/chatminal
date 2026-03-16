# Phase 07 - Full Verification, Docs Sync, And Freeze

## Context Links
- `appendices/forbidden-symbols-contract.md`
- `appendices/end-state-manifest.md`
- `appendices/future-feature-acceptance-matrix.md`
- `appendices/commit-and-cutover-strategy.md`
- `appendices/final-exit-checklist.md`
- `docs/system-architecture.md`
- `docs/codebase-summary.md`
- `docs/project-changelog.md`
- `docs/development-roadmap.md`
- `plans/20260313-1140-chatminal-engine-private-primitives-cutover/plan.md`

## Overview
- Priority: P0
- Status: completed
- Brief: chốt verification cuối, cập nhật docs theo kiến trúc mới, và khóa boundary để các feature sau không leak lại vocabulary cũ.

## Key Insights
- Nếu không khóa grep/doc gates, chỉ vài batch sau là vocabulary cũ sẽ quay lại.
- Phase này cũng phải xử lý luôn các target thừa/rotted nếu quyết định giữ active `--all-targets` gate.

## Requirements
- Chạy full build/test gates cho luồng active.
- Quyết định rõ policy cho `cargo check --all-targets`:
  - hoặc sửa benches/tests cũ cho sạch
  - hoặc loại chúng khỏi active maintenance surface nếu không còn giá trị
- Cập nhật docs để mô tả đúng kiến trúc mới.
- Chốt grep gates cho boundary mới.

## Final Architecture Definition Of Done
- Feature mới ở app/UI layer có thể implement bằng:
  - `session`
  - `session_view`
  - `session_group`
  - `workspace_layout`
  - `render_target`
  mà không cần mental-map `tab = session`, `pane = terminal`.
- `apps/chatminal-desktop/src/chatminal_runtime/*` là facade desktop duy nhất cho product state.
- `termwindow/*` không own business routing; chỉ render/input shell.
- `desktop_host_runtime/*` là private adapter duy nhất chứa host primitives.
- `chatminal-lua-bridge` không còn buộc extensions/config dựa trên host ids cũ.
- Docs/code/tests/grep kể cùng một câu chuyện.

## Exact Verification Gates
- `cargo check --workspace`
- `cargo test -p chatminal-runtime -- --test-threads=1`
- `cargo test -p chatminal-session-runtime -- --test-threads=1`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- `cargo test --manifest-path apps/chatminald/Cargo.toml -- --test-threads=1`
- `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml -- --test-threads=1`
- `cargo check --workspace --all-targets`
  - expected: either pass clean, or policy doc + code changes make excluded targets no longer part of active maintenance surface
- `rg -n --glob '!third_party/**' --glob '!plans/**' --glob '!docs/**' "host_runtime::(Mux|tab::Tab|pane::Pane)|\\bMuxWindow\\b" apps/chatminal-desktop/src/chatminal_runtime apps/chatminal-desktop/src/termwindow apps/chatminal-desktop/src/desktop_termwindow_*`
  - expected: zero outside allowed private adapter scope
- `rg -n --glob '!third_party/**' --glob '!plans/**' --glob '!docs/**' "CloseTab|ActivateTab|ActivateTabRelative|ActivateTabRelativeNoWrap|ActivateLastTab|MoveTab|MoveTabRelative|get_host_tab|get_host_leaf" apps/chatminal-desktop crates/chatminal-lua-bridge`
  - expected: zero product-facing residuals or only annotated temporary compatibility shim if phase policy explicitly allows

## Architecture
- Sau phase này, docs và compile graph phải kể cùng một câu chuyện.
- Boundary freeze mới sẽ là nền cho các plan feature tiếp theo như session group, clone/group layout kiểu VSCode.

## Related Code Files
- Refactor: `docs/system-architecture.md`
- Refactor: `docs/codebase-summary.md`
- Refactor: `docs/project-changelog.md`
- Refactor: `docs/development-roadmap.md`
- Refactor/Delete: bench/test targets cũ nếu bị xác nhận không còn active value

## Implementation Steps
1. Chạy full gates cho active path.
2. Chạy grep gates cho `mux/tab/pane/surface/leaf` theo allowed scopes.
3. Sửa hoặc xóa benchmark/test targets thừa nếu chọn enforce `--all-targets`.
4. Cập nhật docs kiến trúc, roadmap, changelog.
5. Chạy qua [Final Exit Checklist](./appendices/final-exit-checklist.md) từng mục.
6. Mark plan complete chỉ khi docs, code, grep, tests đồng bộ.

## `--all-targets` Policy
- Preferred:
  - fix broken benches/tests cũ nếu chúng còn phản ánh engine behavior đáng giữ
- Acceptable delete:
  - bench/test target không thuộc luồng active, không có owner, và đã rot
- Not acceptable:
  - giữ target chết nhưng bỏ qua gate bằng lời nói

## Todo List
- [x] Active build/test gates pass
- [x] Grep gates pass theo allowed scopes
- [x] `--all-targets` policy được xử lý rõ ràng
- [x] Docs kiến trúc/codebase/changelog đồng bộ
- [x] Final architecture DOD được chứng minh bằng source + gates
- [x] Plan được khóa complete với evidence cụ thể

## Success Criteria
- Người mới vào repo đọc docs và code sẽ thấy một mô hình thống nhất.
- Feature plans sau này có thể giả định `session/layout/render_target` là language chính thức.
- Không còn “half-migrated architecture”, và có thể phát triển feature mới mà không cần quay lại refactor nền này.

## Risk Assessment
- Risk: docs update quá muộn dẫn tới lệch source.
- Mitigation: phase này bắt buộc trước khi đóng plan.

## Security Considerations
- Khi xóa test/bench targets phải chắc chắn không bỏ mất security-relevant validation path.

## Next Steps
- Không còn phase sau. Plan đã complete.
- Final report:
  - `reports/phase-07-final-verification-and-freeze.md`
