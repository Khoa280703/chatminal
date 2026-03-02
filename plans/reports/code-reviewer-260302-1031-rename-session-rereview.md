## Code Review Summary

### Scope
- Files: `frontend/src/App.svelte`, `src-tauri/src/service.rs`, `src-tauri/src/models.rs`, `src-tauri/src/main.rs`
- LOC: 1160
- Focus: specific (rename-session re-review)
- Scout findings:
  - `git diff --name-only HEAD~1` không chứa code rename (chủ yếu docs), nên scout theo targeted dependency tracing trên code hiện tại.
  - Rủi ro chính nằm ở async state mutation (`renameBusy`) và consistency rule giữa API create/rename.

### Overall Assessment
- Luồng rename đã tốt hơn trước (input click/keydown đã chặn bubble đúng hướng), build pass.
- Tuy nhiên còn 1 lỗi concurrency thực sự ở guard `renameBusy`, và 1 inconsistency validation giữa create/rename ở backend API contract.

### Critical Issues
- None.

### High Priority
1. `renameBusy` guard bị bypass bằng phím `Escape` khi request rename đang in-flight.
- Location: `frontend/src/App.svelte:82`, `frontend/src/App.svelte:264`, `frontend/src/App.svelte:335`
- Impact: trong lúc `rename_session` đang chờ backend, user vẫn có thể bấm `Escape` tại input (input không disable), `cancelRename()` reset `renameBusy=false` sớm. Kết quả: có thể mở/submit rename mới trước khi request cũ kết thúc -> race + state flicker.
- Fix gợi ý:
  - Không reset `renameBusy` trong `cancelRename()`.
  - Disable input khi `renameBusy`.
  - Bỏ xử lý `Escape` khi `renameBusy=true`.

### Medium Priority
1. Validation create/rename chưa nhất quán cho empty/whitespace name.
- Location: `src-tauri/src/service.rs:96`, `src-tauri/src/service.rs:238`, `src-tauri/src/service.rs:309`
- Impact: `create_session` với `name="   "` sẽ silently fallback sang default `Session {n}`, nhưng `rename_session` với cùng input trả lỗi `session name cannot be empty`. API behavior khó đoán khi client truyền explicit empty name.
- Fix gợi ý:
  - Nếu `payload.name` là `Some`, luôn chạy `validate_session_name`.
  - Chỉ auto-default khi `payload.name` là `None`.

2. Keyboard bubbling còn side-effect ở nút Save/Cancel.
- Location: `frontend/src/App.svelte:331`, `frontend/src/App.svelte:349`, `frontend/src/App.svelte:356`
- Impact: parent row có `on:keydown` (Enter/Space => `setActiveSession`), nhưng Save/Cancel chỉ chặn `click`, không chặn `keydown`. Dùng keyboard trên button có thể trigger chọn session ngoài ý muốn.
- Fix gợi ý: thêm `on:keydown|stopPropagation` cho Save/Cancel (hoặc guard target trong `onSessionKeydown`).

### Low Priority
- No additional low-priority issues in scoped area.

### Edge Cases Found by Scout
1. In-flight rename + `Escape` -> reset busy state sớm -> mở ra concurrent rename.
2. Button keyboard event bubbling -> đổi active session ngoài ý muốn khi thao tác rename bằng keyboard.
3. API client gửi whitespace name khi create vs rename nhận behavior khác nhau.

### Positive Observations
- Input rename đã có `on:click|stopPropagation` và `on:keydown|stopPropagation`, giảm đáng kể bubbling lỗi trực tiếp từ input.
- Server-side có `validate_session_name` shared cho giới hạn độ dài + trim.
- Build/check hiện pass: `cargo test --manifest-path src-tauri/Cargo.toml`, `npm --prefix frontend run build`.

### Recommended Actions
1. Sửa guard `renameBusy` (không reset trong `cancelRename`, khóa input khi busy).
2. Chuẩn hóa rule create/rename: explicit empty name phải cùng behavior.
3. Chặn keyboard bubbling trên Save/Cancel để hoàn tất triệt để bug nhóm “bubble”.

### Metrics
- Type Coverage: N/A (không có report coverage)
- Test Coverage: N/A (không có unit/integration test cho luồng rename; `cargo test` hiện 0 test)
- Linting Issues: N/A (repo chưa có script lint rõ ràng trong scope này)

### Unresolved Questions
1. Sản phẩm muốn hành vi chính thức nào cho `create_session(name="   ")`: reject hay auto-fallback default?
2. Khi rename đang pending, UX mong muốn có cho phép `Cancel` thật sự (abort request) hay chỉ khóa toàn bộ input đến khi request trả về?
