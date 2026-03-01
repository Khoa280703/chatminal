# Security Adversary Review — Chatminal Plan

**Reviewer:** code-reviewer (Security Adversary perspective)
**Date:** 2026-03-01
**Scope:** All 8 plan files — plan.md + phase-01 through phase-07
**Plan path:** `/home/khoa2807/working-sources/chatminal/plans/260301-1521-chatminal-messenger-terminal/`

---

## Finding 1: Shell Path Injection qua Config File

- **Severity:** Critical
- **Location:** Phase 07, section "Security Considerations" + "Implementation Steps" step 1
- **Flaw:** Plan đề xuất validate shell path bằng `std::fs::metadata(path).is_ok()` — không đủ. Chỉ verify file tồn tại và executable, không ngăn path traversal hay substitution. Shell được đọc từ `~/.config/chatminal/config.toml` không được sanitize trước khi truyền vào `CommandBuilder`.
- **Failure scenario:** Kẻ tấn công kiểm soát config file (shared machine, malicious dotfile manager, symlink attack) đặt `shell = "/tmp/malicious_backdoor"`. Vì file tồn tại và có execute bit, validation pass. App spawn PTY với process độc hại, chạy dưới user's context với full PTY access và toàn bộ env vars của user — full RCE nếu config đến từ nguồn không tin cậy.
- **Evidence:** Phase 07, Security: *"Shell path from config: validate it exists and is executable"* — không có allowlist, không có path canonicalization, không restrict đến known shells.
- **Suggested fix:** Validate shell path chỉ từ allowlist (`/bin/bash`, `/bin/zsh`, `/usr/bin/fish`, etc.) hoặc verify nó xuất hiện trong `/etc/shells`. Canonicalize path trước validate. Từ chối bất kỳ giá trị nào chứa `..`, null byte, hoặc path ngoài expected directories.

---

## Finding 2: PTY Input Forwarding Không Giới Hạn Kích Thước — DoS qua Memory Exhaustion

- **Severity:** High
- **Location:** Phase 05, section "Security Considerations" + Phase 02, "Architecture"
- **Flaw:** Plan tuyên bố *"Input bytes forwarded verbatim to PTY — no sanitization needed"*. Đúng về content, nhưng không có giới hạn kích thước trên mỗi input write. `input_tx: tokio::sync::mpsc::Sender<Vec<u8>>` nhận `Vec<u8>` không bounded về kích thước.
- **Failure scenario:** Iced clipboard paste event chuyển một file 100MB được paste vào terminal. Hệ thống tạo `Vec<u8>` với 100MB data, đẩy vào bounded channel (cap=4). 4 slots × 100MB = 400MB trên channel queue. App không crash ngay nhưng memory spike nghiêm trọng, bounded channel đầy khiến UI thread block.
- **Evidence:** Phase 05: *"Input bytes forwarded verbatim to PTY — intentional; no sanitization needed"*. Phase 02 channel: `bounded(4)` chỉ giới hạn số message, không size mỗi message.
- **Suggested fix:** Thêm `MAX_INPUT_BYTES_PER_WRITE = 64_000` (64KB). Truncate hoặc chunk input trước khi gửi vào channel.

---

## Finding 3: Terminal Escape Sequence Injection — OSC/DCS Attacks qua PTY Output

- **Severity:** High
- **Location:** Phase 02, section "Implementation Steps" step 2 (vte::Perform impl) + Phase 04, section "Security Considerations"
- **Flaw:** Plan implement `vte::Perform` với một số CSI/ESC handlers, nhưng không đề cập xử lý các escape sequences nguy hiểm: OSC (Operating System Commands), DCS, và đặc biệt **OSC 52** (clipboard write). Một process chạy trong PTY có thể gửi `\x1b]52;c;BASE64DATA\x07` để ghi vào clipboard của host mà không cần user interaction.
- **Failure scenario:** User chạy `cat malicious_file.txt`. File chứa OSC 52 sequence. Nếu implementation sau này thêm OSC 52 support (reasonable feature request), clipboard hijacking trở thành trivial. Cũng: OSC 8 hyperlinks có thể render URLs gây confusion/phishing.
- **Evidence:** Phase 02: chỉ mention `?1049h/l, ?25h/l` — không mention OSC handlers. Phase 04 Security: *"not applicable for Canvas renderer"* — sai, applicable ở PTY parsing layer.
- **Suggested fix:** Explicitly document rằng OSC 52 sẽ NOT được implement và bị silently dropped. Audit tất cả OSC sequences, chỉ implement safe ones với length limits.

---

## Finding 4: Shell Inherits Toàn Bộ Parent Environment — Credential Leakage

- **Severity:** High
- **Location:** Phase 02, section "Security Considerations" + "Implementation Steps" step 4
- **Flaw:** Plan ghi nhận *"do not inject secrets into CommandBuilder env"* nhưng không có bước nào FILTER parent environment trước khi spawn. `CommandBuilder` mặc định kế thừa toàn bộ process env, bao gồm `ANTHROPIC_API_KEY`, `AWS_SECRET_ACCESS_KEY`, `GITHUB_TOKEN`, etc.
- **Failure scenario:** User chạy `chatminal` từ terminal session đã có `AWS_SECRET_ACCESS_KEY` set (CI/CD environment). PTY shell sessions kế thừa env đó. Nếu app sau này thêm session recording/export feature, toàn bộ credentials bị exposed.
- **Evidence:** Phase 02: *"Shell inherits parent env — do not inject secrets into CommandBuilder"* — chỉ cấm inject thêm, không filter existing env.
- **Suggested fix:** Strip known credential patterns (`*_KEY`, `*_SECRET`, `*_TOKEN`, `*PASSWORD*`) từ spawned shell env, hoặc document rõ behavior này.

---

## Finding 5: Race Condition — UI Thread Block khi Close Session Dưới Heavy Output

- **Severity:** High
- **Location:** Phase 07, section "Risk Assessment" + Phase 02, Architecture
- **Flaw:** Plan thừa nhận race condition nhưng bỏ sót case: sau `drop(master)`, reader thread có thể đang block ở `blocking_send()` trên full channel (cap=4). `reader_handle.join()` trong `close_session()` block caller thread — đây là Iced's UI/main thread — gây UI freeze.
- **Failure scenario:** User đóng session đang `cat /dev/urandom | head -c 100M`. Reader thread block ở `blocking_send()` vì channel full. Channel drain cần Iced Subscription chạy trên tokio. Nhưng nếu `close_session()` đang block UI thread, Iced không process events, tokio subscription không drain channel, deadlock hoàn toàn.
- **Evidence:** Phase 02: *"blocking_send from OS thread to tokio mpsc channel"* + *"reader_handle.join()"* gọi trong `close_session()` trên cùng thread. Channel cap=4.
- **Suggested fix:** `close_session()` không gọi `join()` synchronously trên UI thread. Spawn detached cleanup thread, hoặc dùng `try_join` với timeout. Drain channel trước khi drop master.

---

## Finding 6: Font File Downloaded từ External URL Không Verify Hash — Supply Chain Risk

- **Severity:** High
- **Location:** Phase 01, section "Implementation Steps" step 3
- **Flaw:** Plan hướng dẫn download font bằng `curl -L "https://github.com/JetBrains/JetBrainsMono/releases/latest/download/..."`. Dùng `latest` URL thay vì pinned version. Không có hash verification. Font được `include_bytes!()` embed vào binary.
- **Failure scenario:** MITM/DNS hijack trong developer's network, hoặc JetBrains repo compromise → attacker-controlled TTF embed vào release binary. TTF parser trong wgpu/Iced có lịch sử CVEs — malformed font ảnh hưởng tất cả users của binary.
- **Evidence:** Phase 01: `curl -L "https://...JetBrainsMono/releases/latest/download/JetBrainsMono-2.304.zip"` — không verify SHA256/SHA512 hash sau download.
- **Suggested fix:** Pin URL đến specific version (không `latest`). Add `sha256sum -c` verification sau download. Document expected hash. Hoặc commit font file vào repo trực tiếp.

---

## Finding 7: Mutex Poisoning Silently Disables Terminal Updates — Zero UX Feedback

- **Severity:** Medium
- **Location:** Phase 03, section "Implementation Steps" step 4 (subscription code)
- **Flaw:** Code xử lý `Err(_)` mutex poison case bằng `log::error!` rồi `return` — kill toàn bộ terminal update subscription silently. Không có recovery, không có user-visible error.
- **Failure scenario:** Bất kỳ panic nào trong tokio runtime gây mutex poisoning. Subscription closure trả về early. User thấy terminal sessions vẫn trong sidebar nhưng output frozen — không có error message. User force-quit, mất work đang chạy.
- **Evidence:** Phase 03: *"Err(_) → log::error!('update_rx mutex poisoned; disable session subscription') → return"*
- **Suggested fix:** Dùng `poisoned.into_inner()` thay vì return — mutex poisoning có thể recover. Đây là Rust pattern chuẩn khi lock holder không mutate inconsistent state.

---

## Finding 8: Không Có Process Isolation Giữa Sessions — Input Routing Bug Attack Surface

- **Severity:** Medium
- **Location:** Phase 02, toàn bộ architecture section
- **Flaw:** `SessionManager` owns tất cả session masters trong cùng `IndexMap` không có isolation. Không có guard/assertion enforce invariant "input chỉ gửi đến active_session_id". Nếu stale `SessionId` reference gây wrong lookup, input từ session A có thể gửi đến PTY của session B.
- **Failure scenario:** Logic bug trong `active_session_id` state management (sau CloseSession + rapid NewSession) cause wrong session lookup. Keystroke từ user gửi đến wrong session — nếu user đang sudo trong session B, keystrokes từ session A ghi đến session B's sudo prompt.
- **Evidence:** Phase 05: *"Input must ONLY go to active_session_id — never broadcast"* — intention documented nhưng không có enforcement mechanism trong `send_input` implementation plan.
- **Suggested fix:** Thêm assertion trong `send_input` verify `session_id` matches `active_session_id`. Thêm session validation step trước mỗi input forward.

---

## Unresolved Questions

1. Plan không đề cập đến `seccomp` filtering hay sandbox cho spawned PTY processes — liệu MVP có cần process-level sandboxing không?
2. `cargo audit` được mention ở phase 01 nhưng không được integrate vào CI/CD pipeline (không có `.github/workflows/` trong planned file structure) — audit chỉ chạy một lần tại setup time, không liên tục.
3. OSC 52 clipboard write có được implement không? Plan ambiguous — không explicit deny, không explicit allow.
