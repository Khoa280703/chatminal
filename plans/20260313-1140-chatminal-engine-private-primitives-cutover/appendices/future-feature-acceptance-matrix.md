# Future Feature Acceptance Matrix

Purpose: chứng minh sau plan này, feature mới không cần mở thêm refactor nền `Mux/Tab/Pane`.

## Accepted Without New Architecture Refactor
- Create session
  - path: `chatminal-runtime -> session-runtime -> desktop facade`
- Clone session
  - path: `chatminal-runtime` creates new session, workspace attaches new `session_view`
- Group sessions
  - path: mutate `session_group/workspace_layout`, not host leaf tree directly
- Move session between groups
  - path: `workspace_layout/session_group` mutation only
- Focus next/previous session view
  - path: facade lookup + render target switch
- Persist/restore workspace layout
  - path: `chatminal-runtime` source of truth
- Render multiple grouped sessions together
  - path: `termwindow` renders `session_group/layout` snapshots

## Not Allowed After Plan Completes
- Adding product feature by exposing new `Tab`/`Pane` APIs in app-facing layer
- Solving feature gaps by letting `termwindow` own business routing again
- Solving scripting/config gaps by exposing new `host_tab` or `host_leaf` product APIs

## Acceptance Check
For each new feature, answer must be yes:
- Can this feature be described only with `session/session_view/session_group/workspace_layout/render_target`?
- Can the main implementation live outside `desktop_host_runtime`?
- Does it avoid new public host primitive leakage?
