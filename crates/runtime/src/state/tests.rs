use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::workspace_ids::SessionViewId;
use crate::workspace_layout::{WorkspaceLayoutState, WorkspaceSplitAxis};
use store::{Store, StoredScrollbackRecordInput, StoredScrollbackRecordKind, StoredSessionStatus};

use super::canonical_scrollback::{
    build_logical_snapshot, materialize_output_chunk, render_snapshot, render_snapshot_for_terminal,
};
use super::explorer_utils::{normalize_relative_path, resolve_explorer_target};
use super::{
    RestoredPromptRedrawDecision, RuntimeState, SessionSpawnPlan,
    append_disconnected_restore_cleanup, classify_restored_prompt_redraw,
    collapse_trailing_duplicate_prompt_redraws, is_duplicate_restored_prompt_fragment,
    normalize_session_snapshot, normalize_terminal_fragment_for_compare, prepend_run_boundary,
    snapshot_requires_run_boundary, snapshot_trailing_fragment,
    strip_duplicate_restored_prompt_prefix, strip_volatile_terminal_control_sequences,
    trim_live_output, visible_terminal_fragment,
};
use crate::api::{RuntimeEvent, RuntimeSessionBridgeAction, RuntimeSessionLookup};
use crate::config::RuntimeConfig;
use crate::session::SessionEvent;

struct TempDb {
    path: PathBuf,
}

impl TempDb {
    fn new() -> Self {
        static NEXT_TEMP_DB_ID: AtomicU64 = AtomicU64::new(1);
        let path = std::env::temp_dir().join(format!(
            "chatminal-state-test-{}-{}-{}.db",
            std::process::id(),
            NEXT_TEMP_DB_ID.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        Self { path }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn create_state_with_session() -> (RuntimeState, String, TempDb) {
    let db = TempDb::new();
    let store = Store::initialize(&db.path).expect("initialize test store");
    let active_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;
    let session = store
        .create_session(
            &active_profile_id,
            Some("State Test".to_string()),
            "/tmp".to_string(),
            "/bin/sh".to_string(),
            false,
        )
        .expect("create session");
    store
        .set_active_session(&active_profile_id, Some(&session.session_id))
        .expect("set active session");

    let config = RuntimeConfig {
        endpoint: format!(
            "/tmp/chatminal-state-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_millis())
                .unwrap_or(0)
        ),
        default_shell: "/bin/sh".to_string(),
        default_preview_lines: 1000,
        max_scrollback_lines_per_session: 5_000,
        default_cols: 120,
        default_rows: 32,
        health_interval_ms: 1_000,
    };
    let state = RuntimeState::new(config, store).expect("create runtime state");
    (state, session.session_id, db)
}

fn reopen_state(db: &TempDb) -> RuntimeState {
    let config = RuntimeConfig {
        endpoint: format!(
            "/tmp/chatminal-state-reopen-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_millis())
                .unwrap_or(0)
        ),
        default_shell: "/bin/sh".to_string(),
        default_preview_lines: 1000,
        max_scrollback_lines_per_session: 5_000,
        default_cols: 120,
        default_rows: 32,
        health_interval_ms: 1_000,
    };
    RuntimeState::new(config, Store::initialize(&db.path).expect("reopen store"))
        .expect("recreate runtime state")
}

fn create_state_with_two_sessions() -> (RuntimeState, String, String, TempDb) {
    let db = TempDb::new();
    let store = Store::initialize(&db.path).expect("initialize test store");
    let active_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;
    let session_a = store
        .create_session(
            &active_profile_id,
            Some("State Test A".to_string()),
            "/tmp".to_string(),
            "/bin/sh".to_string(),
            true,
        )
        .expect("create session A");
    let session_b = store
        .create_session(
            &active_profile_id,
            Some("State Test B".to_string()),
            "/tmp".to_string(),
            "/bin/sh".to_string(),
            true,
        )
        .expect("create session B");
    store
        .set_active_session(&active_profile_id, Some(&session_a.session_id))
        .expect("set active session");

    let config = RuntimeConfig {
        endpoint: format!(
            "/tmp/chatminal-state-two-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_millis())
                .unwrap_or(0)
        ),
        default_shell: "/bin/sh".to_string(),
        default_preview_lines: 1000,
        max_scrollback_lines_per_session: 5_000,
        default_cols: 120,
        default_rows: 32,
        health_interval_ms: 1_000,
    };
    let state = RuntimeState::new(config, store).expect("create runtime state");
    (state, session_a.session_id, session_b.session_id, db)
}

fn restore_snapshot(state: &RuntimeState, session_id: &str) -> crate::api::RuntimeSessionSnapshot {
    state
        .session_restore_snapshot_get(session_id)
        .expect("load restore snapshot")
}

fn wait_for_file_len(path: &std::path::Path, expected_len: usize) -> String {
    for _ in 0..40 {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        if content.len() >= expected_len {
            return content;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn trim_live_output_keeps_tail_for_ascii() {
    let mut value = "abcdef".to_string();
    trim_live_output(&mut value, 4);
    assert_eq!(value, "cdef");
}

#[test]
fn trim_live_output_preserves_utf8_boundaries() {
    let mut value = "ééé".to_string();
    trim_live_output(&mut value, 5);
    assert_eq!(value, "éé");
}

#[test]
fn snapshot_without_trailing_newline_requires_run_boundary() {
    assert!(snapshot_requires_run_boundary(
        &store::StoredSessionSnapshot {
            content: "khoa2807@host ~ % ".to_string(),
            seq: 1,
        }
    ));
    assert!(!snapshot_requires_run_boundary(
        &store::StoredSessionSnapshot {
            content: "line\n".to_string(),
            seq: 1,
        }
    ));
}

#[test]
fn snapshot_trailing_fragment_returns_only_unterminated_tail() {
    assert_eq!(
        snapshot_trailing_fragment(&store::StoredSessionSnapshot {
            content: "line-1\nkhoa2807@host ~ % ".to_string(),
            seq: 2,
        }),
        Some("khoa2807@host ~ % ".to_string())
    );
    assert_eq!(
        snapshot_trailing_fragment(&store::StoredSessionSnapshot {
            content: "line-1\nline-2\n".to_string(),
            seq: 2,
        }),
        None
    );
}

#[test]
fn prepend_run_boundary_only_when_chunk_needs_it() {
    assert_eq!(prepend_run_boundary("prompt"), "\r\nprompt");
    assert_eq!(prepend_run_boundary("\nprompt"), "\nprompt");
    assert_eq!(prepend_run_boundary("\r\nprompt"), "\r\nprompt");
    assert_eq!(prepend_run_boundary("\rprompt"), "\r\n\rprompt");
}

#[test]
fn disconnected_restore_cleanup_forces_chatminal_title_and_visible_cursor() {
    assert_eq!(
        append_disconnected_restore_cleanup("abc"),
        "abc\x1b]1;Chatminal\x07\x1b]2;Chatminal\x07\x1b[?25h"
    );
}

#[test]
fn normalize_terminal_fragment_strips_ansi_and_crlf_noise() {
    let value = "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % ";
    assert_eq!(
        normalize_terminal_fragment_for_compare(value),
        "khoa2807@host ~ %"
    );
    assert_eq!(visible_terminal_fragment(value), "khoa2807@host ~ % ");
}

#[test]
fn normalize_terminal_fragment_ignores_dcs_xtversion_and_device_attributes() {
    let value = concat!(
        "\x1bP>|Chatminal 20260327-085528-1b1caf5365\x1b\\",
        "\x1b[?65;4;6;18;22;52c",
        "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % "
    );
    assert_eq!(visible_terminal_fragment(value), "khoa2807@host ~ % ");
}

#[test]
fn collapse_trailing_duplicate_prompt_redraws_keeps_only_latest_prompt_instance() {
    let value = concat!(
        "khoa2807@host ~ % ",
        "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % ",
        "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % "
    );
    let collapsed = collapse_trailing_duplicate_prompt_redraws(value);
    assert_eq!(collapsed, "khoa2807@host ~ % ");
}

#[test]
fn normalize_session_snapshot_collapses_duplicate_prompt_tail() {
    let snapshot = normalize_session_snapshot(store::StoredSessionSnapshot {
        content: concat!(
            "echo hi\n",
            "khoa2807@host ~ % ",
            "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % ",
            "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % "
        )
        .to_string(),
        seq: 3,
    });
    assert_eq!(snapshot.content, "echo hi\nkhoa2807@host ~ % ");
}

#[test]
fn normalize_session_snapshot_does_not_leak_xtversion_payload_into_prompt_tail() {
    let snapshot = normalize_session_snapshot(store::StoredSessionSnapshot {
        content: concat!(
            "echo hi\n",
            "khoa2807@host ~ % ",
            "\x1bP>|Chatminal 20260327-085528-1b1caf5365\x1b\\",
            "\x1b[?65;4;6;18;22;52c",
            "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % "
        )
        .to_string(),
        seq: 3,
    });
    assert_eq!(snapshot.content, "echo hi\nkhoa2807@host ~ % ");
}

#[test]
fn normalize_session_snapshot_collapses_trailing_prompt_only_lines() {
    let snapshot = normalize_session_snapshot(store::StoredSessionSnapshot {
        content: concat!(
            "echo hi\n",
            "hi\n",
            "khoa2807@host ~ % \n",
            "\n",
            "khoa2807@host ~ % \n",
            "khoa2807@host ~ % "
        )
        .to_string(),
        seq: 6,
    });
    assert_eq!(snapshot.content, "echo hi\nhi\nkhoa2807@host ~ % ");
}

#[test]
fn strip_volatile_terminal_control_sequences_removes_title_and_private_modes() {
    let value = concat!(
        "\x1b]0;Claude Code\x07",
        "\x1b]2;Claude Code\x1b\\",
        "\x1b[?25l",
        "\x1b[?1049h",
        "echo hi\n",
        "khoa2807@host ~ % "
    );

    assert_eq!(
        strip_volatile_terminal_control_sequences(value),
        "echo hi\nkhoa2807@host ~ % "
    );
}

#[test]
fn normalize_session_snapshot_strips_title_and_cursor_state_from_restore() {
    let snapshot = normalize_session_snapshot(store::StoredSessionSnapshot {
        content: concat!(
            "\x1b]0;Claude Code\x07",
            "\x1b[?25l",
            "echo hi\n",
            "khoa2807@host ~ % "
        )
        .to_string(),
        seq: 2,
    });

    assert_eq!(snapshot.content, "echo hi\nkhoa2807@host ~ % ");
}

#[test]
fn duplicate_restored_prompt_detection_requires_prompt_like_fragment() {
    assert!(is_duplicate_restored_prompt_fragment(
        "khoa2807@host ~ % ",
        "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % "
    ));
    assert!(!is_duplicate_restored_prompt_fragment(
        "identical payload",
        "identical payload"
    ));
}

#[test]
fn duplicate_restored_prompt_prefix_can_be_stripped_from_mixed_chunk() {
    let stripped = strip_duplicate_restored_prompt_prefix(
        "khoa2807@host ~ % ",
        "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % command output\n",
    )
    .expect("prompt prefix stripped");
    assert_eq!(stripped, "command output\n");
}

#[test]
fn duplicate_restored_prompt_redraw_waits_for_split_chunks() {
    assert!(matches!(
        classify_restored_prompt_redraw(
            "khoa2807@host ~ % ",
            "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807"
        ),
        RestoredPromptRedrawDecision::AwaitMore
    ));

    assert!(matches!(
        classify_restored_prompt_redraw(
            "khoa2807@host ~ % ",
            "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % "
        ),
        RestoredPromptRedrawDecision::Strip(stripped) if stripped.is_empty()
    ));
}

#[test]
fn stale_output_event_is_ignored_by_generation_guard() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 3;
        entry.session.seq = 7;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 2,
        chunk: "ignored-output".to_string(),
        ts: 1,
    });

    let inner = state.inner.lock().expect("lock state");
    let entry = inner
        .sessions
        .get(&session_id)
        .expect("session entry exists");
    assert_eq!(entry.session.seq, 7);
    assert_eq!(entry.session.status, StoredSessionStatus::Running);
    assert!(entry.live_output.is_empty());
}

#[test]
fn stale_exit_event_does_not_flip_session_status() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 5;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Exited {
        session_id: Arc::from(session_id.as_str()),
        generation: 4,
        exit_code: Some(0),
        reason: "stale".to_string(),
    });

    let inner = state.inner.lock().expect("lock state");
    let entry = inner
        .sessions
        .get(&session_id)
        .expect("session entry exists");
    assert_eq!(entry.session.status, StoredSessionStatus::Running);
}

#[test]
fn lifecycle_preferences_default_values() {
    let (state, _session_id, _db) = create_state_with_session();
    let inner = state.inner.lock().expect("lock state");
    let lifecycle = inner
        .get_lifecycle_preferences()
        .expect("get lifecycle preferences");
    assert!(lifecycle.keep_alive_on_close);
    assert!(!lifecycle.start_in_tray);
}

#[test]
fn metrics_snapshot_tracks_session_events() {
    let (state, session_id, _db) = create_state_with_session();
    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "hello\n".to_string(),
        ts: 1,
    });

    let snapshot = state.metrics_snapshot();
    assert!(snapshot.session_events_total >= 1);
}

#[test]
fn metrics_snapshot_tracks_runtime_broadcasts() {
    let (state, _session_id, _db) = create_state_with_session();
    let _ = state.profile_create(Some("Metrics".to_string()));

    let snapshot = state.metrics_snapshot();
    assert!(snapshot.broadcast_frames_total >= 1);
}

#[test]
fn native_workspace_load_passive_returns_runtime_snapshot() {
    let (state, session_id, _db) = create_state_with_session();

    let workspace = state
        .workspace_load_passive()
        .expect("load runtime workspace");

    assert_eq!(
        workspace.active_session_id.as_deref(),
        Some(session_id.as_str())
    );
    assert_eq!(workspace.sessions.len(), 1);
    assert_eq!(workspace.sessions[0].session_id, session_id);
}

#[test]
fn workspace_layout_roundtrip_persists_for_active_profile() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();
    let workspace_id = "desktop-main";
    let mut layout = WorkspaceLayoutState::new_single(session_a.clone());
    layout
        .split_view(
            SessionViewId::new(1),
            WorkspaceSplitAxis::Vertical,
            session_b.clone(),
        )
        .expect("split layout");

    state
        .workspace_layout_save(workspace_id, &layout)
        .expect("save layout");

    let loaded = state
        .workspace_layout_load(workspace_id)
        .expect("load layout")
        .expect("layout exists");
    assert_eq!(loaded, layout);
}

#[test]
fn workspace_layout_restore_persisted_populates_runtime_registry() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();
    let workspace_id = "desktop-main";
    let mut layout = WorkspaceLayoutState::new_single(session_a.clone());
    layout
        .split_view(
            SessionViewId::new(1),
            WorkspaceSplitAxis::Vertical,
            session_b.clone(),
        )
        .expect("split layout");
    state
        .workspace_layout_save(workspace_id, &layout)
        .expect("save layout");

    assert_eq!(
        state
            .workspace_layout_snapshot(workspace_id)
            .expect("snapshot before restore"),
        None
    );

    let restored = state
        .workspace_layout_restore_persisted(workspace_id)
        .expect("restore persisted layout")
        .expect("restored layout");
    assert_eq!(restored, layout);
    assert_eq!(
        state
            .workspace_layout_snapshot(workspace_id)
            .expect("snapshot after restore"),
        Some(layout)
    );
}

#[test]
fn workspace_layout_facade_mutations_update_runtime_registry() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();

    let initial = state
        .workspace_layout_ensure_session("desktop-main", &session_a)
        .expect("ensure session a");
    assert_eq!(initial.views.len(), 1);

    let split = state
        .workspace_layout_split_view(
            "desktop-main",
            SessionViewId::new(1),
            WorkspaceSplitAxis::Vertical,
            &session_b,
        )
        .expect("split call")
        .expect("split result");
    assert_eq!(split.views.len(), 2);

    let session_b_view = state
        .workspace_layout_view_id_for_session("desktop-main", &session_b)
        .expect("view lookup")
        .expect("session b view");
    assert_eq!(session_b_view, SessionViewId::new(2));

    let focused = state
        .workspace_layout_focus_view("desktop-main", SessionViewId::new(1))
        .expect("focus call")
        .expect("focus result");
    assert_eq!(focused.active_view_id, SessionViewId::new(1));

    let removed = state
        .workspace_layout_remove("desktop-main")
        .map(|_| {
            state
                .workspace_layout_snapshot("desktop-main")
                .expect("snapshot")
        })
        .expect("remove layout");
    assert_eq!(removed, None);
}

#[test]
fn workspace_layout_load_prunes_sessions_missing_from_profile() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();
    let workspace_id = "desktop-main";
    let mut layout = WorkspaceLayoutState::new_single(session_a.clone());
    layout
        .split_view(
            SessionViewId::new(1),
            WorkspaceSplitAxis::Vertical,
            session_b.clone(),
        )
        .expect("split layout");
    state
        .workspace_layout_save(workspace_id, &layout)
        .expect("save layout");
    state.session_close(&session_b).expect("close session b");

    let loaded = state
        .workspace_layout_load(workspace_id)
        .expect("load pruned layout")
        .expect("layout exists");
    assert_eq!(loaded.views.len(), 1);
    assert_eq!(loaded.views[0].session_id, session_a);
}

#[test]
fn session_move_to_profile_updates_runtime_state_without_touching_ui_contract() {
    let (state, session_a, _session_b, _db) = create_state_with_two_sessions();
    let target = state
        .profile_create(Some("Work".to_string()))
        .expect("create profile");

    let moved = state
        .session_move_to_profile(&session_a, &target.profile_id, Some(0))
        .expect("move session");

    let active_profile_id = moved
        .active_profile_id
        .clone()
        .expect("active profile id after move");
    assert!(
        moved
            .sessions
            .iter()
            .any(|session| session.profile_id == active_profile_id)
    );
    assert!(
        moved
            .sessions
            .iter()
            .any(|session| session.session_id == session_a
                && session.profile_id == target.profile_id)
    );

    let launch = state
        .session_launch_spec(&session_a)
        .expect("load moved session launch spec");
    assert_eq!(launch.session_id, session_a);
}

#[test]
fn sessions_move_to_profile_reorders_group_without_losing_runtime_state() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();
    let created = state
        .session_create(Some("Third".to_string()), 120, 32, None, Some(true))
        .expect("create third");
    let session_c = created.session_id.clone();

    let moved = state
        .sessions_move_to_profile(
            &[session_b.clone(), session_c.clone()],
            state
                .workspace_load_passive()
                .expect("load workspace")
                .active_profile_id
                .as_deref()
                .expect("active profile"),
            Some(0),
        )
        .expect("move grouped sessions");

    let ordered_ids = moved
        .sessions
        .into_iter()
        .map(|session| session.session_id)
        .collect::<Vec<_>>();
    assert_eq!(ordered_ids, vec![session_b, session_c, session_a]);
}

#[test]
fn workspace_layout_survives_profile_switch_and_restart_with_cross_profile_sessions() {
    let db = TempDb::new();
    let store = Store::initialize(&db.path).expect("initialize test store");
    let default_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;
    let session_a = store
        .create_session(
            &default_profile_id,
            Some("Default A".to_string()),
            "/tmp".to_string(),
            "/bin/sh".to_string(),
            true,
        )
        .expect("create default session");
    let work_profile = store
        .create_profile(Some("Work".to_string()))
        .expect("create work profile");
    let session_b = store
        .create_session(
            &work_profile.profile_id,
            Some("Work B".to_string()),
            "/tmp".to_string(),
            "/bin/sh".to_string(),
            true,
        )
        .expect("create work session");
    store
        .set_active_session(&default_profile_id, Some(&session_a.session_id))
        .expect("set active session");

    let config = RuntimeConfig {
        endpoint: format!(
            "/tmp/chatminal-state-cross-profile-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_millis())
                .unwrap_or(0)
        ),
        default_shell: "/bin/sh".to_string(),
        default_preview_lines: 1000,
        max_scrollback_lines_per_session: 5_000,
        default_cols: 120,
        default_rows: 32,
        health_interval_ms: 1_000,
    };

    let state = RuntimeState::new(config.clone(), store.clone()).expect("create runtime state");
    let workspace_id = "desktop-main";
    let mut layout = WorkspaceLayoutState::new_single(session_a.session_id.clone());
    layout
        .split_view(
            SessionViewId::new(1),
            WorkspaceSplitAxis::Vertical,
            session_b.session_id.clone(),
        )
        .expect("split cross profile layout");
    state
        .workspace_layout_save(workspace_id, &layout)
        .expect("save layout");

    state
        .profile_switch(&work_profile.profile_id)
        .expect("switch profile");
    let loaded_after_switch = state
        .workspace_layout_load(workspace_id)
        .expect("load layout after switch")
        .expect("layout exists after switch");
    assert_eq!(loaded_after_switch.views.len(), 2);
    assert!(
        loaded_after_switch
            .views
            .iter()
            .any(|view| view.session_id == session_a.session_id)
    );
    assert!(
        loaded_after_switch
            .views
            .iter()
            .any(|view| view.session_id == session_b.session_id)
    );

    drop(state);

    let restored = RuntimeState::new(config, Store::initialize(&db.path).expect("reopen store"))
        .expect("recreate runtime state")
        .workspace_layout_restore_persisted(workspace_id)
        .expect("restore persisted layout")
        .expect("restored layout exists");
    assert_eq!(restored.views.len(), 2);
    assert!(
        restored
            .views
            .iter()
            .any(|view| view.session_id == session_a.session_id)
    );
    assert!(
        restored
            .views
            .iter()
            .any(|view| view.session_id == session_b.session_id)
    );
}

#[test]
fn native_subscription_receives_workspace_updated_event() {
    let (state, _session_id, _db) = create_state_with_session();
    let subscription = state.subscribe().expect("subscribe runtime events");

    let _ = state.profile_create(Some("Workspace".to_string()));

    let event = subscription
        .recv_timeout(Duration::from_secs(1))
        .expect("receive event")
        .expect("event payload");
    match event {
        RuntimeEvent::WorkspaceUpdated(value) => {
            assert!(value.profile_count >= 2);
        }
        other => panic!("unexpected runtime event: {other:?}"),
    }
}

#[test]
fn lifecycle_preferences_set_roundtrip() {
    let (state, _session_id, _db) = create_state_with_session();
    {
        let inner = state.inner.lock().expect("lock state");
        let updated = inner
            .set_lifecycle_preferences(Some(false), Some(true))
            .expect("set lifecycle preferences");
        assert!(!updated.keep_alive_on_close);
        assert!(updated.start_in_tray);
    }

    let inner = state.inner.lock().expect("lock state again");
    let loaded = inner
        .get_lifecycle_preferences()
        .expect("reload lifecycle preferences");
    assert!(!loaded.keep_alive_on_close);
    assert!(loaded.start_in_tray);
}

#[test]
fn lifecycle_preferences_partial_update_keeps_other_key() {
    let (state, _session_id, _db) = create_state_with_session();
    {
        let inner = state.inner.lock().expect("lock state");
        let _ = inner
            .set_lifecycle_preferences(Some(false), None)
            .expect("set keep_alive only");
    }
    {
        let inner = state.inner.lock().expect("lock state");
        let loaded = inner
            .get_lifecycle_preferences()
            .expect("load lifecycle after first update");
        assert!(!loaded.keep_alive_on_close);
        assert!(!loaded.start_in_tray);
    }
    {
        let inner = state.inner.lock().expect("lock state");
        let _ = inner
            .set_lifecycle_preferences(None, Some(true))
            .expect("set start_in_tray only");
    }
    let inner = state.inner.lock().expect("lock state");
    let loaded = inner
        .get_lifecycle_preferences()
        .expect("load lifecycle after second update");
    assert!(!loaded.keep_alive_on_close);
    assert!(loaded.start_in_tray);
}

#[test]
fn mark_session_running_and_publish_flips_disconnected_snapshot_to_running() {
    let (state, session_id, _db) = create_state_with_session();

    let before = state.workspace_load_passive().expect("workspace before");
    let before_session = before
        .sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .expect("session before");
    assert_eq!(
        before_session.status,
        crate::api::RuntimeSessionStatus::Disconnected
    );

    state
        .mark_session_running_and_publish(&session_id)
        .expect("mark session running");

    let after = state.workspace_load_passive().expect("workspace after");
    let after_session = after
        .sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .expect("session after");
    assert_eq!(
        after_session.status,
        crate::api::RuntimeSessionStatus::Running
    );
}

#[test]
fn runtime_new_resets_stale_running_sessions_to_disconnected_in_memory() {
    let db = TempDb::new();
    let store = Store::initialize(&db.path).expect("initialize test store");
    let active_profile_id = store
        .load_workspace()
        .expect("load workspace")
        .active_profile_id;
    let session = store
        .create_session(
            &active_profile_id,
            Some("State Test".to_string()),
            "/tmp".to_string(),
            "/bin/sh".to_string(),
            false,
        )
        .expect("create session");
    store
        .set_session_status(&session.session_id, StoredSessionStatus::Running)
        .expect("seed stale running status");
    store
        .set_active_session(&active_profile_id, Some(&session.session_id))
        .expect("set active session");

    let config = RuntimeConfig {
        endpoint: format!(
            "/tmp/chatminal-state-reset-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_millis())
                .unwrap_or(0)
        ),
        default_shell: "/bin/sh".to_string(),
        default_preview_lines: 1000,
        max_scrollback_lines_per_session: 5_000,
        default_cols: 120,
        default_rows: 32,
        health_interval_ms: 1_000,
    };

    let state = RuntimeState::new(config, store.clone()).expect("create runtime state");

    let workspace = state
        .workspace_load_passive()
        .expect("workspace load passive");
    let session_entry = workspace
        .sessions
        .iter()
        .find(|value| value.session_id == session.session_id)
        .expect("session entry");
    assert_eq!(
        session_entry.status,
        crate::api::RuntimeSessionStatus::Disconnected
    );

    let stored = store
        .get_session(&session.session_id)
        .expect("get stored session")
        .expect("stored session exists");
    assert_eq!(stored.status, StoredSessionStatus::Disconnected);
}

#[test]
fn workspace_load_auto_connects_active_session_runtime() {
    let (state, session_id, _db) = create_state_with_session();

    let workspace = state.workspace_load().expect("workspace load");
    assert_eq!(
        workspace.active_session_id.as_deref(),
        Some(session_id.as_str())
    );

    {
        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_id)
            .expect("session entry exists");
        assert!(entry.runtime.is_some());
        assert_eq!(entry.session.status, StoredSessionStatus::Running);
    }

    state.app_shutdown();
}

#[test]
fn workspace_load_passive_keeps_active_session_disconnected() {
    let (state, session_id, _db) = create_state_with_session();

    let workspace = state
        .workspace_load_passive()
        .expect("workspace load passive");
    assert_eq!(
        workspace.active_session_id.as_deref(),
        Some(session_id.as_str())
    );

    {
        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_id)
            .expect("session entry exists");
        assert!(entry.runtime.is_none());
        assert_eq!(entry.session.status, StoredSessionStatus::Disconnected);
    }
}

#[test]
fn reconcile_session_lookup_prefers_runtime_active_session() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();
    let lookup = RuntimeSessionLookup {
        active_session_id: Some(session_b),
        ..RuntimeSessionLookup::default()
    };

    let action = state
        .reconcile_session_lookup(&lookup)
        .expect("reconcile session runtime lookup");

    assert_eq!(
        action,
        RuntimeSessionBridgeAction::FocusSession {
            session_id: session_a,
        }
    );
}

#[test]
fn session_activate_increments_generation_on_each_spawn() {
    let (state, session_id, _db) = create_state_with_session();
    let generation_before = {
        let inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get(&session_id)
            .expect("session entry exists")
            .generation
    };

    state
        .session_activate(&session_id, 120, 32)
        .expect("activate session");
    let generation_after_first = {
        let inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get(&session_id)
            .expect("session entry exists")
            .generation
    };
    assert_eq!(generation_after_first, generation_before.saturating_add(1));

    state
        .session_history_clear(&session_id)
        .expect("clear session history");
    state
        .session_activate(&session_id, 120, 32)
        .expect("activate session again");
    let generation_after_second = {
        let inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get(&session_id)
            .expect("session entry exists")
            .generation
    };
    assert!(generation_after_second > generation_after_first);

    state.app_shutdown();
}

#[test]
fn session_create_spawn_failure_does_not_change_active_session() {
    let (state, active_session_id, _other_session_id, _db) = create_state_with_two_sessions();
    {
        let mut inner = state.inner.lock().expect("lock state");
        inner.config.default_shell = "/definitely/missing-shell".to_string();
    }

    let err = state
        .session_create(Some("Broken".to_string()), 120, 32, None, Some(true))
        .expect_err("session create should fail when shell is invalid");
    assert!(
        err.contains("No such file")
            || err.contains("not found")
            || err.contains("failed")
            || err.contains("spawn"),
        "unexpected error: {err}"
    );

    let workspace = state
        .workspace_load_passive()
        .expect("workspace should remain readable");
    assert_eq!(
        workspace.active_session_id.as_deref(),
        Some(active_session_id.as_str())
    );
    assert_eq!(workspace.sessions.len(), 2);

    let inner = state.inner.lock().expect("lock state");
    assert_eq!(inner.sessions.len(), 2);
    assert!(inner.sessions.contains_key(&active_session_id));
}

#[test]
fn session_activate_resizes_existing_runtime() {
    let (state, session_id, _db) = create_state_with_session();

    state
        .session_activate(&session_id, 80, 24)
        .expect("activate session");

    let initial_size = {
        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_id)
            .expect("session entry exists");
        let runtime = entry.runtime.as_ref().expect("runtime should exist");
        runtime
            .lock()
            .expect("lock runtime")
            .size()
            .expect("read initial pty size")
    };
    assert_eq!(initial_size, (80, 24));

    state
        .session_activate(&session_id, 132, 43)
        .expect("resize session");

    let resized_size = {
        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_id)
            .expect("session entry exists");
        let runtime = entry.runtime.as_ref().expect("runtime should exist");
        runtime
            .lock()
            .expect("lock runtime")
            .size()
            .expect("read resized pty size")
    };
    assert_eq!(resized_size, (132, 43));

    state.app_shutdown();
}

#[test]
fn session_activate_runs_startup_command_on_spawn() {
    let (state, session_id, db) = create_state_with_session();
    let marker_file = db.path.with_extension("startup-marker");
    let _ = std::fs::remove_file(&marker_file);
    let startup_command = format!("printf x >> '{}'", marker_file.display());

    state
        .session_set_startup_command(&session_id, Some(&startup_command))
        .expect("set startup command");

    state
        .session_activate(&session_id, 80, 24)
        .expect("activate session");
    let first_content = wait_for_file_len(&marker_file, 1);
    assert_eq!(first_content, "x");

    state.app_shutdown();
    let _ = std::fs::remove_file(&marker_file);
}

#[test]
fn session_run_startup_command_supports_wait_for_and_sleep_steps() {
    let (state, session_id, db) = create_state_with_session();
    let marker_file = db.path.with_extension("startup-recipe-marker");
    let _ = std::fs::remove_file(&marker_file);
    let startup_recipe = format!(
        "wait-for READY timeout=3000; wait 50; run printf y >> '{}'",
        marker_file.display()
    );

    state
        .session_set_startup_command(&session_id, Some(&startup_recipe))
        .expect("set startup recipe");

    state
        .session_activate(&session_id, 80, 24)
        .expect("activate session");
    let generation = {
        let inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get(&session_id)
            .expect("session entry exists")
            .generation
    };

    let cloned = state.clone();
    let session_id_for_output = session_id.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        cloned.apply_session_event(SessionEvent::Output {
            session_id: Arc::from(session_id_for_output.as_str()),
            generation,
            chunk: "READY\n".to_string(),
            ts: 0,
        });
    });

    state
        .session_run_startup_command(&session_id)
        .expect("run startup recipe");
    let content = wait_for_file_len(&marker_file, 1);
    assert_eq!(content, "y");

    state.app_shutdown();
    let _ = std::fs::remove_file(&marker_file);
}

#[test]
fn session_focus_updates_active_session_without_resizing_runtime() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();

    state
        .session_activate(&session_a, 80, 24)
        .expect("activate session a");

    let size_before_focus = {
        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_a)
            .expect("session a entry exists");
        let runtime = entry.runtime.as_ref().expect("session a runtime exists");
        runtime
            .lock()
            .expect("lock runtime")
            .size()
            .expect("read runtime size before focus")
    };
    assert_eq!(size_before_focus, (80, 24));

    state
        .session_focus(&session_b)
        .expect("focus session b without spawning");

    let workspace = state
        .workspace_load_passive()
        .expect("workspace should remain readable");
    assert_eq!(
        workspace.active_session_id.as_deref(),
        Some(session_b.as_str())
    );

    let (size_after_focus, session_b_has_runtime) = {
        let inner = state.inner.lock().expect("lock state");
        let entry_a = inner
            .sessions
            .get(&session_a)
            .expect("session a entry exists");
        let runtime_a = entry_a.runtime.as_ref().expect("session a runtime exists");
        let size = runtime_a
            .lock()
            .expect("lock runtime")
            .size()
            .expect("read runtime size after focus");
        let entry_b = inner
            .sessions
            .get(&session_b)
            .expect("session b entry exists");
        (size, entry_b.runtime.is_some())
    };
    assert_eq!(size_after_focus, (80, 24));
    assert!(!session_b_has_runtime);

    state.app_shutdown();
}

#[test]
fn stale_workspace_spawn_plan_cannot_restore_old_active_session() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();

    let (profile_id, expected_generation, shell, cwd) = {
        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_a)
            .expect("session entry exists");
        (
            entry.session.profile_id.clone(),
            entry.generation,
            entry.session.shell.clone(),
            entry.session.cwd.clone(),
        )
    };

    {
        let inner = state.inner.lock().expect("lock state");
        inner
            .store
            .set_active_session(&profile_id, Some(&session_b))
            .expect("switch active session");
    }

    let err = state
        .commit_spawned_session(SessionSpawnPlan {
            session_id: session_a.clone(),
            profile_id: profile_id.clone(),
            expected_active_session_id: Some(session_a.clone()),
            expected_generation,
            next_generation: expected_generation.saturating_add(1),
            shell,
            cwd,
            cols: 120,
            rows: 32,
        })
        .expect_err("stale active-session guard should reject commit");
    assert!(
        err.contains("session state changed"),
        "unexpected error: {err}"
    );

    let workspace = state
        .workspace_load_passive()
        .expect("load passive workspace");
    assert_eq!(
        workspace.active_session_id.as_deref(),
        Some(session_b.as_str())
    );

    {
        let inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get(&session_a)
            .expect("session entry exists");
        assert!(entry.runtime.is_none());
        assert_eq!(entry.generation, expected_generation);
    }
}

#[test]
fn session_history_clear_disconnects_runtime_and_resets_snapshot() {
    let (state, session_id, _db) = create_state_with_session();

    state
        .session_activate(&session_id, 120, 32)
        .expect("activate session");
    state
        .session_set_persist(&session_id, true)
        .expect("enable persist history");

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "echo hello\n".to_string(),
        ts: 11,
    });

    state
        .session_history_clear(&session_id)
        .expect("clear session history");

    let inner = state.inner.lock().expect("lock state");
    let entry = inner
        .sessions
        .get(&session_id)
        .expect("session entry exists");
    assert!(entry.runtime.is_none());
    assert_eq!(entry.session.status, StoredSessionStatus::Disconnected);
    assert_eq!(entry.session.seq, 0);

    let snapshot = inner
        .store
        .session_snapshot(&session_id, 100)
        .expect("load session snapshot after clear");
    assert_eq!(snapshot.seq, 0);
    assert!(snapshot.content.is_empty());
}

#[test]
fn session_set_persist_flushes_live_output_into_store_snapshot() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.live_output = "cached-line\n".to_string();
        entry.session.seq = 0;
        entry.session.persist_history = false;
    }

    state
        .session_set_persist(&session_id, true)
        .expect("enable persist history");

    let inner = state.inner.lock().expect("lock state");
    let entry = inner
        .sessions
        .get(&session_id)
        .expect("session entry exists");
    assert!(entry.live_output.is_empty());
    assert!(entry.session.persist_history);
    assert_eq!(entry.session.seq, 1);
    drop(inner);
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.content, "cached-line\r\n");
}

#[test]
fn first_output_after_respawn_is_separated_from_prompt_only_snapshot() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "khoa2807@host ~ % ".to_string(),
        ts: 1,
    });

    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 1;
        entry.runtime = None;
        entry.session.status = StoredSessionStatus::Disconnected;
        entry.prepend_run_boundary_on_next_output = true;
        entry.restored_trailing_fragment = Some("khoa2807@host ~ % ".to_string());
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "khoa2807@host ~ % ".to_string(),
        ts: 2,
    });

    state.flush_persist();
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(
        snapshot.content,
        append_disconnected_restore_cleanup("khoa2807@host ~ % ")
    );
}

#[test]
fn first_distinct_output_after_respawn_still_starts_on_new_line() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "khoa2807@host ~ % ".to_string(),
        ts: 1,
    });

    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 1;
        entry.runtime = None;
        entry.session.status = StoredSessionStatus::Disconnected;
        entry.prepend_run_boundary_on_next_output = true;
        entry.restored_trailing_fragment = Some("khoa2807@host ~ % ".to_string());
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "command output\n".to_string(),
        ts: 2,
    });

    state.flush_persist();
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.content, "khoa2807@host ~ % command output\n");
}

#[test]
fn ansi_decorated_prompt_redraw_after_respawn_is_not_persisted_twice() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "khoa2807@host ~ % ".to_string(),
        ts: 1,
    });

    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 1;
        entry.runtime = None;
        entry.session.status = StoredSessionStatus::Disconnected;
        entry.prepend_run_boundary_on_next_output = true;
        entry.restored_trailing_fragment = Some("khoa2807@host ~ % ".to_string());
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % ".to_string(),
        ts: 2,
    });

    state.flush_persist();
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(
        snapshot.content,
        append_disconnected_restore_cleanup("khoa2807@host ~ % ")
    );
}

#[test]
fn repeated_prompt_redraws_after_respawn_are_all_ignored_until_real_output() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "khoa2807@host ~ % ".to_string(),
        ts: 1,
    });

    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 1;
        entry.runtime = None;
        entry.session.status = StoredSessionStatus::Disconnected;
        entry.prepend_run_boundary_on_next_output = true;
        entry.restored_trailing_fragment = Some("khoa2807@host ~ % ".to_string());
    }

    for ts in 2..=4 {
        state.apply_session_event(SessionEvent::Output {
            session_id: Arc::from(session_id.as_str()),
            generation: 1,
            chunk: "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % ".to_string(),
            ts,
        });
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "echo hi\nhi\n".to_string(),
        ts: 5,
    });

    state.flush_persist();
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.content, "khoa2807@host ~ % echo hi\nhi\n");
}

#[test]
fn identical_non_prompt_fragment_after_respawn_is_preserved() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "identical payload".to_string(),
        ts: 1,
    });

    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 1;
        entry.runtime = None;
        entry.session.status = StoredSessionStatus::Disconnected;
        entry.prepend_run_boundary_on_next_output = true;
        entry.restored_trailing_fragment = Some("identical payload".to_string());
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "identical payload".to_string(),
        ts: 2,
    });

    state.flush_persist();
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.content, "identical payload\r\nidentical payload");
}

#[test]
fn prompt_redraw_prefixed_output_after_respawn_persists_only_new_output() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "khoa2807@host ~ % ".to_string(),
        ts: 1,
    });

    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 1;
        entry.runtime = None;
        entry.session.status = StoredSessionStatus::Disconnected;
        entry.prepend_run_boundary_on_next_output = true;
        entry.restored_trailing_fragment = Some("khoa2807@host ~ % ".to_string());
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@host ~ % command output\n".to_string(),
        ts: 2,
    });

    state.flush_persist();
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.content, "khoa2807@host ~ % command output\n");
}

#[test]
fn restored_prompt_keeps_first_command_echo_on_same_line() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "khoa2807@192 ~ % ".to_string(),
        ts: 1,
    });

    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.generation = 1;
        entry.runtime = None;
        entry.session.status = StoredSessionStatus::Disconnected;
        entry.prepend_run_boundary_on_next_output = true;
        entry.restored_trailing_fragment = Some("khoa2807@192 ~ % ".to_string());
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "abc\r\nzsh: command not found: abc\r\nkhoa2807@192 ~ % ".to_string(),
        ts: 2,
    });

    state.flush_persist();
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(
        snapshot.content,
        "khoa2807@192 ~ % abc\r\nzsh: command not found: abc\r\nkhoa2807@192 ~ % "
    );
}

#[test]
fn persisted_history_applies_line_retention_limit() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        inner.config.max_scrollback_lines_per_session = 2;
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "l1\n".to_string(),
        ts: 1,
    });
    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "l2\n".to_string(),
        ts: 2,
    });
    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "l3\n".to_string(),
        ts: 3,
    });

    state.flush_persist();
    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.seq, 3);
    assert_eq!(snapshot.content, "l1\nl2\nl3\n");
}

#[test]
fn restore_snapshot_prefers_terminal_replay_over_canonical_snapshot() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists")
            .session
            .status = StoredSessionStatus::Running;
        inner
            .store
            .append_scrollback_records(
                &session_id,
                1,
                &[StoredScrollbackRecordInput {
                    ord: 0,
                    kind: StoredScrollbackRecordKind::Line,
                    text: "canonical-only".to_string(),
                }],
                1,
            )
            .expect("append canonical");
        inner
            .store
            .append_terminal_replay_chunk(&session_id, 2, "\u{1b}]2;Claude Code\u{7}prompt % ", 2)
            .expect("append replay");
    }

    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.seq, 2);
    assert_eq!(snapshot.content, "prompt % ");
}

#[test]
fn restore_snapshot_collapses_duplicate_prompt_tail_from_terminal_replay() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists")
            .session
            .status = StoredSessionStatus::Running;
        inner
            .store
            .append_terminal_replay_chunk(
                &session_id,
                1,
                concat!("echo hi\r\n", "khoa2807@192 ~ % \r\n", "khoa2807@192 ~ % "),
                1,
            )
            .expect("append replay");
    }

    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.seq, 1);
    assert_eq!(snapshot.content, "echo hi\r\nkhoa2807@192 ~ % ");
}

#[test]
fn restore_snapshot_keeps_duplicate_prompt_lines_when_last_prompt_is_committed() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists")
            .session
            .status = StoredSessionStatus::Running;
        inner
            .store
            .append_terminal_replay_chunk(
                &session_id,
                1,
                concat!(
                    "echo hi\r\n",
                    "khoa2807@192 ~ % \r\n",
                    "khoa2807@192 ~ % \r\n"
                ),
                1,
            )
            .expect("append replay");
    }

    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(snapshot.seq, 1);
    assert_eq!(
        snapshot.content,
        "echo hi\r\nkhoa2807@192 ~ % \r\nkhoa2807@192 ~ % \r\n"
    );
}

#[test]
fn restore_snapshot_stays_stable_across_restart_and_prompt_redraw_cycle() {
    let (state, session_id, db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "khoa2807@192 ~ % ".to_string(),
        ts: 1,
    });
    state.flush_persist();
    state.app_shutdown();
    drop(state);

    let reopened = reopen_state(&db);
    let restored_before = restore_snapshot(&reopened, &session_id);
    assert_eq!(
        restored_before.content,
        append_disconnected_restore_cleanup("khoa2807@192 ~ % ")
    );

    reopened
        .session_activate(&session_id, 120, 32)
        .expect("activate reopened session");
    let generation = {
        let inner = reopened.inner.lock().expect("lock reopened state");
        inner
            .sessions
            .get(&session_id)
            .expect("session entry exists")
            .generation
    };
    reopened.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation,
        chunk: "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@192 ~ % ".to_string(),
        ts: 2,
    });
    reopened.flush_persist();
    reopened.app_shutdown();
    drop(reopened);

    let reopened_again = reopen_state(&db);
    let restored_after = restore_snapshot(&reopened_again, &session_id);
    assert_eq!(
        restored_after.content,
        append_disconnected_restore_cleanup("khoa2807@192 ~ % ")
    );
}

#[test]
fn restore_snapshot_stays_stable_across_restart_and_split_prompt_redraw_cycle() {
    let (state, session_id, db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "khoa2807@192 ~ % ".to_string(),
        ts: 1,
    });
    state.flush_persist();
    state.app_shutdown();
    drop(state);

    let reopened = reopen_state(&db);
    reopened
        .session_activate(&session_id, 120, 32)
        .expect("activate reopened session");
    let generation = {
        let inner = reopened.inner.lock().expect("lock reopened state");
        inner
            .sessions
            .get(&session_id)
            .expect("session entry exists")
            .generation
    };
    reopened.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation,
        chunk: "\r\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807".to_string(),
        ts: 2,
    });
    reopened.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation,
        chunk: "@192 ~ % ".to_string(),
        ts: 3,
    });
    reopened.flush_persist();
    reopened.app_shutdown();
    drop(reopened);

    let reopened_again = reopen_state(&db);
    let restored_after = restore_snapshot(&reopened_again, &session_id);
    assert_eq!(
        restored_after.content,
        append_disconnected_restore_cleanup("khoa2807@192 ~ % ")
    );
}

#[test]
fn disconnected_restore_snapshot_appends_cleanup_to_terminal_replay() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let inner = state.inner.lock().expect("lock state");
        inner
            .store
            .append_terminal_replay_chunk(&session_id, 1, "\u{1b}]2;Claude Code\u{7}abc", 1)
            .expect("append replay");
    }

    let snapshot = restore_snapshot(&state, &session_id);
    assert_eq!(
        snapshot.content,
        "abc\x1b]1;Chatminal\x07\x1b]2;Chatminal\x07\x1b[?25h"
    );
}

#[test]
fn clear_history_generation_gate_ignores_old_output_after_reset() {
    let (state, session_id, _db) = create_state_with_session();
    state
        .session_activate(&session_id, 120, 32)
        .expect("activate session");
    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "before-clear\n".to_string(),
        ts: 1,
    });
    state
        .session_history_clear(&session_id)
        .expect("clear session history");

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 1,
        chunk: "stale-after-clear\n".to_string(),
        ts: 2,
    });

    let inner = state.inner.lock().expect("lock state");
    let entry = inner
        .sessions
        .get(&session_id)
        .expect("session entry exists");
    assert_eq!(entry.session.seq, 0);
    assert!(entry.live_output.is_empty());
    let snapshot = inner
        .store
        .session_snapshot(&session_id, 100)
        .expect("snapshot after clear");
    assert_eq!(snapshot.seq, 0);
    assert!(snapshot.content.is_empty());
}

#[test]
fn canonical_materializer_handles_backspace_carriage_return_and_erase_in_line() {
    let materialized = materialize_output_chunk(
        "prompt % ",
        "prompt % ".chars().count(),
        false,
        "echo helo\x08l\nnext\x1b[Kdone",
    );

    assert_eq!(
        materialized.records,
        vec![
            StoredScrollbackRecordInput {
                ord: 0,
                kind: StoredScrollbackRecordKind::Line,
                text: "prompt % echo hell".to_string(),
            },
            StoredScrollbackRecordInput {
                ord: 1,
                kind: StoredScrollbackRecordKind::Fragment,
                text: "nextdone".to_string(),
            },
        ]
    );
    assert_eq!(materialized.open_fragment, "nextdone");
    assert!(!materialized.pending_carriage_return);
}

#[test]
fn canonical_materializer_preserves_content_for_crlf_line_breaks() {
    let materialized = materialize_output_chunk("", 0, false, "line-1\r\nline-2\r\nprompt % ");

    assert_eq!(
        materialized.records,
        vec![
            StoredScrollbackRecordInput {
                ord: 0,
                kind: StoredScrollbackRecordKind::Line,
                text: "line-1".to_string(),
            },
            StoredScrollbackRecordInput {
                ord: 1,
                kind: StoredScrollbackRecordKind::Line,
                text: "line-2".to_string(),
            },
            StoredScrollbackRecordInput {
                ord: 2,
                kind: StoredScrollbackRecordKind::Fragment,
                text: "prompt % ".to_string(),
            },
        ]
    );
    assert_eq!(materialized.open_fragment, "prompt % ");
    assert!(!materialized.pending_carriage_return);
}

#[test]
fn canonical_materializer_preserves_fragment_across_split_crlf_boundary() {
    let first = materialize_output_chunk("", 0, false, "ád\r");
    assert_eq!(first.records.len(), 1);
    assert_eq!(first.open_fragment, "ád");
    assert_eq!(first.cursor_col, 2);
    assert!(first.pending_carriage_return);

    let second = materialize_output_chunk(
        &first.open_fragment,
        first.cursor_col,
        first.pending_carriage_return,
        "\nzsh: command not found: ád\r\n",
    );

    assert_eq!(
        second.records,
        vec![
            StoredScrollbackRecordInput {
                ord: 0,
                kind: StoredScrollbackRecordKind::Line,
                text: "ád".to_string(),
            },
            StoredScrollbackRecordInput {
                ord: 1,
                kind: StoredScrollbackRecordKind::Line,
                text: "zsh: command not found: ád".to_string(),
            },
            StoredScrollbackRecordInput {
                ord: 2,
                kind: StoredScrollbackRecordKind::Fragment,
                text: "".to_string(),
            },
        ]
    );
    assert_eq!(second.cursor_col, 0);
    assert!(!second.pending_carriage_return);
}

#[test]
fn canonical_materializer_preserves_prompt_when_shell_redraws_with_cursor_forward() {
    let prompt = "khoa2807@192 ~ % ";
    let redraw = format!("\r\x1b[{}Cád", prompt.chars().count());
    let materialized = materialize_output_chunk(prompt, prompt.chars().count(), false, &redraw);

    assert_eq!(materialized.records.len(), 1);
    assert_eq!(materialized.records[0].text, "khoa2807@192 ~ % ád");
    assert_eq!(materialized.open_fragment, "khoa2807@192 ~ % ád");
    assert_eq!(
        materialized.cursor_col,
        "khoa2807@192 ~ % ád".chars().count()
    );
    assert!(!materialized.pending_carriage_return);
}

#[test]
fn canonical_materializer_preserves_prompt_when_cursor_redraw_split_across_chunks() {
    let prompt = "khoa2807@192 ~ % ";
    let cursor_forward = format!("\r\x1b[{}C", prompt.chars().count());
    let first = materialize_output_chunk(prompt, prompt.chars().count(), false, &cursor_forward);
    assert_eq!(first.open_fragment, prompt);
    assert_eq!(first.cursor_col, prompt.chars().count());

    let second = materialize_output_chunk(
        &first.open_fragment,
        first.cursor_col,
        first.pending_carriage_return,
        "ád\r\n",
    );

    assert_eq!(
        second.records,
        vec![
            StoredScrollbackRecordInput {
                ord: 0,
                kind: StoredScrollbackRecordKind::Line,
                text: "khoa2807@192 ~ % ád".to_string(),
            },
            StoredScrollbackRecordInput {
                ord: 1,
                kind: StoredScrollbackRecordKind::Fragment,
                text: "".to_string(),
            },
        ]
    );
}

#[test]
fn canonical_materializer_preserves_prompt_from_real_zsh_trace() {
    let prompt_chunk = "\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@192 chatminal % \x1b[K\x1b[?2004h";
    let command_chunk = "s\x08sdf\x1b[?2004l\r\r\nzsh: command not found: sdf\r\n\x1b[0m\x1b[27m\x1b[24m\x1b[Jkhoa2807@192 chatminal % \x1b[K\x1b[?2004h";

    let first = materialize_output_chunk("", 0, false, prompt_chunk);
    assert_eq!(first.open_fragment, "khoa2807@192 chatminal % ");

    let second = materialize_output_chunk(
        &first.open_fragment,
        first.cursor_col,
        first.pending_carriage_return,
        command_chunk,
    );

    assert_eq!(
        second.records,
        vec![
            StoredScrollbackRecordInput {
                ord: 0,
                kind: StoredScrollbackRecordKind::Line,
                text: "khoa2807@192 chatminal % sdf".to_string(),
            },
            StoredScrollbackRecordInput {
                ord: 1,
                kind: StoredScrollbackRecordKind::Line,
                text: "zsh: command not found: sdf".to_string(),
            },
            StoredScrollbackRecordInput {
                ord: 2,
                kind: StoredScrollbackRecordKind::Fragment,
                text: "khoa2807@192 chatminal % ".to_string(),
            },
        ]
    );
}

#[test]
fn canonical_materializer_replaces_changed_prompt_redraw_on_same_line() {
    let previous_prompt = "khoa2807@192 ~ % ";
    let next_prompt = "khoa2807@MacBook-Pro-cua-Khoa ~ % ";
    let redraw = format!("\r\x1b[{}C{}", previous_prompt.chars().count(), next_prompt);

    let materialized = materialize_output_chunk(
        previous_prompt,
        previous_prompt.chars().count(),
        false,
        &redraw,
    );

    assert_eq!(materialized.records.len(), 1);
    assert_eq!(materialized.records[0].text, next_prompt);
    assert_eq!(materialized.open_fragment, next_prompt);
    assert_eq!(materialized.cursor_col, next_prompt.chars().count());
    assert!(!materialized.pending_carriage_return);
}

#[test]
fn canonical_materializer_collapses_repeated_prompt_redraws_on_same_line() {
    let previous_prompt = "khoa2807@192 ~ % ";
    let next_prompt = "khoa2807@MacBook-Pro-cua-Khoa ~ % ";
    let redraw = format!(
        "\r\x1b[{}C{}{}",
        previous_prompt.chars().count(),
        next_prompt,
        next_prompt
    );

    let materialized = materialize_output_chunk(
        previous_prompt,
        previous_prompt.chars().count(),
        false,
        &redraw,
    );

    assert_eq!(materialized.records.len(), 1);
    assert_eq!(materialized.records[0].text, next_prompt);
    assert_eq!(materialized.open_fragment, next_prompt);
    assert_eq!(materialized.cursor_col, next_prompt.chars().count());
    assert!(!materialized.pending_carriage_return);
}

#[test]
fn logical_snapshot_backfills_missing_legacy_sequences_into_canonical_records() {
    let (state, session_id, _db) = create_state_with_session();
    let store = {
        let inner = state.inner.lock().expect("lock state");
        inner.store.clone()
    };

    store
        .append_scrollback_chunk(&session_id, 1, "line-1\n", 100)
        .expect("append legacy seq1");
    store
        .append_scrollback_chunk(&session_id, 2, "legacy-2\n", 101)
        .expect("append legacy seq2");
    store
        .append_scrollback_records(
            &session_id,
            2,
            &[
                StoredScrollbackRecordInput {
                    ord: 0,
                    kind: StoredScrollbackRecordKind::Line,
                    text: "canon-2".to_string(),
                },
                StoredScrollbackRecordInput {
                    ord: 1,
                    kind: StoredScrollbackRecordKind::Fragment,
                    text: "prompt % ".to_string(),
                },
            ],
            101,
        )
        .expect("append canonical seq2");
    store
        .append_scrollback_chunk(&session_id, 3, "cmd\n", 102)
        .expect("append legacy seq3");

    let snapshot = render_snapshot(
        &build_logical_snapshot(&store, &session_id).expect("build logical snapshot"),
        None,
    );
    assert_eq!(snapshot.seq, 3);
    assert_eq!(snapshot.content, "line-1\ncanon-2\nprompt % cmd\n");
    let canonical = store
        .list_scrollback_records(&session_id)
        .expect("list canonical records after backfill");
    let migrated_seq1 = canonical.iter().any(|record| {
        record.seq == 1
            && record.kind == StoredScrollbackRecordKind::Line
            && record.text == "line-1"
    });
    let migrated_seq3 = canonical.iter().any(|record| {
        record.seq == 3
            && record.kind == StoredScrollbackRecordKind::Line
            && record.text == "prompt % cmd"
    });
    assert!(
        migrated_seq1,
        "seq1 should be backfilled into canonical records"
    );
    assert!(
        migrated_seq3,
        "seq3 should be backfilled into canonical records"
    );
    assert!(
        store
            .list_legacy_scrollback_chunks(&session_id)
            .expect("legacy chunks after backfill")
            .is_empty(),
        "legacy chunks should be cleared after migration"
    );
}

#[test]
fn logical_snapshot_orders_multiple_records_within_same_seq_by_ord() {
    let (state, session_id, _db) = create_state_with_session();
    let store = {
        let inner = state.inner.lock().expect("lock state");
        inner.store.clone()
    };

    store
        .append_scrollback_records(
            &session_id,
            7,
            &[
                StoredScrollbackRecordInput {
                    ord: 0,
                    kind: StoredScrollbackRecordKind::Line,
                    text: "line-a".to_string(),
                },
                StoredScrollbackRecordInput {
                    ord: 1,
                    kind: StoredScrollbackRecordKind::Line,
                    text: "line-b".to_string(),
                },
                StoredScrollbackRecordInput {
                    ord: 2,
                    kind: StoredScrollbackRecordKind::Fragment,
                    text: "tail".to_string(),
                },
            ],
            100,
        )
        .expect("append canonical seq");

    let snapshot = render_snapshot(
        &build_logical_snapshot(&store, &session_id).expect("build logical snapshot"),
        None,
    );
    assert_eq!(snapshot.seq, 7);
    assert_eq!(snapshot.content, "line-a\nline-b\ntail");
}

#[test]
fn logical_snapshot_repairs_prompt_only_line_followed_by_command_without_prefix() {
    let (state, session_id, _db) = create_state_with_session();
    let store = {
        let inner = state.inner.lock().expect("lock state");
        inner.store.clone()
    };

    store
        .append_scrollback_records(
            &session_id,
            1,
            &[StoredScrollbackRecordInput {
                ord: 0,
                kind: StoredScrollbackRecordKind::Line,
                text: "khoa2807@192 ~ % ".to_string(),
            }],
            100,
        )
        .expect("append prompt-only line");
    store
        .append_scrollback_records(
            &session_id,
            2,
            &[
                StoredScrollbackRecordInput {
                    ord: 0,
                    kind: StoredScrollbackRecordKind::Fragment,
                    text: "".to_string(),
                },
                StoredScrollbackRecordInput {
                    ord: 1,
                    kind: StoredScrollbackRecordKind::Fragment,
                    text: "ád".to_string(),
                },
            ],
            101,
        )
        .expect("append command fragment");
    store
        .append_scrollback_records(
            &session_id,
            3,
            &[
                StoredScrollbackRecordInput {
                    ord: 0,
                    kind: StoredScrollbackRecordKind::Line,
                    text: "ád".to_string(),
                },
                StoredScrollbackRecordInput {
                    ord: 1,
                    kind: StoredScrollbackRecordKind::Fragment,
                    text: "".to_string(),
                },
            ],
            102,
        )
        .expect("append command line");

    let snapshot = render_snapshot(
        &build_logical_snapshot(&store, &session_id).expect("build logical snapshot"),
        None,
    );
    assert_eq!(snapshot.content, "khoa2807@192 ~ % ád\n");
}

#[test]
fn logical_snapshot_repairs_prompt_only_line_into_open_fragment_when_command_still_typing() {
    let (state, session_id, _db) = create_state_with_session();
    let store = {
        let inner = state.inner.lock().expect("lock state");
        inner.store.clone()
    };

    store
        .append_scrollback_records(
            &session_id,
            1,
            &[StoredScrollbackRecordInput {
                ord: 0,
                kind: StoredScrollbackRecordKind::Line,
                text: "khoa2807@192 ~ % ".to_string(),
            }],
            100,
        )
        .expect("append prompt-only line");
    store
        .append_scrollback_records(
            &session_id,
            2,
            &[StoredScrollbackRecordInput {
                ord: 0,
                kind: StoredScrollbackRecordKind::Fragment,
                text: "fds".to_string(),
            }],
            101,
        )
        .expect("append typing fragment");

    let snapshot = render_snapshot(
        &build_logical_snapshot(&store, &session_id).expect("build logical snapshot"),
        None,
    );
    assert_eq!(snapshot.content, "khoa2807@192 ~ % fds");
}

#[test]
fn terminal_restore_snapshot_uses_crlf_between_committed_lines() {
    let snapshot = render_snapshot_for_terminal(&super::canonical_scrollback::LogicalSnapshot {
        lines: vec!["line-1".to_string(), "line-2".to_string()],
        open_fragment: "prompt % ".to_string(),
        seq: 2,
    });

    assert_eq!(snapshot.content, "line-1\r\nline-2\r\nprompt % ");
}

#[test]
fn persisted_output_writes_canonical_records_not_legacy_chunks() {
    let (state, session_id, _db) = create_state_with_session();
    {
        let mut inner = state.inner.lock().expect("lock state");
        let entry = inner
            .sessions
            .get_mut(&session_id)
            .expect("session entry exists");
        entry.session.persist_history = true;
        entry.session.status = StoredSessionStatus::Running;
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: Arc::from(session_id.as_str()),
        generation: 0,
        chunk: "echo hi\nprompt % ".to_string(),
        ts: 1,
    });

    state.flush_persist();
    let store = {
        let inner = state.inner.lock().expect("lock state");
        inner.store.clone()
    };
    assert!(
        store
            .list_scrollback_records(&session_id)
            .expect("list canonical records")
            .len()
            >= 2
    );
    assert!(
        store
            .list_legacy_scrollback_chunks(&session_id)
            .expect("list legacy chunks")
            .is_empty()
    );
}

#[test]
fn workspace_history_clear_all_resets_all_sessions() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();
    for session_id in [&session_a, &session_b] {
        state
            .session_activate(session_id, 120, 32)
            .expect("activate session");
        state.apply_session_event(SessionEvent::Output {
            session_id: Arc::from(session_id.as_str()),
            generation: 1,
            chunk: format!("output-{session_id}\n"),
            ts: 1,
        });
    }

    state
        .workspace_history_clear_all()
        .expect("clear workspace history");
    let inner = state.inner.lock().expect("lock state");
    for session_id in [&session_a, &session_b] {
        let entry = inner
            .sessions
            .get(session_id)
            .expect("session entry exists");
        assert_eq!(entry.session.status, StoredSessionStatus::Disconnected);
        assert_eq!(entry.session.seq, 0);
        assert!(entry.runtime.is_none());

        let snapshot = inner
            .store
            .session_snapshot(session_id, 100)
            .expect("snapshot after workspace clear");
        assert_eq!(snapshot.seq, 0);
        assert!(snapshot.content.is_empty());
    }
}

#[test]
fn clear_all_data_resets_workspace_and_clears_layout_registry() {
    let (state, session_a, _session_b, _db) = create_state_with_two_sessions();
    let layout = WorkspaceLayoutState::new_single(session_a.clone());
    state
        .workspace_layout_replace("desktop-main", layout)
        .expect("persist layout");

    let workspace = state.clear_all_data().expect("clear all data");
    assert_eq!(workspace.profiles.len(), 1);
    assert_eq!(workspace.profiles[0].name, "Default");
    assert_eq!(
        workspace.active_profile_id.as_deref(),
        Some(workspace.profiles[0].profile_id.as_str())
    );
    assert!(workspace.sessions.is_empty());
    assert!(workspace.active_session_id.is_none());
    assert!(
        state
            .workspace_layout_snapshot("desktop-main")
            .expect("snapshot layout after clear")
            .is_none()
    );

    let inner = state.inner.lock().expect("lock state");
    assert!(inner.sessions.is_empty());
}

#[test]
fn normalize_relative_path_rejects_parent_component() {
    let err = normalize_relative_path("../etc/passwd").expect_err("parent path must fail");
    assert!(err.contains("parent path"));
}

#[test]
fn normalize_relative_path_strips_curdir_and_windows_separators() {
    let normalized = normalize_relative_path("./src\\main.rs").expect("normalize path");
    assert_eq!(normalized, "src/main.rs");
}

#[cfg(unix)]
#[test]
fn resolve_explorer_target_handles_symlink_alias_and_blocks_escape() {
    use std::os::unix::fs::symlink;

    let base = std::env::temp_dir().join(format!(
        "chatminal-explorer-symlink-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0)
    ));
    let root = base.join("root");
    let nested = root.join("nested");
    let outside = base.join("outside");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&nested).expect("create nested root dir");
    std::fs::create_dir_all(&outside).expect("create outside dir");
    std::fs::write(nested.join("inside.txt"), "ok").expect("write inside file");

    symlink(&nested, root.join("alias")).expect("create alias symlink inside root");
    symlink(&outside, root.join("escape")).expect("create escape symlink");

    let canonical_root = std::fs::canonicalize(&root).expect("canonicalize root");
    let resolved_alias = resolve_explorer_target(&root, "alias/inside.txt")
        .expect("alias path inside root should resolve");
    assert!(resolved_alias.starts_with(&canonical_root));

    let escape_err =
        resolve_explorer_target(&root, "escape").expect_err("symlink escape must be rejected");
    assert!(escape_err.contains("escapes selected root"));

    let _ = std::fs::remove_dir_all(&base);
}
