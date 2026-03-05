# IPC Compatibility Matrix - Phase 04

Generated: 2026-03-05  
Scope: `window-wezterm-gui` migration, daemon-first ownership unchanged.

## Request/Response Compatibility
| Area | Request(s) | Expected Response/Event | Status | Notes |
| --- | --- | --- | --- | --- |
| Health | `ping` | `response.ping` | pass | Framing + parse covered in client tests |
| Workspace hydrate | `workspace_load` | `response.workspace` | pass | Used by `workspace`, `dashboard`, `window` boot |
| Session create | `session_create` | `response.session_create` | pass | First-run bootstrap path |
| Session activate | `session_activate` | `response.empty` + live events | pass | Used by attach/proxy/window paths |
| Session snapshot | `session_snapshot_get` | `response.session_snapshot` | pass | Reconnect bootstrap |
| Session input | `session_input_write` | `response.empty` | pass | Ctrl+C and interactive typing path |
| Session resize | `session_resize` | `response.empty` | pass | Proxy/window resize loop |
| Session lifecycle | `session_rename`, `session_close`, `session_set_persist` | `response.empty` + `session_updated` | pass | Covered in daemon unit tests |
| History clear | `session_history_clear`, `workspace_history_clear_all` | `response.empty` + state updates | pass | Generation-gate regression tests exist |
| Profile lifecycle | `profile_list/create/rename/delete/switch` | `response.profiles/profile/empty` + `workspace_updated` | pass | Used by dashboard and workspace views |
| Explorer lifecycle | `session_explorer_*` | explorer responses | pass | Out of scope for window cutover, protocol unchanged |
| App lifecycle | `lifecycle_preferences_get/set`, `app_shutdown` | `response.lifecycle_preferences/empty` | pass | Transport/daemon tests pass |

## Event Compatibility
| Event | Consumer path | Status | Notes |
| --- | --- | --- | --- |
| `pty_output` | proxy + pane adapter + workspace binding | pass | Watermark/order guard in runtime tests |
| `pty_exited` | proxy + workspace binding | pass | Exit tail drain in proxy |
| `pty_error` | proxy stderr + runtime adapter | pass | Non-fatal |
| `session_updated` | workspace binding | pass | Out-of-order seq guard covered |
| `workspace_updated` | workspace binding | pass | Marks stale for safe rehydrate |
| `daemon_health` | runtime observer | pass | Non-mutating |

## Rollback/Kill-switch Contract
- `CHATMINAL_WINDOW_BACKEND=wezterm-gui` (default) -> launcher path (`window-wezterm-gui`).
- `CHATMINAL_WINDOW_BACKEND=legacy` -> legacy embedded egui fallback path.
- Verification script: `scripts/migration/phase08-wezterm-gui-killswitch-verify.sh`.

## Conclusion
- No protocol/schema migration required for this phase.
- Compatibility is additive and backward-safe for current daemon/app pair.
