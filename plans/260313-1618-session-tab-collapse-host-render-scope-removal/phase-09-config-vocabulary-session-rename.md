---
title: "Phase 09 — Config Vocabulary: Tab/Pane → Session rename"
status: completed
priority: P2
effort: 1d
blocked_by: Phase 08
completed: 2026-03-16
---

# Phase 09 — Config Vocabulary Rename

## Goal

Xóa toàn bộ "Tab" và "Pane" vocabulary còn sót trong public config API và internal code. Sau phase này user config Lua dùng `SpawnSession`, `ActivateSession`, `CloseCurrentSession`, v.v. — không còn `SpawnTab`, `ActivateTab`.

## Scope

3 categories:
- **A**: KeyAssignment "Tab" variants → "Session"
- **B**: KeyAssignment "Pane" split variants → "Session"
- **C**: `tab_bar` config options → `session_bar`

---

## Category A — KeyAssignment Tab → Session

**File: `crates/chatminal-config/src/keyassignment.rs`**

| Trước | Sau |
|-------|-----|
| `SpawnTabTarget` (enum) | `SpawnSessionTarget` |
| `SpawnTabTarget::CurrentPaneTarget` | `SpawnSessionTarget::CurrentSessionTarget` |
| `SpawnTabTarget::DefaultTarget` | `SpawnSessionTarget::DefaultTarget` (giữ) |
| `SpawnTabTarget::TargetName(String)` | `SpawnSessionTarget::TargetName(String)` (giữ) |
| `SpawnTabTarget::TargetId(usize)` | `SpawnSessionTarget::TargetId(usize)` (giữ) |
| `SpawnCommand.target: SpawnTabTarget` | `.target: SpawnSessionTarget` |
| `KeyAssignment::SpawnTab(SpawnTabTarget)` | `SpawnSession(SpawnSessionTarget)` |
| `KeyAssignment::SpawnCommandInNewTab(SpawnCommand)` | `SpawnCommandInNewSession(SpawnCommand)` |
| `KeyAssignment::ActivateTab(isize)` | `ActivateSession(isize)` |
| `KeyAssignment::ActivateTabRelative(isize)` | `ActivateSessionRelative(isize)` |
| `KeyAssignment::ActivateTabRelativeNoWrap(isize)` | `ActivateSessionRelativeNoWrap(isize)` |
| `KeyAssignment::ActivateLastTab` | `ActivateLastSession` |
| `KeyAssignment::CloseCurrentTab { confirm }` | `CloseCurrentSession { confirm }` |
| `KeyAssignment::MoveTab(isize)` | `MoveSession(isize)` |
| `KeyAssignment::MoveTabRelative(isize)` | `MoveSessionRelative(isize)` |
| `KeyAssignment::DetachTarget(SpawnTabTarget)` | `DetachTarget(SpawnSessionTarget)` |
| `PaneSelectMode::MoveToNewTab` | `MoveToNewSession` |

**Cascade (all files importing SpawnTabTarget/KeyAssignment Tab variants):**
- `crates/chatminal-lua-bridge/src/leaf.rs`
- `crates/chatminal-lua-bridge/src/lib.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_commands.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_spawn.rs`
- `apps/chatminal-desktop/src/desktop_mouse_actions.rs`
- `apps/chatminal-desktop/src/frontend.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `apps/chatminal-desktop/src/spawn.rs` (SpawnWhere::NewTab → NewSession)

---

## Category B — KeyAssignment Pane → Session (split context)

**File: `crates/chatminal-config/src/keyassignment.rs`**

| Trước | Sau |
|-------|-----|
| `PaneDirection` (enum) | `SessionDirection` |
| `KeyAssignment::SplitPane(SplitPane)` | `SplitSession(SplitSession)` |
| `KeyAssignment::CloseCurrentPane { confirm }` | `CloseCurrentSession { confirm }` → **MERGE** với `CloseCurrentTab` renamed |
| `KeyAssignment::AdjustPaneSize(PaneDirection, usize)` | `AdjustSplitSize(SessionDirection, usize)` |
| `KeyAssignment::ActivatePaneDirection(PaneDirection)` | `ActivateSessionDirection(SessionDirection)` |
| `KeyAssignment::PaneSelect(PaneSelectArguments)` | `SessionSelect(SessionSelectArguments)` |
| `PaneSelectArguments` struct | `SessionSelectArguments` |
| `SplitPane` struct | `SplitSession` struct |
| `SplitPane::direction: PaneDirection` | `SplitSession::direction: SessionDirection` |

**Merge decision**: `CloseCurrentPane` và `CloseCurrentTab` đều thành `CloseCurrentSession`.
- Giữ lại `#[deprecated]` alias `CloseCurrentPane` → forward đến `CloseCurrentSession` trong 1 version.
- Xóa `CloseCurrentTab` variant cũ, chỉ còn `CloseCurrentSession`.

**Cascade:**
- `apps/chatminal-desktop/src/desktop_commands.rs` (chính — nhiều nhất)
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/termwindow/paneselect.rs`
- `apps/chatminal-desktop/src/termwindow/keyevent.rs`
- `crates/chatminal-lua-bridge/src/leaf.rs`

---

## Category C — `tab_bar` config → `session_bar`

**File: `crates/chatminal-config/src/config.rs`**

| Trước | Sau |
|-------|-----|
| `enable_tab_bar: bool` | `enable_session_bar: bool` |
| `use_fancy_tab_bar: bool` | `use_fancy_session_bar: bool` |
| `tab_bar_at_bottom: bool` | `session_bar_at_bottom: bool` |
| `show_tab_index_in_tab_bar: bool` | `show_session_index_in_session_bar: bool` |
| `show_tabs_in_tab_bar: bool` | `show_sessions_in_session_bar: bool` |
| `show_new_tab_button_in_tab_bar: bool` | `show_new_session_button_in_session_bar: bool` |
| `hide_tab_bar_if_only_one_tab: bool` | `hide_session_bar_if_only_one_session: bool` |
| `tab_bar_style: TabBarStyle` | `session_bar_style: SessionBarStyle` |
| `TabBarColors` (struct) | `SessionBarColors` |
| `TabBarStyle` (struct) | `SessionBarStyle` |

**Cascade:**
- `apps/chatminal-desktop/src/tabbar.rs` (rename field usages)
- `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`
- `apps/chatminal-desktop/src/termwindow/render/fancy_tab_bar.rs`

---

## Category D — Lua public API: Leaf/HandySplit rename

**File: `crates/chatminal-lua-bridge/src/leaf.rs` + `lib.rs`**

| Trước | Sau | Lý do |
|-------|-----|-------|
| `LeafRef` (public struct) | `TerminalRef` | User Lua script dùng `LeafRef` — "Leaf" là wezterm term cho terminal slot. `TerminalRef` maps với `terminal_instance_id`. |
| `HandySplitDirection` (internal enum) | `SessionSplitDirection` | Tên non-standard, dùng trong `spawn_impl`. |

**Cascade `LeafRef → TerminalRef`:**
- `leaf.rs`: struct def, impl blocks, method signatures, `Ok(TerminalRef(pane.pane_id()))` returns
- `lib.rs`: `pub use leaf::TerminalRef`, spawn return tuples `(SessionRef, TerminalRef, WindowRef)`, collect calls
- Lua user API trả về: `local session, terminal, window = chatminal.spawn(...)`

**Note:** `LeafRef` → `TerminalRef` là **breaking change** cho user Lua config. Thêm deprecated type alias `LeafRef = TerminalRef` để backward compat 1 version.

---

## Category E — Internal UI/spawn naming

| File | Trước | Sau |
|------|-------|-----|
| `apps/chatminal-desktop/src/tabbar.rs:36` | `SessionBarItem::NewTabButton` | `SessionBarItem::NewSessionButton` |
| `apps/chatminal-desktop/src/desktop_spawn.rs:13` | `SpawnWhere::NewTab` | `SpawnWhere::NewSession` |
| `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` | `HostLauncherTabEntry` → `LauncherSessionEntry`, `host_launcher_tabs` → `launcher_sessions`, `tab_idx` → `session_idx` |
| `apps/chatminal-desktop/src/overlay/launcher.rs` | `LauncherTabEntry` type alias → `LauncherSessionEntry` |

---

## Implementation approach

**Do NOT rename file `tabbar.rs`** — low priority, internal only. File rename = risky (Rust mod declarations), not worth it.

### Step-by-step

1. Start với `crates/chatminal-config/src/keyassignment.rs`:
   - Rename tất cả Cat A + Cat B types/variants
   - Merge `CloseCurrentPane` + `CloseCurrentTab` → `CloseCurrentSession` với deprecated alias
   - `cargo check -p chatminal-config` pass

2. Rename `TabBarColors`, `TabBarStyle` trong `crates/chatminal-config/src/config.rs` (Cat C)
   - `cargo check -p chatminal-config` pass

3. Fix cascade: `crates/chatminal-lua-bridge/` (4 usages)
   - `cargo check -p chatminal-lua-bridge` pass

4. Fix cascade: `apps/chatminal-desktop/src/` (bulk — dùng `sed` hoặc IDE rename)
   - Fix `chatminal_runtime/mod.rs`, `desktop_commands.rs`, `frontend.rs`, `desktop_spawn.rs`, `overlay/launcher.rs`, `spawn.rs`, `desktop_mouse_actions.rs`, `tabbar.rs`, render files
   - `cargo check -p chatminal-desktop` pass

5. Rename internal: `HostLauncherTabEntry` → `LauncherSessionEntry` (trong Phase 06 đã migrate implementation, phase này chỉ rename type)

6. `cargo check --workspace --all-targets` pass

7. `cargo test --workspace` pass

---

## Grep gate (sau phase này phải về 0 trong chatminal code)

```bash
grep -rn \
  "SpawnTab\b\|ActivateTab\b\|CloseCurrentTab\|MoveTab\b\|ActivateLastTab\|SpawnTabTarget\|DetachTarget.*Tab\|PaneSelectMode::Move.*Tab\|CloseCurrentPane\|SplitPane\b\|AdjustPaneSize\|ActivatePaneDirection\|PaneSelect\b\|PaneDirection\b\|enable_tab_bar\|use_fancy_tab_bar\|tab_bar_at_bottom\|hide_tab_bar\|show_tab.*tab_bar\|TabBarColors\|TabBarStyle\|HostLauncherTabEntry\|host_launcher_tabs\b\|SpawnWhere::NewTab\|NewTabButton\|LeafRef\b\|HandySplitDirection" \
  crates/chatminal-config/ crates/chatminal-lua-bridge/ crates/chatminal-runtime/ \
  apps/chatminal-desktop/src/ \
  --include="*.rs" \
  | grep -v "third_party\|// deprecated\|#\[deprecated\]\|= LeafRef\|type LeafRef"
```
Expected: 0 results (ngoài deprecated aliases).

---

## Success criteria

- Grep gate = 0 results
- `cargo check --workspace --all-targets` pass
- `cargo test --workspace` pass
- User Lua config dùng `SpawnSession`, `ActivateSession`, `CloseCurrentSession`, `TerminalRef` — không còn Tab/Pane/Leaf vocabulary
- Deprecated aliases (`CloseCurrentPane`, `LeafRef`, `SpawnTab`) present với `#[deprecated]` annotation

## Risk

- `desktop_commands.rs` có ~50 Tab/Pane references — lớn nhất. Làm từng nhóm, check sau mỗi nhóm.
- Lua bridge: test scripting path sau rename để đảm bảo Lua types map đúng.
- `CloseCurrentPane` deprecated alias: giữ 1 version để không break existing config files.
