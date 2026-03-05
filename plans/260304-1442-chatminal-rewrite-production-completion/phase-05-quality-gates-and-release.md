# Phase 05 - Quality Gates and Release

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/.github/workflows/rewrite-quality-gates.yml](/home/khoa2807/working-sources/chatminal/.github/workflows/rewrite-quality-gates.yml)

## Overview
- Priority: P1
- Status: Completed
- Mục tiêu: dựng gate tự động + checklist thủ công để cutover production.

## Release Order
1. Stable channel cho Linux + macOS trước.
2. Windows release sau khi Named Pipe + matrix fidelity pass ổn định.

## Hard Gates (chốt)
1. Latency:
- p95 input RTT `<= 30ms` (fail nếu `> 45ms`)
- p99 input RTT `<= 60ms`
2. Memory RSS:
- `chatminald` mục tiêu `<= 220MB`, fail nếu `> 300MB`
- `chatminal-app` mục tiêu `<= 280MB`, fail nếu `> 380MB`
- Tổng hai process mục tiêu `<= 450MB`, fail nếu `> 600MB`

## Key Insights
- Rewrite chỉ an toàn khi gate theo matrix thực tế, không chỉ unit test.
- Cần freeze scope trước release để tránh regression cuối.
- Dry-run release cần có artifact checksum + smoke từ binary release, không chỉ từ `cargo run` path.

## Requirements
- Functional:
1. CI chạy check/test/integration matrix bắt buộc.
2. Manual scripts cho fidelity + reconnect + soak.
3. Release artifacts + notes nhất quán.
- Non-functional:
1. Reproducible build.
2. Có rollback plan rõ ràng.

## Architecture
- Quality gate pipeline: build -> tests -> integration -> fidelity scripts -> soak summary.
- Release pipeline: tag -> artifacts -> checksum -> changelog.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/.github/workflows/rewrite-quality-gates.yml`
2. `/home/khoa2807/working-sources/chatminal/docs/development-roadmap.md`
3. `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`
- Create:
1. `scripts/soak/*`
2. `scripts/fidelity/*`
3. `docs/release-checklist.md`
- Delete:
1. Script tạm không còn dùng sau gate chuẩn hóa

## Implementation Steps
1. Mở rộng workflow CI theo phase outputs.
2. Thêm scripts soak/fidelity có format report ổn định.
3. Viết release checklist + rollback checklist.
4. Dry-run release trên branch staging.
5. Cutover main khi pass toàn bộ gate.

## Todo List
- [x] Chốt checklist gate bắt buộc (auto + manual). (`docs/release-checklist.md`)
- [x] Thêm scripts soak/fidelity có output machine-readable. (`scripts/fidelity/phase05-fidelity-smoke.sh`, `scripts/soak/phase05-soak-smoke.sh`)
- [x] Cập nhật docs roadmap/changelog/architecture sau mỗi milestone.
- [x] Chốt tiêu chí cutover và rollback. (`docs/release-checklist.md`)
- [x] Thêm dry-run release script tạo artifact + checksum + smoke report. (`scripts/release/phase05-release-dry-run.sh`)
- [x] Harden release dry-run portability + fail-path observability:
  - checksum Linux/macOS (`sha256sum`/`shasum -a 256`)
  - luôn ghi JSON report kể cả fail sớm (fallback path trong `/tmp`)
  - build daemon/app tách riêng fail-fast để tránh false-pass khi một lệnh build lỗi
- [x] Chạy full quality gates 2 vòng liên tiếp:
  - `cargo check --workspace`
  - `cargo test` (daemon/app)
  - `scripts/bench/phase02-rtt-memory-gate.sh`
  - `scripts/fidelity/phase05-fidelity-smoke.sh`
  - `scripts/soak/phase05-soak-smoke.sh`
  - `scripts/release/phase05-release-dry-run.sh`
  - `validate-docs`

## Success Criteria
- Pass full quality gate 2 vòng liên tiếp.
- Không còn P1/P0 bug trước tag release.
- Release artifact cài chạy được theo deployment guide.
- Pass đầy đủ Hard Gates latency/memory ở trên trong 2 vòng liên tiếp.

## Risk Assessment
- Risk: kéo dài thời gian do bug fidelity cuối kỳ.
- Mitigation: freeze feature, ưu tiên fix regression trước.

## Security Considerations
- Scan artifacts và cấu hình trước publish.
- Bảo đảm không lộ endpoint/path nội bộ trong release logs.

## Next Steps
- Duy trì gate matrix Linux/macOS/Windows trong CI để bắt regression sớm.

## Unresolved questions
- None.
