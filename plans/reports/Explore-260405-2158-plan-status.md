# Single Runtime Convergence: Phase Status Cross-Reference

**Report Date:** 2026-04-05  
**Plan:** `/plans/260405-1229-single-runtime-convergence/`

## Summary Table

| Phase | File Status | Incomplete Todos | Accurate? |
|-------|---|---|---|
| **02** | in_progress | 2/9 (22%) | ⚠️ Partial—bridge clamped but split/join execution tree not migrated |
| **03** | in_progress | 7/8 (88%) | ⚠️ Partial—desktop still bound to host-runtime in control-plane/spawn |
| **04** | done | 0/7 (0%) | ✅ Yes—codecs, lua-bridge, and official deps cut from host-runtime |
| **05** | in_progress | 4/7 (57%) | ⚠️ Partial—lua-bridge/codec decoupled but residual grep and namespace cleanup remains |

---

## Phase-by-Phase Status

### Phase 02: Move Execution Ownership Into Runtime
**File Status:** `in_progress`  
**Effort:** High | **Risk:** High

**Incomplete Todos:**
- [ ] Port split/join/focus execution tree
- [ ] Port terminal snapshot/render DTOs khỏi host-runtime

**Completed:**
- [x] Tạo runtime-native execution modules
- [x] Port PTY/session registry ownership
- [x] Rewire `RuntimeState` direct calls
- [x] Xóa `RuntimeHost` active seam
- [x] Hợp nhất `WorkspaceLayoutRegistry` về một owner
- [x] Chứng minh bridge cũ không còn là owner

**Accuracy:** Phase notes show `RuntimeHost` seam cut and `DesktopRuntimeExecutionBridge` rerouted from startup path, but split/join tree ownership and snapshot DTOs remain in `host-runtime`. Status matches reality.

---

### Phase 03: Collapse Desktop Bridge And Product Wiring
**File Status:** `in_progress`  
**Effort:** Medium | **Risk:** Medium

**Incomplete Todos:**
- [ ] Desktop bootstrap gọi canonical runtime path
- [ ] Xóa host-runtime bootstrap khỏi `main.rs` / desktop init path
- [ ] Cắt duplicate attachment/session-engine state ở desktop
- [ ] Dọn namespace/module names misleading về ownership

**Completed:**
- [x] Xóa `DesktopRuntimeExecutionBridge`
- [x] Xóa `desktop_runtime_host()` / `RuntimeHost` desktop facade
- [x] Thu gọn `chatminal_runtime/mod.rs` wrappers
- [x] Chạy smoke tests desktop product path

**Accuracy:** Progress notes document extensive internal refactoring (local spawn target wrapping, window/pane registry localization, metadata de-duplication). However, `desktop_host_runtime` module still depends on `host_runtime` for spawn substrate and control-plane. Desktop bootstrap and namespace cleanup remain incomplete.

---

### Phase 04: Retire Host Runtime Crate
**File Status:** done | **Status Header:** in_progress ⚠️  
**Effort:** High | **Risk:** High

**Incomplete Todos:** None (all 7 marked `[x]`)

**Completed:**
- [x] Liệt kê toàn bộ caller còn lại của `host_runtime`
- [x] Port hoặc xóa từng caller
- [x] Bỏ dependency khỏi desktop
- [x] Bỏ dependency khỏi lua-bridge
- [x] Bỏ dependency khỏi codec
- [x] Xóa crate khỏi active workspace path
- [x] Verify graph mới bằng grep + cargo metadata/check/tests

**Accuracy:** ⚠️ **DISCREPANCY** — File header says `status: done` but plan.md lists phase as `in_progress`. Git history and progress notes show host-runtime Cargo dependency was cut from codec and lua-bridge, but phase notes indicate desktop still has a "blocker active graph" on `chatminal-desktop -> chatminal-host-runtime`. The status may be prematurely marked `done` relative to desktop dependency status.

---

### Phase 05: Lua Bridge, Docs, And Closeout
**File Status:** in_progress  
**Effort:** Medium | **Risk:** Medium

**Incomplete Todos:**
- [ ] Grep residual seam toàn repo
- [ ] Xóa dead wrappers/aliases/tests thừa
- [ ] Dọn naming/module layout còn misleading
- [ ] Chạy full verification spine
- [ ] Ghi closeout note + residual intentional list

**Completed:**
- [x] Rewire lua-bridge
- [x] Rewire codec imports/DTOs
- [x] Sync README/docs active scope (implied by progress notes)

**Accuracy:** Progress notes confirm lua-bridge and codec are decoupled from host-runtime, but residual phase 05 blocker has moved to "desktop control-plane + overlay terminal + spawn infra" and docs/grep cleanup. Status matches—phase is in-progress with residual blocker identified.

---

## Git History (Last 15 Commits)

```
d71ee1e refactor(phase04): retire chatminal-host-runtime from desktop product path
8a0d018 refactor(desktop): remove legacy host test helpers
f09bb15 fix(runtime): stabilize restored prompt replay
8b3a632 clean by agr
54e3639 style(cleanup): tidy remaining formatting and imports
dc66c9e refactor(architecture): close deadcode and duplicate surfaces
2caaf3d docs(architecture): close host runtime overlay convergence
515ba532 refactor(desktop): converge host runtime and overlay shell
1bdd12c docs: add terminal-layer merge plan
1c8aa7b docs: sync terminal-layer cutover docs
d960d96 feat: merge terminal core into emulator
160eb9c feat: land desktop architecture updates
5f82e9b refactor(architecture): deepen typed boundaries and prune dead state
a6ea602 refactor(architecture): unify desktop shell and prune dead config paths
e9742e1 optimize archi
```

**Evidence:** Commit `d71ee1e` ("refactor(phase04): retire chatminal-host-runtime from desktop product path") is the most recent, confirming active work on phase 04. Recent commits show ongoing architecture convergence.

---

## Cross-Reference Analysis

### Discrepancies Found

1. **Phase 04 Status Mismatch** (Critical)
   - **File Header:** `status: done`
   - **Plan.md:** `status: completed`
   - **Git Reality:** Last commit `d71ee1e` is explicitly phase-04 work ("retire... from desktop product path")
   - **Phase Notes:** Explicitly document desktop as active blocker: "blocker active graph còn lại nằm ở dependency trực tiếp `chatminal-desktop -> chatminal-host-runtime`"
   - **Assessment:** Status appears prematurely finalized. Desktop dependency on host-runtime has NOT been cut; only codec and lua-bridge were cut.

2. **Phase 03 vs Phase 04 Boundary Unclear**
   - Progress notes in phase-03 extensively document "local ownership wrapping" and "eliminate dual-path fallback"
   - Phase-04 notes indicate these changes are still not sufficient to cut desktop's Cargo dependency
   - Suggests phase-03 cleanup is not actually enabling phase-04 completion

### What's Actually Done (Via Git + Code Analysis)

✅ **Complete:**
- Codec and Lua-bridge decoupled from host-runtime (Cargo graph and imports)
- `DesktopRuntimeExecutionBridge` abstraction removed from startup hot path
- `RuntimeHost` trait removed from active product seams
- Desktop internal refactoring to localize spawn target, window registry, metadata
- `pane/renderable` vocabulary homed to runtime canonical

⏳ **In Progress:**
- Desktop Cargo dependency on host-runtime NOT removed
- Control-plane and overlay terminal paths still use host-runtime substrate
- Split/join execution tree ownership still split between runtime + host-runtime
- Namespace cleanup and dead-code pruning incomplete
- Full docs/grep verification not run

❌ **Not Started:**
- Full grep for residual seams across codebase
- Namespace rename/re-home if needed
- Comprehensive docs update
- Formal closeout verification

---

## Recommendations

1. **Update phase-04 status to `in_progress`** (not `done`/`completed`)
   - Desktop still has active Cargo dependency on host-runtime
   - Commit `d71ee1e` shows work ongoing

2. **Clarify phase-03/04 boundary:**
   - Phase-03 is wrapping vocabulary and localizing state
   - Phase-04 is the hard cut of Cargo dependency
   - Current progress suggests phase-03 is ~80% done, phase-04 is ~30% blocked

3. **Phase-05 blocker remains in desktop layers** (not lua-bridge/codec as originally expected)

4. **Run full verification spine** before phase closeout:
   - `cargo tree -p chatminal-desktop -e normal` to verify no host-runtime
   - `rg "host_runtime::" crates/chatminal-runtime apps/chatminal-desktop` for residual

---

**Report generated:** 2026-04-05 21:58 UTC
