# Project Overview + PDR

Last updated: 2026-03-04

## Product goal
Xây terminal workspace local theo kiến trúc daemon-first, dùng terminal core nội bộ để xử lý terminal state ổn định.

## Scope (current)
- Multi-profile
- Multi-session
- PTY input/output/resize
- Session snapshot + history retention
- Local IPC between client and daemon

## Non-goals (current)
- Webview frontend
- Browser bridge runtime

## NFR
| ID | Requirement |
| --- | --- |
| NFR-01 | IPC phải local-only, không mở TCP production path |
| NFR-02 | PTY hot path không block bởi DB write |
| NFR-03 | Reconnect phải trả preview nhanh trước live output |
| NFR-04 | Retention policy phải giới hạn growth của scrollback |
