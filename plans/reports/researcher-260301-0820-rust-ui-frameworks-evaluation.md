# Rust Native UI Frameworks: Desktop Messenger-Like App Analysis

**Date:** 2026-03-01 | **Status:** Research Complete

---

## Executive Summary

**RECOMMENDATION: Iced** for production use. Floem as aggressive alternative if cutting-edge perf required.

Both handle messenger-like layouts (sidebar + scrollable panels) with GPU acceleration. Iced matures fastest; Floem more innovative but less stable. GPUI/Vello/Xilem unfit for current deadline. egui/Slint viable but worse fit for terminal cell rendering.

---

## Framework Comparison Matrix

### 1. **Iced** ⭐ RECOMMENDED
**Status:** Experimental but production-proven | **Stars:** 29.7k | **Latest:** v0.14 (Dec 2025)

**Strengths:**
- GPU backend: wgpu (Vulkan/Metal/DX12) + CPU fallback (tiny_skia)
- Custom widgets: Official canvas API + shader support via `iced::widget::shader`
- Virtual scrolling: Built-in scrollable containers, optimized layouts
- Active dev: 6,655 commits, 6.1k dependents, Kraken-sponsored
- Elm-like architecture: Familiar reactive model, easy async PTY integration
- Cross-platform: Win/Mac/Linux native

**Weaknesses:**
- Pre-1.0, breaking changes expected
- Learning curve: Elm architecture unfamiliar to imperative devs
- Renderer abstraction adds complexity for custom terminal cell rendering

**Terminal Cell Integration:** Moderate friction. Need custom widget wrapping terminal cells, but wgpu shader support enables efficient batch rendering.

---

### 2. **Floem** (Lapce Team) ⭐⭐ AGGRESSIVE ALTERNATIVE
**Status:** Pre-1.0 active dev | **Stars:** 4k | **Deps:** 208 projects

**Strengths:**
- GPU rendering: wgpu (vger/vello), CPU fallback (tiny-skia)
- View tree constructed once = no accidental bottlenecks
- Virtual lists built-in for large datasets
- Performance-focused architecture
- Active community (Discord)
- Lower barrier than Iced for custom rendering

**Weaknesses:**
- Smaller ecosystem (208 vs 6.1k dependents)
- Less mature API, fewer examples
- Fewer production apps shipping
- Dependent on experimental vger/vello backends

**Terminal Cell Integration:** Better than Iced. Lower abstraction overhead for custom rendering pipeline.

---

### 3. **GPUI** (Zed Editor)
**Status:** v0.2.2 pre-1.0 | **Available:** macOS/Linux only (no Windows)

**Verdict:** ❌ NOT RECOMMENDED

Reasons:
- Explicitly designed for Zed, secondarily generic
- No Windows support (deal-breaker)
- Learning curve: "Best way to learn is read Zed source code"
- Unstable API, not standalone-friendly
- High maintenance overhead for unfamiliar codebase

---

### 4. **Vello + Xilem** (Linebender)
**Status:** Alpha, not production-ready

**Verdict:** ❌ EXPERIMENTAL

- Vello acknowledged gaps: blur/filters, conflict artifacts, GPU memory strategies, glyph caching
- 177 fps reported but incomplete feature set
- If shipping Sept 2025+, too bleeding-edge
- Viable only if custom 2D vector rendering critical

---

### 5. **egui**
**Status:** Actively developed, experimental | **Powers:** Rerun Viewer

**Verdict:** ⚠️ VIABLE BUT NOT IDEAL

Strengths:
- Immediate mode simplicity
- Fast iteration
- Works web/native
- Proven in production (Rerun)

Weaknesses:
- Immediate mode redraws entire UI each frame (inefficient for large terminal output panels)
- Terminal cell batch rendering awkward with immediate mode
- Limited virtual scrolling optimization
- Less suitable for retained-mode custom rendering pipelines

---

### 6. **Slint**
**Status:** Commercial backing (declarative) | **Licensing:** GPL + permissive options

**Verdict:** ⚠️ VIABLE FOR SIMPLE UIs ONLY

Strengths:
- Declarative syntax (HTML/CSS-like)
- Fast iteration with live preview
- Lightweight footprint (<300KB)
- Multi-platform (desktop/mobile/web)

Weaknesses:
- Limited custom widget flexibility
- Terminal cell rendering = fighting the framework
- Not designed for data-heavy reactive pipelines
- Licensing complexity for open-source projects

---

## Evaluation: Your Specific Needs

| Criterion | Iced | Floem | GPUI | Vello | egui | Slint |
|-----------|------|-------|------|-------|------|-------|
| GPU acceleration | ✅ wgpu | ✅ wgpu | ✅ GPU | ✅ Alpha | ⚠️ varies | ✅ GPU opt |
| Custom terminal cell renderer | ✅ Easy | ✅✅ Easier | ❌ No | ⚠️ Possible | ❌ Hard | ❌ No |
| Virtual scrolling | ✅ Built-in | ✅ Built-in | ✅ UniformList | ❌ No | ⚠️ Workaround | ⚠️ Limited |
| Async PTY integration | ✅ Excellent | ✅ Good | ❌ Zed-centric | N/A | ✅ Good | ⚠️ Awkward |
| Production-ready | ✅ Yes* | ⚠️ Maturing | ❌ No | ❌ No | ✅ Yes | ✅ Yes |
| Linux primary | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| macOS/Windows | ✅ | ✅ | ❌/❌ | ✅ | ✅ | ✅ |
| Ecosystem maturity | ✅✅ | ⚠️ | ❌ | ⚠️ | ✅ | ✅ |

*Iced is "experimental" but 6.1k projects depend on it; stable in practice.

---

## Architecture Recommendation

### If shipping Q2 2025 (soon):
```
PRIMARY: Iced (0.14+)
  → wgpu backend for GPU terminal rendering
  → Custom canvas widget for term cell batch rendering
  → Message-passing for PTY stream integration

FALLBACK: egui (if time critical)
  → Fast iteration, proven rendering
  → Custom immediate-mode cell widget
  → Accept frame repaint overhead for now
```

### If timeline flexible (Q3+):
```
PRIMARY: Floem
  → Lower abstraction overhead
  → More direct GPU control
  → Smaller learning curve for custom rendering

SECONDARY: Iced (safer bet)
```

---

## Integration Pattern (Iced example)

```rust
// Pseudo-pattern for PTY data → UI
pub struct Terminal {
    pty_receiver: mpsc::UnboundedReceiver<TerminalOutput>,
}

impl Program for App {
    type Message = TerminalMsg;
    
    fn update(&mut self, msg: TerminalMsg) -> Command<TerminalMsg> {
        match msg {
            TerminalMsg::PTYOutput(data) => {
                self.terminal.buffer.push(data);
                Command::none()
            }
        }
    }
    
    fn view(&self) -> Element {
        scrollable(
            custom::terminal_renderer(
                self.terminal.buffer,
                gpu_style
            )
        ).into()
    }
}

// Custom widget handles cell rendering via wgpu pipeline
```

---

## Cons by Framework

**Iced:** API volatility, Elm pattern learning curve, abstraction overhead for custom rendering  
**Floem:** Smaller ecosystem, fewer production examples, underdocumented  
**GPUI:** Windows omitted, Zed-centric, unstable API, poor standalone guidance  
**Vello:** Alpha state, incomplete feature set, memory management gaps  
**egui:** Immediate-mode redraws inefficient, weak virtual scrolling, awkward PTY binding  
**Slint:** Terminal rendering feels unnatural, limited widget customization, licensing overhead  

---

## Unresolved Questions

1. How many terminal rows in scrollable viewport? (Affects virtual scrolling complexity)
2. Real-time PTY streaming throughput? (May favor immediate-mode egui if high-volume)
3. Sidebar widget library available? (Iced has more community widgets)
4. Zed integration needed? (Would justify GPUI despite limitations)
5. Windows critical on day-1? (Eliminates GPUI)

