# Docs extract: PDR + Deployment

Date: 2026-03-01
Scope: `docs/project-overview-pdr.md`, `docs/deployment-guide.md`

## 1) docs/project-overview-pdr.md

### Purpose
- Định nghĩa sản phẩm mức PDR: mục tiêu, phạm vi, yêu cầu chức năng/phi chức năng, tiêu chí chấp nhận, ràng buộc kỹ thuật, phụ thuộc, rủi ro.
- Là baseline để kiểm tra “đã build đúng cái gì” và “chưa làm gì”.

### Key sections
1. Project Overview + Problem Statement + Goals + Non-Goals.
2. Functional Requirements (FR-01..FR-08, trạng thái Implemented).
3. Non-Functional Requirements (NFR-01..NFR-06, kèm evidence).
4. Acceptance Criteria.
5. Technical Constraints.
6. Dependencies.
7. Risks and Mitigations.
8. Requirement Change Log.

### Areas needing update
1. Dependencies list đang lệch runtime thực tế:
- Có `vte` trong docs nhưng `Cargo.toml` hiện dùng `wezterm-term` + `wezterm-surface`.
- Cần cập nhật mục dependencies để phản ánh parser/render pipeline hiện tại.
2. NFR-05 test baseline ghi `13 unit tests` đã cũ so với changelog gần nhất (23).
3. Acceptance Criteria #1 nói launch có “at least one created session”; nên bổ sung edge case boot fail khi shell invalid (hiện có thể không có session).
4. Nên thêm explicit behavior: đóng session cuối thì app không auto mở session mới.
5. Nên thêm note về single-consumer PTY event receiver (`SESSION_EVENT_RX` semantics) để tránh hiểu sai khả năng multi-runtime.

## 2) docs/deployment-guide.md

### Purpose
- Tài liệu vận hành/dev-deploy: môi trường, build/run/test, release artifact, runtime config, checks, troubleshooting.

### Key sections
1. Environment requirements.
2. Local Development commands.
3. Release Build and binary output.
4. Runtime Configuration (`~/.config/chatminal/config.toml`) + clamp notes.
5. Operational Checks.
6. Troubleshooting matrix.
7. Packaging Notes.

### Areas needing update
1. Test baseline `13 passed` đã cũ; cần sync số hiện tại.
2. Troubleshooting thiếu case “app mở nhưng sidebar không có session” khi create session fail ở boot.
3. Runtime config note nói invalid TOML fallback default; nên ghi rõ là silent fallback (không warning log) để support team biết cách debug.
4. Nên thêm bước verify alternate-screen/scrollback behavior trong Operational Checks (vì runtime hiện có rule reset offset khi alt screen).
5. Có thể bổ sung quick check cho shell resolution order: config `shell` -> env `SHELL` -> `/bin/bash` -> `/bin/sh` (all phải pass `/etc/shells` + executable).

## Priority updates (đề xuất ngắn)
1. Sửa ngay dependency/runtime facts và test baseline (độ lệch thông tin hiện hữu).
2. Mở rộng troubleshooting cho boot-without-session + config parse silent fallback.
3. Thêm behavioral notes cho session lifecycle edge cases (close last session, selection policy).

## Unresolved questions
1. Có muốn docs giữ test baseline dạng số tuyệt đối (dễ stale) hay chuyển sang “run `cargo test` must pass”?
2. Có muốn đổi behavior từ silent config parse fallback sang warn log trước khi cập nhật deployment docs?
