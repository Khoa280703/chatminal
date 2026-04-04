# Workload Ranking

## How To Read
- `Workload`: khối lượng kỹ thuật + breadth file ownership + mức cần self-verify.
- `Risk`: xác suất làm gãy contract/flow đang chạy.
- `Suggested Staff`: số người hợp lý cho phase đó; không phải cứ nhiều người hơn là nhanh hơn.
- `Critical Path`: phase có nằm trên đường phải mở khóa tuần tự hay không. Ở plan này, phase nào cũng nằm trên critical path; khác nhau ở chỗ phase nào có lane để nhét thêm người vào.

## Phase Ranking By Workload

| Rank | Phase | Workload | Risk | Suggested Staff | Max Parallel Lanes | Vì sao nặng |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | [Phase 07](./phase-07-pty-owner-migration-parallel.md) | Very High | Very High | 3 | 2 | Đụng ownership thật của PTY/output/exit; chỉ cần lệch lifecycle một nhịp là sinh ghost output, cleanup sai, crash khó debug. |
| 2 | [Phase 05](./phase-05-control-plane-foundation.md) | Very High | Very High | 1 | 1 | Foundation khó nhất; đụng control-plane, singleton shape, ownership root. Không nên chia nhiều người vì file overlap gần như 100%. |
| 3 | [Phase 06](./phase-06-runtimehost-wiring-parallel.md) | High | Very High | 3 | 2 | Sau foundation phải kéo desktop + lua sang explicit RuntimeHost; breadth callsite lớn, sai một edge là split-brain runtime. |
| 4 | [Phase 02](./phase-02-host-runtime-api-foundation.md) | High | Very High | 1 | 1 | Freeze public host-runtime contract; phase này unblock gần như toàn bộ phần sau nên ít người nhưng phải rất chắc tay. |
| 5 | [Phase 09](./phase-09-config-propagation-parallel.md) | High | High | 3 | 2 | Scope file rộng, nhiều singleton reads rải rác; dễ sót path và tạo config drift giữa desktop với host-runtime. |
| 6 | [Phase 03](./phase-03-consumer-cutover-parallel.md) | Medium-High | High | 4 | 3 | Phù hợp nhất để tăng headcount: Lua, desktop adapter, config dead sweep tách ownership rõ. |
| 7 | [Phase 04](./phase-04-ui-and-desktop-facade-parallel.md) | Medium | Medium-High | 2 | 2 | Nhiều callsite UI/facade nhưng risk thấp hơn PTY/control-plane; chủ yếu cleanup typed-handle/raw-id. |
| 8 | [Phase 10](./phase-10-final-rename-and-sweep.md) | Medium | High | 1 | 1 | Rename toàn workspace không quá khó về logic nhưng conflict rất cao; phải để cuối và giao một người cầm trịch. |
| 9 | [Phase 01](./phase-01-regression-and-contract-lock.md) | Low-Medium | Medium | 2 | 2 | Việc hẹp, unblock nhanh; hợp để chốt sạch regression trước khi tăng tốc các phase sau. |
| 10 | [Phase 08](./phase-08-config-foundation.md) | Low | Medium | 1 | 1 | Contract freeze cho config; scope nhỏ hơn propagation phase sau. |

## Phase Ranking By Staffing Efficiency

| Rank | Phase | Nên nhét người vào không | Lý do |
| --- | --- | --- | --- |
| 1 | [Phase 03](./phase-03-consumer-cutover-parallel.md) | Rất nên | 3 lane tách file rõ, verification tương đối rõ, ít tranh chấp. |
| 2 | [Phase 07](./phase-07-pty-owner-migration-parallel.md) | Nên, nhưng phải có lead | 2 lane tách file tốt, nhưng contract hook đầu phase phải khóa trước. |
| 3 | [Phase 06](./phase-06-runtimehost-wiring-parallel.md) | Nên | Desktop và Lua tách tốt sau khi foundation phase 05 xong. |
| 4 | [Phase 09](./phase-09-config-propagation-parallel.md) | Nên | 2 lane đủ độc lập, nhưng cần grep/audit cuối phase. |
| 5 | [Phase 04](./phase-04-ui-and-desktop-facade-parallel.md) | Tương đối nên | 2 lane, risk vừa phải. |
| 6 | [Phase 01](./phase-01-regression-and-contract-lock.md) | Có thể | Chủ yếu để tăng tốc mở khóa đầu plan. |
| 7 | [Phase 02](./phase-02-host-runtime-api-foundation.md) | Không | Foundation file overlap nặng; thêm người chỉ tăng merge conflict. |
| 8 | [Phase 05](./phase-05-control-plane-foundation.md) | Không | Một cụm lõi duy nhất, cần một owner. |
| 9 | [Phase 08](./phase-08-config-foundation.md) | Không | Freeze contract, scope nhỏ. |
| 10 | [Phase 10](./phase-10-final-rename-and-sweep.md) | Không | Rename sweep diện rộng, nên một người làm để tránh nổ conflict. |

## Lane Ranking By Workload

| Rank | Lane | Parent Phase | Workload | Risk | Suggested Owner | Ghi chú phân công |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `07B Session Engine Native PTY Path` | [Phase 07](./phase-07-pty-owner-migration-parallel.md) | Very High | Very High | Senior runtime/desktop | Nặng nhất ở phía desktop owner thật; đòi hiểu session engine sâu. |
| 2 | `07A Host Runtime Compat PTY Path` | [Phase 07](./phase-07-pty-owner-migration-parallel.md) | Very High | Very High | Senior host-runtime | Cần ghép chuẩn với 07B; chỉ giao cho người hiểu compat path. |
| 3 | `05A Control Plane And Singleton Extraction Foundation` | [Phase 05](./phase-05-control-plane-foundation.md) | Very High | Very High | Tech lead duy nhất | Không chia nhỏ an toàn. |
| 4 | `06A Desktop Explicit Wiring` | [Phase 06](./phase-06-runtimehost-wiring-parallel.md) | High | Very High | Senior desktop | Breadth file lớn hơn 06B. |
| 5 | `02A Host Runtime Surface Hardening` | [Phase 02](./phase-02-host-runtime-api-foundation.md) | High | Very High | Tech lead duy nhất | Public contract choke point. |
| 6 | `09B Host Runtime Config Propagation` | [Phase 09](./phase-09-config-propagation-parallel.md) | High | High | Senior host-runtime | Config read-loop edge dễ sót. |
| 7 | `03A Lua Bridge Cutover` | [Phase 03](./phase-03-consumer-cutover-parallel.md) | High | High | Senior Lua/runtime | Scope khá rộng nhưng lane sạch. |
| 8 | `03B Desktop Host Adapter Cutover` | [Phase 03](./phase-03-consumer-cutover-parallel.md) | Medium-High | High | Senior desktop | Callsite nhiều nhưng ownership rõ. |
| 9 | `06B Lua Explicit Wiring` | [Phase 06](./phase-06-runtimehost-wiring-parallel.md) | Medium-High | High | Senior Lua/runtime | Phụ thuộc phase 05 nhưng file tách tốt. |
| 10 | `09A Desktop Config Propagation` | [Phase 09](./phase-09-config-propagation-parallel.md) | Medium-High | Medium-High | Mid/Senior desktop | Nhiều file desktop nhưng logic rải đều. |
| 11 | `04A TermWindow And Overlay Cleanup` | [Phase 04](./phase-04-ui-and-desktop-facade-parallel.md) | Medium | Medium-High | Mid/Senior UI-desktop | Typed handle cleanup, cần smoke UI kỹ. |
| 12 | `10A Rename Engine Crates And Final Sweep` | [Phase 10](./phase-10-final-rename-and-sweep.md) | Medium | High | Release owner duy nhất | Không khó về logic, khó ở breadth và conflict. |
| 13 | `04B Desktop Facade Cleanup` | [Phase 04](./phase-04-ui-and-desktop-facade-parallel.md) | Medium | Medium | Mid/Senior desktop | Cleanup/facade scope vừa. |
| 14 | `03C Config Dead Sweep` | [Phase 03](./phase-03-consumer-cutover-parallel.md) | Low-Medium | Medium | Mid config owner | Lane phụ, thích hợp cho người thứ 3 trong wave 03. |
| 15 | `01A Startup Env Regression` | [Phase 01](./phase-01-regression-and-contract-lock.md) | Low-Medium | Medium | Mid/Senior | Nhanh, unblock sớm. |
| 16 | `01B Lua Active Session Contract` | [Phase 01](./phase-01-regression-and-contract-lock.md) | Low-Medium | Medium | Mid/Senior | Hẹp hơn 01A một chút. |
| 17 | `08A Config API Foundation` | [Phase 08](./phase-08-config-foundation.md) | Low | Medium | Một owner | Scope nhỏ, nên làm gọn nhanh. |

## Suggested Assignment Strategy

### Nếu có 3 người
- Người 1: giữ toàn bộ foundation phases `02`, `05`, `08`, `10`
- Người 2: desktop-heavy lanes `03B`, `04A`, `04B`, `06A`, `09A`
- Người 3: runtime/lua-heavy lanes `01B`, `03A`, `06B`, `07A`, `07B`, `09B`

### Nếu có 5 người
- Người 1: `02A`, `05A`, `08A`, `10A`
- Người 2: `03B`, `04B`, `06A`
- Người 3: `03A`, `06B`
- Người 4: `04A`, `09A`
- Người 5: `07A`, `07B`, `09B`

### Nếu có 7 người trở lên
- Không tăng người vào `02A`, `05A`, `08A`, `10A`
- Dồn thêm người review/smoke/grep audit cho `06`, `07`, `09`
- Chỉ scale ngang ở các phase có lane tách file thật: `01`, `03`, `04`, `06`, `07`, `09`

## Practical Rule
- `Foundation phase`: 1 owner mạnh + 1 reviewer, không phải 2 implementer.
- `Parallel phase`: mỗi lane 1 owner rõ ràng; nếu cần người phụ, người phụ chỉ làm test/smoke/docs cho lane đó.
- `Rename/final sweep`: khóa merge queue trước khi làm.
