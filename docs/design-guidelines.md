# Design Guidelines

Last updated: 2026-03-04

## UX direction
- Terminal-first native experience.
- Ưu tiên fidelity và độ ổn định của terminal behavior.
- Không thêm panel/phần UI phụ khi chưa cần cho terminal core.

## Interaction
- Session switch phải giữ đúng pane state theo session.
- Reconnect phải giữ history preview hợp lý trước live stream.
- Input/output path không chặn render loop.

## Visual constraints
- Giao diện tối giản, dễ đọc.
- Focus vào mật độ thông tin (sessions + active pane + status).
