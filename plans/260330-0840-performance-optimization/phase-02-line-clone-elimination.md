# Phase 02: Render Line Clone Elimination

## Overview
- **Priority:** HIGH — hot path optimization
- **Status:** pending
- **Effort:** Medium-High (~3-4 hours)

## Problem

`build_line_element_shape` trong screen_line.rs vẫn clone data nặng trên hot path:
- composing-path line clone đã là conditional hiện tại, nên không còn là issue chính
- `ClusterStyleCache` clone vẫn xảy ra per cluster nhưng tương đối nhỏ
- `cluster.clone()` mới là clone nóng thật sự vì `LineToElementShape` đang giữ nguyên `CellCluster`
- Khi render terminal nhiều clusters, clone `CellCluster` lặp lại rất tốn

## Key Insights

- `params.line.clone()` đã chỉ xảy ra khi IME composing active; phase này không nên tốn effort cho tối ưu đã có sẵn
- Downstream render hiện vẫn cần nhiều field từ cluster: `attrs`, `width`, `first_cell_idx`, `images`, `vertical_align`
- Vì vậy hướng `cluster_idx + cell_range` thuần túy là không đủ
- Hướng sạch hơn là giữ shared backing storage cho clusters, còn shape chỉ giữ index

## Related Code Files

### Modify
- `apps/chatminal-desktop/src/termwindow/render/screen_line.rs` — line rendering hot path
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs` — `LineToElementShape` / cache item structs

### Read-only references
- `crates/chatminal-engine-surface/src/` — `CellCluster` definition
- `apps/chatminal-desktop/src/termwindow/render/screen_line.rs` — downstream shape consumers

## Implementation Steps

### Step 1: Keep composing-path optimization as-is

Do not rewrite the conditional line clone unless another change forces touching it. It is already gated on composing state.

### Step 2: Replace per-shape `cluster.clone()` with shared cluster backing

Refactor cache payload:
```rust
pub struct LineToElementShapeItem {
    pub shaped: Rc<Vec<LineToElementShape>>,
    pub clusters: Rc<Vec<CellCluster>>,
    ...
}

pub struct LineToElementShape {
    pub cluster_idx: usize,
    ...
}
```

During render:
```rust
let cluster = &clusters[item.cluster_idx];
```

### Step 3: Update render consumers to resolve cluster by index

Consumers currently read:
- `cluster.attrs`
- `cluster.width`
- `cluster.first_cell_idx`
- `cluster.attrs.images()`
- `cluster.attrs.vertical_align()`

Move all these reads through shared backing storage instead of cloning cluster into every shaped item.

### Step 4: Re-measure before touching `ClusterStyleCache`

Only if cluster-sharing still leaves measurable clone cost:
- consider `Rc<ClusterStyleCache>`
- otherwise keep current form

### Step 5: Keep lightweight `CellAttributes` extraction out-of-scope initially

Do not add a new render-style abstraction unless measurement proves current attrs handling is still a major hotspot after Step 2.

## Todo

- [ ] Confirm composing-path clone is already conditional and leave it alone
- [ ] Add shared cluster backing to `LineToElementShapeItem`
- [ ] Replace `cluster: CellCluster` with `cluster_idx: usize` in `LineToElementShape`
- [ ] Update all shape consumers to read cluster via shared backing storage
- [ ] Re-measure before touching `ClusterStyleCache`
- [ ] Keep CellAttributes extraction out unless profiling proves need

## Success Criteria

- `cargo check -p chatminal-desktop` — 0 errors
- Terminal renders correctly: text, colors, underlines, cursor, selection, IME composition
- Idle frame allocations reduced materially in line render path
- Fast scroll stays smooth

## Risk Assessment

- **High risk:** `LineToElementShape` consumers rely on richer cluster data than the initial audit assumed. Mitigation: shared cluster backing instead of over-pruning shape data.
- **Medium risk:** Shape cache item layout changes. Mitigation: compiler-driven updates plus visual verification.
- **Low risk:** Composing path correctness unchanged because it is explicitly not rewritten.

## Verify

```bash
cargo check -p chatminal-desktop
# Visual test: type text, scroll, select, IME input, verify render correct
```
