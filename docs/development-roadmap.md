# Development Roadmap

Last updated: 2026-03-04

## Completed
1. Tách daemon/client/protocol/store thành các crate/app độc lập.
2. Chuyển source tree sang runtime native-only theo hướng WezTerm core.
3. Chuyển repo sang workspace chỉ còn `apps/*` và `crates/*` đang dùng.
4. Dọn compatibility code legacy (`config.toml`, env typo aliases, text ping format).

## Active
1. Hoàn thiện native client UX trên nền WezTerm core.
2. Tối ưu daemon concurrency path.
3. Mở rộng test matrix terminal fidelity.
