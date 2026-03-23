# UI Shell Surface Map

Date: 2026-03-23  
Scope: desktop UI shell only. Repo-only read. No terminal core proposal.

## High-signal findings
- Product shell ownership sits in `apps/chatminal-desktop/src/termwindow/*`, `apps/chatminal-desktop/src/desktop_termwindow_*`, `apps/chatminal-desktop/src/tabbar.rs`, `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`, `apps/chatminal-desktop/src/overlay/*`, `apps/chatminal-desktop/src/chatminal_layout/*`, `apps/chatminal-desktop/src/chatminal_render/*`.
- `docs/system-architecture.md` is explicit: `termwindow/*` and `desktop_termwindow_*` are render/input shell, not business source of truth. Good fit for this roadmap.
- `crates/chatminal-terminal-core/*` is parser/state core. Wrong layer for sidebar/session bar/footer/overlay/layout polish.
- Workspace/session geometry already flows through desktop facade and `WorkspaceLayoutState`; shell work can stay app-local if no new runtime contract is introduced.

## Surface map
- Sidebar state + sync: `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- Sidebar render tree + footer: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Session bar model/render: `apps/chatminal-desktop/src/tabbar.rs`
- Shell geometry padding/chrome/footer height: `apps/chatminal-desktop/src/termwindow/mod.rs`, `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`, `apps/chatminal-desktop/src/termwindow/resize.rs`
- Layout primitives and split hit zones: `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`, `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
- Overlay lifecycle: `apps/chatminal-desktop/src/overlay/mod.rs`, `apps/chatminal-desktop/src/overlay/launcher.rs`, `apps/chatminal-desktop/src/overlay/prompt.rs`, `apps/chatminal-desktop/src/overlay/confirm*.rs`
- Input glue / hit-testing / wheel / hover: `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Paint/invalidation budget for lightweight animation: `apps/chatminal-desktop/src/termwindow/render/paint.rs`, `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`, `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`

## Candidate files likely touched
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/termwindow/resize.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/overlay/mod.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `apps/chatminal-desktop/src/overlay/prompt.rs`
- `apps/chatminal-desktop/src/overlay/confirm.rs`
- `apps/chatminal-desktop/src/overlay/confirm_close_pane.rs`
- `apps/chatminal-desktop/src/termwindow/render/paint.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`

## Coupling risks
- `termwindow/mod.rs` is large and central; shell polish can sprawl if primitives are not extracted narrowly.
- `desktop_termwindow_mouseevent.rs` mixes terminal input path with UI hit zones; avoid regressions by isolating sidebar/session bar/footer branches.
- `overlay/*` shares pane/render-scope lifecycle. Visual polish is safe; overlay sizing/focus semantics are riskier.
- `desktop_termwindow_layout_render.rs` bridges runtime layout snapshot into paint geometry. Safe for chrome/layout math, unsafe for changing workspace semantics.
- `tabbar.rs` still has Lua/config formatting hooks. Avoid breaking existing `format-tab-title` behavior when restyling.

## Guardrails
- Hard no-touch: `crates/chatminal-terminal-core/**`
- Strong no-touch unless separate scope approved: `crates/chatminal-runtime/**`, `crates/chatminal-session-runtime/**`, `apps/chatminal-desktop/src/desktop_host_runtime/**`
- Keep runtime DTOs, session lifecycle, render-target contract, terminal input/scrollback semantics unchanged.
- Prefer app-shell-only refactors in `apps/chatminal-desktop/src/**`.
- If any phase needs new runtime API or terminal behavior change, stop and split scope.

## Unresolved questions
- None from repo read. Main unknown is desired visual language, not technical boundary.
