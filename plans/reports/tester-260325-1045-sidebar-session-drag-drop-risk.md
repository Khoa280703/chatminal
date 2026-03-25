# Tester Report

Date: 2026-03-25
Scope: sidebar session drag-drop reorder/move
Focus: same-profile reorder, cross-profile move, joined-group drag, drag/no-drag click behavior

## Commands Run
- `cargo check -p chatminal-desktop` ✅
- `cargo test --manifest-path crates/chatminal-store/Cargo.toml move_sessions_to_profile -- --nocapture` ✅
- `cargo test -p chatminal-runtime sessions_move_to_profile -- --nocapture` ✅
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml chatminal_layout::workspace_store::tests::profile_group_session_ids_returns_full_join_group_for_member_session -- --nocapture` ✅
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml termwindow::render::chatminal_sidebar::tests::joined_session_markers_assigns_visual_positions_per_group -- --nocapture` ✅
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml` ❌

## Build/Test Status
- Store layer green for grouped reorder/move.
- Runtime layer green for grouped reorder path.
- Desktop layout/render helper tests green.
- Full desktop suite not green: `quad::size` fails at `apps/chatminal-desktop/src/quad.rs:502`.

## Findings

### 1. Full desktop suite currently red
- Evidence: `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml`
- Failure: `quad::size` at `apps/chatminal-desktop/src/quad.rs:502`
- Impact: branch is not test-clean yet; cannot claim desktop verification complete even if drag-drop-specific tests pass.

### 2. Cross-profile multi-drag appears to synthesize joined-layout aliases even for plain multi-selection
- Code: `apps/chatminal-desktop/src/termwindow/mod.rs:913-929`
- Logic persists `WorkspaceLayoutState::grouped_sessions(session_ids)` for any cross-profile move where `session_ids.len() > 1`.
- No check that dragged sessions were actually joined before move.
- Impact: dragging 2 separately selected, non-joined sessions across profiles may make target profile restore them as a joined group later; unjoin/join markers can drift from user intent.

### 3. Joined-group drag can reorder the cluster when drag starts from a middle member
- Code: `apps/chatminal-desktop/src/termwindow/mod.rs:813-830`
- `ordered_selected_chatminal_session_ids()` first preserves sidebar order, then swaps the anchor session to index 0.
- Impact: dragging joined group `[A,B,C]` by grabbing `B` likely sends move order `[B,A,C]`; same-profile reorder and cross-profile move may invert cluster order unexpectedly.

## Coverage Present
- Store tests cover:
  - same-profile grouped reorder
  - cross-profile grouped move at index `0`
- Runtime test covers:
  - grouped reorder without losing runtime state
- Desktop tests cover:
  - joined-group lookup from profile layout alias
  - joined marker rendering positions

## Important Manual Cases Missing
- Same-profile reorder:
  - drag first session below last session in same profile
  - drag last session upward before first session
  - drag joined group by middle member and verify relative order unchanged
  - drag selected non-joined multi-session block within same profile and verify exact resulting order
- Cross-profile move:
  - drop on profile row to append at end
  - drop between target profile sessions at middle index, not only index `0`
  - move active session out of source profile and verify next active session selection
  - move non-joined multi-selection cross-profile and verify target profile does not gain joined markers/layout unexpectedly
- Joined-group drag:
  - drag joined cluster from first member, middle member, last member
  - move joined cluster to inactive target profile, restart app, verify restore/unjoin still behaves correctly
  - drag joined cluster onto another joined cluster’s profile and verify aliases do not overwrite each other
- Drag/no-drag click behavior:
  - press/release without crossing threshold still acts as click only
  - small pointer jitter below threshold does not start drag
  - click on already multi-selected row without drag: verify whether selection should collapse to single or stay multi
  - right-click after a small left drag attempt does not leave stale drag preview/selection
  - release outside valid drop target clears drag preview and does not move sessions

## Risk Summary
- Build risk: medium-high until full desktop suite is green.
- Logic risk: high for joined-cluster ordering and cross-profile multi-selection semantics.
- Gesture risk: medium due lack of direct automated tests around threshold/hit-test/release behavior.

## Unresolved Questions
- Is drag of arbitrary multi-selection intended, or should drag only move one session or one joined cluster?
- On plain click of one row inside a multi-selection, should selection collapse immediately or remain multi-selected?
