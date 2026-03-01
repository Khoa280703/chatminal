# Terminal Multiplexer Architecture Research Report

## 1. ZELLIJ ARCHITECTURE

**PTY Management:**
- Plugin-based system (WebAssembly) handling pane rendering
- Server-client architecture (similar to tmux model)
- No explicit zero-copy implementation described; plugins handle rendering via STDOUT

**Rendering Approach:**
- Plugin renders output to STDOUT (string-serialized)
- Server converts plugin output to terminal display
- Floating/stacked panes via layout engine
- No GPU acceleration

**Key Trade-off:** Simplicity over native performance; relies on plugin system rather than core complexity

---

## 2. WEZTERM ARCHITECTURE

**PTY Management:**
- Uses `portable-pty` crate (custom Rust implementation)
- Master-slave PTY pairs per pane
- Hierarchical state: Workspaces → Tabs → Panes (each pane = independent PTY)

**Rendering:**
- GPU-accelerated (OpenGL)
- Cell buffer stores terminal state (character, color, formatting)
- Full-screen redraw optimization (like Alacritty)
- Window resize handled directly on PTY

**Threading Model:**
- Async I/O from PTY to cell buffer
- GPU rendering decoupled from PTY I/O
- Workspace/Tab/Pane state machine managed centrally

---

## 3. ALACRITTY ARCHITECTURE

**GPU Rendering (Most Advanced):**
- OpenGL-based full-screen redraw every frame
- Glyph atlas: characters rasterized once → cached in texture
- 2 draw calls per frame (massive optimization)
- ~500 fps achievable (but only redraws on state change = power-efficient)

**Cell Buffer:**
- Holds: character + color + formatting per cell
- Not optimized for zero-copy (copies from PTY to buffer acceptable)
- VTE parser handles ANSI escape sequences

**Design Philosophy:**
- No tabs/panes built-in (delegate to tmux, window manager)
- Intentionally simple; prioritizes single-pane performance

---

## 4. KITTY TERMINAL

**Pane Implementation:**
- 7 preset layouts (Fat, Grid, Horizontal, Splits, Stack, Tall, Vertical)
- Auto-layout engine handles pane resizing
- OpenGL rendering (like Alacritty/WezTerm)

**Language Mix:**
- C: performance-critical paths
- Python: UI logic, configuration
- Go: small CLI extensions ("kittens")

**Multiplexing:**
- Built-in tab/pane support (unlike Alacritty)
- Remote control API for SSH-based operations

---

## 5. TMUX (CLI REFERENCE)

**Model for GUI Apps:**
- Client-server architecture (separate concerns)
- Session → Window → Pane hierarchy
- One PTY per pane, server manages I/O distribution
- No GUI, but architecture solid for translation

**Threading:**
- Server event loop handles multiple clients
- Each pane = separate process (not multi-threaded within server)
- IPC via Unix sockets

---

## 6. MESSENGER-LIKE TERMINAL UI PATTERNS

**Session Sidebar (Chat List):**
- Render pane list in narrow left panel
- Highlight active pane
- Virtual scrolling if >100 panes (list only visible items)

**Terminal Output Panel (Conversation):**
- Cell buffer backing store
- Viewport over buffer (allows scrollback)
- Render only visible rows + context (virtual scrolling)

**Virtual Scrolling for Scrollback:**
- Store full history in ring buffer (circular queue)
- Keep cursor position separate
- Only render current viewport (O(height) complexity, not O(history))

**Zero-Copy PTY Flow:**
- Read from PTY into buffer pool (preallocated)
- Pass &[u8] through channels (no copy)
- VTE parser mutates cell buffer in-place
- GPU reads directly from cell buffer texture

---

## 7. ARCHITECTURAL DECISIONS FOR CHATMINAL

**Thread Model (Recommended):**
- Tokio async runtime for PTY I/O (multiple panes concurrently)
- Separate UI thread (egui/iced/Tauri) handling render loop
- Channel between I/O thread → UI (batched updates)

**Buffer Management:**
- Ring buffer per pane for scrollback (fixed size, configurable)
- Cell buffer (Vec<Cell>) for current viewport
- Dirty flag per cell to avoid unnecessary GPU uploads

**Resize Handling:**
- Signal PTY on window resize via SIGWINCH
- Drop/recreate cell buffers if pane layout changes
- Smooth relayout without process restart

**State Management:**
- Pane state: PTY handle, cell buffer, scrollback, cursor position
- Session state: active pane, layout config, workspaces
- Use enum-based state machine (not ad-hoc boolean flags)

**GPU Rendering:**
- OpenGL/WGPU: Glyph atlas + instancing (Alacritty approach)
- Or: wgpu + existing glyph rasterizer (more portable)
- Redraw on: PTY data available OR user interaction

**Open-Source Rust Examples:**
1. **Alacritty** - Cell buffer + OpenGL reference
2. **WezTerm** - portable-pty + hierarchical pane model
3. **Zellij** - Plugin system (alternative arch)
4. **Kitty** - Built-in multiplexing (C + Python hybrid)

---

## CRITICAL ARCHITECTURAL GAPS

**Unresolved Questions:**
1. How to handle PTY reads across multiple panes without blocking UI?
   - Answer: Tokio select! on multiple PTY readers
2. How large should scrollback buffer be? Impact on memory?
   - Answer: Configurable ring buffer (e.g., 10k lines = ~1MB per pane)
3. Should sidebar be a separate GPU surface or immediate-mode UI?
   - Answer: Immediate-mode (egui) simpler; less perf-critical
4. How to sync pane metadata (name, unread count) with visual state?
   - Answer: State machine; redraw full UI on structural changes

---

**Report Status:** Complete  
**Date:** 2026-03-01  
**Next:** Architect detailed phase plan for implementation
