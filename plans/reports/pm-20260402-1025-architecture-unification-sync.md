# Architecture unification sync 2026-04-02 10:25

## Landed in workspace
- Phase 03D/03E boundary cleanup advanced again.
- `chatminal-host-runtime/src/window.rs`
  - snapshot `switch_to_last_active_tab_when_closing_tab` at window construction
  - route window notifications via `notify_mux(...)` helper instead of inline `Mux` access
- `chatminal-lua-bridge/src/window.rs`
  - common root-window getters/setters/session listing now go through `LuaBridgeHost` closure helpers
  - `active_session_id` uses owned session-id helper
  - `active_terminal` uses `TerminalRef::from_pane_id(...)`
- Phase 04 low-risk config cleanup in desktop already present in workspace:
  - `frontend.rs` keeps config snapshot + refreshes on reload
  - `main.rs` threads `ConfigHandle` through desktop startup/serial bootstrap
  - `stats.rs` caches periodic logging config in atomic + reload subscription
  - `customglyph.rs` reuses glyph-config AA decision in hot draw path

## Verification
- `cargo check -p chatminal-lua-bridge` pass
- `cargo check -p chatminal-host-runtime` pass
- `cargo check -p chatminal-desktop` pass
- `cargo test -p chatminal-runtime` pass (`104/104`)
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass (`87/87`)

## Remaining critical path
- 03D still not done: singleton still lives in `chatminal-host-runtime/src/lib.rs`
- 03F still not done: `PaneId`/`TabId` still leak heavily in host-runtime public surface
- Phase 04 still partial: `configuration()` remains in host-runtime PTY/control path and a few desktop bootstrap sites

## Suggested next cuts
1. `crates/chatminal-host-runtime/src/lib.rs`
   - extract control-plane helpers around spawn-target / client-workspace-focus / subscriber notify further away from direct singleton access
2. `crates/chatminal-host-runtime/src/lib.rs` + consumers
   - start replacing public `PaneId`/`TabId` helper surface with narrower wrapped ids or desktop/runtime-native DTOs
3. Desktop/config follow-up
   - leave test-only desktop callsites alone
   - only continue if a non-singleton config source can be threaded without touching PTY read loop yet

## Unresolved questions
- none
