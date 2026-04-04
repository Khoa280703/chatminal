# Macro 4-Phase Staffing

## Goal
Gom 10 phase hiện tại thành 4 macro-phase để dễ chia việc cho team 4 người, nhưng vẫn giữ dependency thật của plan gốc.

## Important Constraint
- Có thể gom để **quản lý**.
- Không thể coi 4 macro-phase là **4 khối độc lập chạy đồng thời từ đầu đến cuối**.
- Mỗi macro-phase vẫn chỉ được mở khi gate của macro-phase trước đã xanh.

## Macro Phase Map

### Macro Phase 1: Contract Freeze
- Gồm:
  - [Phase 01](./phase-01-regression-and-contract-lock.md)
  - [Phase 02](./phase-02-host-runtime-api-foundation.md)
- Mục tiêu:
  - khóa regression đang mở
  - khóa host-runtime public contract để phần sau không đổi nền giữa chừng
- Tính chất:
  - ngắn nhưng rất quan trọng
  - không nên có nhiều implementer cùng sửa foundation file

### Macro Phase 2: Consumer And UI Cutover
- Gồm:
  - [Phase 03](./phase-03-consumer-cutover-parallel.md)
  - [Phase 04](./phase-04-ui-and-desktop-facade-parallel.md)
- Mục tiêu:
  - migrate consumer lớn sang contract mới
  - dọn raw handle / typed handle / facade / UI callsites
- Tính chất:
  - phase đáng nhét nhiều người nhất
  - ownership lane tách khá rõ

### Macro Phase 3: Ownership And Runtime Wiring
- Gồm:
  - [Phase 05](./phase-05-control-plane-foundation.md)
  - [Phase 06](./phase-06-runtimehost-wiring-parallel.md)
  - [Phase 07](./phase-07-pty-owner-migration-parallel.md)
- Mục tiêu:
  - tách control-plane khỏi singleton/Mux shape
  - chuyển sang explicit RuntimeHost wiring
  - migrate ownership thật của PTY/output/exit
- Tính chất:
  - nặng nhất toàn plan
  - nên xem đây là cụm kiến trúc lõi, cần một lead giữ nhịp

### Macro Phase 4: Config And Final Sweep
- Gồm:
  - [Phase 08](./phase-08-config-foundation.md)
  - [Phase 09](./phase-09-config-propagation-parallel.md)
  - [Phase 10](./phase-10-final-rename-and-sweep.md)
- Mục tiêu:
  - khóa config foundation
  - propagate config bỏ singleton reads trong scope mục tiêu
  - chạy rename/final sweep cuối
- Tính chất:
  - phần config có thể song song nội bộ
  - rename cuối phải do 1 owner làm

## Recommended Team Split For 4 People

### Person 1: Core Lead
- Giữ các phần foundation/lõi:
  - `02A`
  - `05A`
  - `08A`
  - `10A`
- Vai trò:
  - chốt contract
  - review mọi lane có đụng boundary cross-crate

### Person 2: Desktop/UI Owner
- Giữ các lane desktop/UI-heavy:
  - `03B`
  - `04A`
  - `04B`
  - `06A`
  - `09A`

### Person 3: Lua/Runtime Consumer Owner
- Giữ các lane runtime consumer:
  - `01B`
  - `03A`
  - `06B`

### Person 4: PTY/Host Runtime Owner
- Giữ các lane runtime/PTY sâu:
  - `01A`
  - `03C`
  - `07A`
  - `07B`
  - `09B`

## Best Execution Model

### Wave 1
- Person 3 làm `01B`
- Person 4 làm `01A`
- Person 1 review gate và chuẩn bị `02A`
- Person 2 đọc trước desktop callsites để chuẩn bị cutover

### Wave 2
- Person 1 làm `02A`
- Các người còn lại chuẩn bị branch/notes/test plan, nhưng chưa merge gì phụ thuộc contract mới cho tới khi `02A` xong

### Wave 3
- Person 3 làm `03A`
- Person 2 làm `03B`
- Person 4 làm `03C`
- Person 1 review và chốt gate cho sang `04`

### Wave 4
- Person 2 làm `04A` và `04B`
- Person 1 chuẩn bị `05A`
- Person 3 và 4 hỗ trợ smoke/review nếu rảnh

### Wave 5
- Person 1 làm `05A`
- Những người còn lại chỉ prep cho `06`, không merge code phá assumption của `05A`

### Wave 6
- Person 2 làm `06A`
- Person 3 làm `06B`
- Person 4 hỗ trợ trace/smoke
- Person 1 review contract

### Wave 7
- Person 4 làm `07A` và `07B`
- Person 1 giữ review chặt vì đây là cụm rủi ro cao nhất
- Person 2/3 hỗ trợ repro/test matrix

### Wave 8
- Person 1 làm `08A`
- Person 2 làm `09A`
- Person 4 làm `09B`
- Person 3 hỗ trợ grep audit / verification

### Wave 9
- Person 1 làm `10A`
- Person 2/3/4 chỉ làm smoke, docs sync, grep sweep, không chạm rename tree nếu chưa được phân

## What Not To Do
- Không giao “mỗi người 1 macro-phase” rồi chạy cùng lúc.
- Không tách `02A`, `05A`, `08A`, `10A` cho nhiều implementer.
- Không cho lane sau tự đổi contract foundation nếu gate phase trước chưa khóa.

## Practical Conclusion
- `Có thể` gom 10 phase thành 4 macro-phase để quản lý và báo cáo.
- `Không nên` dùng 4 macro-phase như 4 workstream độc lập hoàn toàn.
- Với team 4 người, cách tối ưu là:
  - quản lý theo 4 macro-phase
  - thực thi theo lane/wave bên trong từng macro-phase
