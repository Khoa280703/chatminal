# Phase 05: GPU State Reuse

## Overview
- **Priority:** MEDIUM — GPU efficiency
- **Status:** pending
- **Effort:** Medium (~2-3 hours)

## Problem

Draw path still rebuilds some GPU-side setup every frame:
- Glium path creates samplers every frame
- Uniform setup repeats identical field adds each draw
- `layers.borrow()` / `layer.vb.borrow()` happen inside hot loops
- WebGPU path recreates bind groups every frame

## Key Insights

- `UniformBuilder` currently only has `add` / `add_struct`; no `.set(...)`
- Therefore plan should not pretend we can mutate-and-reuse one builder object across all draws without changing the API
- Borrow hoisting is low risk and worth doing
- WebGPU bind-group caching is the most concrete reusable state here
- Sampler caching may be blocked by lifetimes and might end up WONTFIX

## Related Code Files

### Modify
- `apps/chatminal-desktop/src/termwindow/render/draw.rs` — draw call, sampler/uniform/bind-group setup
- `apps/chatminal-desktop/src/uniforms.rs` — only if tiny helper support is needed

### Read-only references
- `apps/chatminal-desktop/src/termwindow/webgpu.rs` — WebGPU render state

## Implementation Steps

### Step 1: Hoist borrows outside inner loops

Refactor:
```rust
let layers = gl_state.layers.borrow();
for layer in layers.iter() {
    let vbs = layer.vb.borrow();
    for idx in 0..3 {
        let vb = &vbs[idx];
        ...
    }
}
```

### Step 2: Replace fake UniformBuilder reuse with shared helper code

Instead of imaginary `.set(...)`, keep per-draw builders but centralize common fields:
```rust
fn add_common_uniforms<'a>(builder: &mut UniformBuilder<'a>, ...) {
    builder.add("projection", &projection);
    builder.add("atlas_nearest_sampler", &atlas_nearest_sampler);
    builder.add("atlas_linear_sampler", &atlas_linear_sampler);
    builder.add("foreground_text_hsb", &foreground_text_hsb);
    builder.add("subpixel_aa", &subpixel_aa);
    builder.add("milliseconds", &milliseconds);
    builder.add_struct("cursor_blink", &cursor_blink);
    builder.add_struct("blink", &blink);
    builder.add_struct("rapid_blink", &rapid_blink);
}
```

This is mostly duplication cleanup plus slightly clearer hot path, not a miracle reuse win.

### Step 3: Evaluate sampler caching feasibility

Try only if lifetimes permit. If sampler objects borrow texture in a way that makes caching awkward, document WONTFIX and move on.

### Step 4: Cache WebGPU bind groups by atlas generation

Cache bind groups in WebGPU state and rebuild only when atlas texture/view changes.

## Todo

- [ ] Hoist `layers.borrow()` outside draw loop
- [ ] Hoist `layer.vb.borrow()` outside inner loop
- [ ] Replace fake `UniformBuilder` reuse idea with helper/build-common-fields approach
- [ ] Evaluate sampler caching feasibility
- [ ] Cache WebGPU bind groups with atlas generation tracking

## Success Criteria

- `cargo check -p chatminal-desktop` — 0 errors
- Rendering visually identical
- Reduced WebGPU bind-group churn
- No regression in frame time

## Risk Assessment

- **Low risk:** Borrow hoisting is pure refactor.
- **Low risk:** Uniform helper extraction is straightforward, but perf benefit may be modest.
- **Low risk:** Bind group caching is concrete and localized.

## Verify

```bash
cargo check -p chatminal-desktop
# Visual test: render correctness
# Optional: Instruments/Metal trace compare before/after
```
