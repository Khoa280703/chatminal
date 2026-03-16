# Final Exit Checklist

Plan is not complete unless every item below is true.

## Architecture
- [ ] App-facing desktop path does not think in `tab = session`
- [ ] `chatminal-runtime` is the only product facade
- [ ] `termwindow` is render/input shell only
- [ ] `desktop_host_runtime` is private adapter only
- [ ] `chatminal-lua-bridge` no longer requires host ids for product scripting

## Vocabulary
- [ ] Product/UI code can be explained with `session/session_view/session_group/workspace_layout/render_target`
- [ ] No new public `Mux/Tab/Pane` leakage exists
- [ ] Deprecated shims from this plan are removed or explicitly time-boxed

## Build/Test/Grep
- [ ] Active build/test gates pass
- [ ] `cargo check --workspace --all-targets` policy is closed, not deferred
- [ ] Forbidden symbol contract passes in intended scopes

## Future Feature Readiness
- [ ] Clone session can be added without exposing host primitives
- [ ] Group/move sessions can be added through layout model only
- [ ] Persist/restore grouped layout can be added through runtime-owned layout state

## Docs
- [ ] `docs/system-architecture.md` matches source
- [ ] `docs/codebase-summary.md` matches source
- [ ] roadmap/changelog reflect final state
