use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use chatminal_protocol::{ClientFrame, Request, Response, ServerBody};
use chatminal_session_runtime::{SessionBridgeAction, SessionSurfaceLookup};
use chatminal_store::{Store, StoredSessionStatus};

use crate::api::RuntimeEvent;
use crate::config::DaemonConfig;
use crate::session::SessionEvent;

use super::explorer_utils::{normalize_relative_path, resolve_explorer_target};
use super::{
    DaemonState, SessionSpawnPlan, prepend_run_boundary, snapshot_requires_run_boundary,
    trim_live_output,
};

struct TempDb {
    path: PathBuf,
}

impl TempDb {
    fn new() -> Self {
        static NEXT_TEMP_DB_ID: AtomicU64 = AtomicU64::new(1);
        let path = std::env::temp_dir().join(format!(
            "chatminald-state-test-{}-{}-{}.db",
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

fn create_state_with_session() -> (DaemonState, String, TempDb) {
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

    let config = DaemonConfig {
        endpoint: format!(
            "/tmp/chatminald-state-{}-{}.sock",
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
    let state = DaemonState::new(config, store).expect("create daemon state");
    (state, session.session_id, db)
}

fn create_state_with_two_sessions() -> (DaemonState, String, String, TempDb) {
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

    let config = DaemonConfig {
        endpoint: format!(
            "/tmp/chatminald-state-two-{}-{}.sock",
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
    let state = DaemonState::new(config, store).expect("create daemon state");
    (state, session_a.session_id, session_b.session_id, db)
}

fn request_ok(state: &DaemonState, request: Request) -> Response {
    let frame = state.handle_request(ClientFrame {
        id: "test-request".to_string(),
        request,
    });
    match frame.body {
        ServerBody::Response {
            ok: true,
            response: Some(response),
            ..
        } => response,
        ServerBody::Response {
            ok: false,
            error: Some(error),
            ..
        } => panic!("request failed unexpectedly: {error}"),
        other => panic!("unexpected frame body: {other:?}"),
    }
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
        &chatminal_store::StoredSessionSnapshot {
            content: "khoa2807@host ~ % ".to_string(),
            seq: 1,
        }
    ));
    assert!(!snapshot_requires_run_boundary(
        &chatminal_store::StoredSessionSnapshot {
            content: "line\n".to_string(),
            seq: 1,
        }
    ));
}

#[test]
fn prepend_run_boundary_only_when_chunk_needs_it() {
    assert_eq!(prepend_run_boundary("prompt"), "\r\nprompt");
    assert_eq!(prepend_run_boundary("\nprompt"), "\nprompt");
    assert_eq!(prepend_run_boundary("\rprompt"), "\rprompt");
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
        session_id: session_id.clone(),
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
        session_id: session_id.clone(),
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
fn metrics_snapshot_tracks_requests_and_errors() {
    let (state, _session_id, _db) = create_state_with_session();

    let _ = request_ok(&state, Request::Ping);
    let error_frame = state.handle_request(ClientFrame {
        id: "invalid-request".to_string(),
        request: Request::SessionInputWrite {
            session_id: "missing-session".to_string(),
            data: "echo test".to_string(),
        },
    });
    match error_frame.body {
        ServerBody::Response { ok: false, .. } => {}
        other => panic!("expected error response frame, got {other:?}"),
    }

    let snapshot = state.metrics_snapshot();
    assert!(snapshot.requests_total >= 2);
    assert!(snapshot.request_errors_total >= 1);
}

#[test]
fn metrics_snapshot_tracks_dropped_clients_on_full_queue() {
    let (state, _session_id, _db) = create_state_with_session();
    let (tx, _rx) = mpsc::sync_channel(1);
    state.register_client(42, tx);

    state.broadcast_daemon_health();
    state.broadcast_daemon_health();

    let snapshot = state.metrics_snapshot();
    assert!(snapshot.broadcast_frames_total >= 2);
    assert!(snapshot.dropped_clients_full_total >= 1);
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
fn native_subscription_receives_daemon_health_event() {
    let (state, _session_id, _db) = create_state_with_session();
    let subscription = state.subscribe().expect("subscribe runtime events");

    state.broadcast_daemon_health();

    let event = subscription
        .recv_timeout(Duration::from_secs(1))
        .expect("receive event")
        .expect("event payload");
    match event {
        RuntimeEvent::DaemonHealth(value) => {
            assert!(value.session_count >= 1);
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
fn workspace_load_auto_connects_active_session_runtime() {
    let (state, session_id, _db) = create_state_with_session();

    let response = request_ok(&state, Request::WorkspaceLoad);
    let workspace = match response {
        Response::Workspace(value) => value,
        other => panic!("unexpected response: {other:?}"),
    };
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

    let _ = request_ok(&state, Request::AppShutdown);
}

#[test]
fn workspace_load_passive_keeps_active_session_disconnected() {
    let (state, session_id, _db) = create_state_with_session();

    let response = request_ok(&state, Request::WorkspaceLoadPassive);
    let workspace = match response {
        Response::Workspace(value) => value,
        other => panic!("unexpected response: {other:?}"),
    };
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
fn reconcile_session_surface_lookup_prefers_runtime_active_session() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();
    let lookup = SessionSurfaceLookup {
        active_session_id: Some(session_b),
        ..SessionSurfaceLookup::default()
    };

    let action = state
        .reconcile_session_surface_lookup(&lookup)
        .expect("reconcile session surface lookup");

    assert_eq!(
        action,
        SessionBridgeAction::FocusSurface {
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

    let _ = request_ok(
        &state,
        Request::SessionActivate {
            session_id: session_id.clone(),
            cols: 120,
            rows: 32,
        },
    );
    let generation_after_first = {
        let inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get(&session_id)
            .expect("session entry exists")
            .generation
    };
    assert_eq!(generation_after_first, generation_before.saturating_add(1));

    let _ = request_ok(
        &state,
        Request::SessionHistoryClear {
            session_id: session_id.clone(),
        },
    );
    let _ = request_ok(
        &state,
        Request::SessionActivate {
            session_id: session_id.clone(),
            cols: 120,
            rows: 32,
        },
    );
    let generation_after_second = {
        let inner = state.inner.lock().expect("lock state");
        inner
            .sessions
            .get(&session_id)
            .expect("session entry exists")
            .generation
    };
    assert!(generation_after_second > generation_after_first);

    let _ = request_ok(&state, Request::AppShutdown);
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

    let _ = request_ok(
        &state,
        Request::SessionActivate {
            session_id: session_id.clone(),
            cols: 80,
            rows: 24,
        },
    );

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

    let _ = request_ok(
        &state,
        Request::SessionActivate {
            session_id: session_id.clone(),
            cols: 132,
            rows: 43,
        },
    );

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

    let _ = request_ok(&state, Request::AppShutdown);
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

    let _ = request_ok(
        &state,
        Request::SessionActivate {
            session_id: session_id.clone(),
            cols: 120,
            rows: 32,
        },
    );
    let _ = request_ok(
        &state,
        Request::SessionSetPersist {
            session_id: session_id.clone(),
            persist_history: true,
        },
    );

    state.apply_session_event(SessionEvent::Output {
        session_id: session_id.clone(),
        generation: 0,
        chunk: "echo hello\n".to_string(),
        ts: 11,
    });

    let _ = request_ok(
        &state,
        Request::SessionHistoryClear {
            session_id: session_id.clone(),
        },
    );

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

    let _ = request_ok(
        &state,
        Request::SessionSetPersist {
            session_id: session_id.clone(),
            persist_history: true,
        },
    );

    let inner = state.inner.lock().expect("lock state");
    let entry = inner
        .sessions
        .get(&session_id)
        .expect("session entry exists");
    assert!(entry.live_output.is_empty());
    assert!(entry.session.persist_history);
    assert_eq!(entry.session.seq, 1);
    let snapshot = inner
        .store
        .session_snapshot(&session_id, 100)
        .expect("load snapshot");
    assert_eq!(snapshot.content, "cached-line\n");
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
        session_id: session_id.clone(),
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
    }

    state.apply_session_event(SessionEvent::Output {
        session_id: session_id.clone(),
        generation: 1,
        chunk: "khoa2807@host ~ % ".to_string(),
        ts: 2,
    });

    let inner = state.inner.lock().expect("lock state");
    let snapshot = inner
        .store
        .session_snapshot(&session_id, 100)
        .expect("load snapshot");
    assert_eq!(snapshot.content, "khoa2807@host ~ % \r\nkhoa2807@host ~ % ");
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
        session_id: session_id.clone(),
        generation: 0,
        chunk: "l1\n".to_string(),
        ts: 1,
    });
    state.apply_session_event(SessionEvent::Output {
        session_id: session_id.clone(),
        generation: 0,
        chunk: "l2\n".to_string(),
        ts: 2,
    });
    state.apply_session_event(SessionEvent::Output {
        session_id: session_id.clone(),
        generation: 0,
        chunk: "l3\n".to_string(),
        ts: 3,
    });

    let inner = state.inner.lock().expect("lock state");
    let snapshot = inner
        .store
        .session_snapshot(&session_id, 100)
        .expect("load session snapshot");
    assert_eq!(snapshot.seq, 3);
    assert_eq!(snapshot.content, "l2\nl3\n");
}

#[test]
fn clear_history_generation_gate_ignores_old_output_after_reset() {
    let (state, session_id, _db) = create_state_with_session();
    let _ = request_ok(
        &state,
        Request::SessionActivate {
            session_id: session_id.clone(),
            cols: 120,
            rows: 32,
        },
    );
    state.apply_session_event(SessionEvent::Output {
        session_id: session_id.clone(),
        generation: 1,
        chunk: "before-clear\n".to_string(),
        ts: 1,
    });
    let _ = request_ok(
        &state,
        Request::SessionHistoryClear {
            session_id: session_id.clone(),
        },
    );

    state.apply_session_event(SessionEvent::Output {
        session_id: session_id.clone(),
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
fn workspace_history_clear_all_resets_all_sessions() {
    let (state, session_a, session_b, _db) = create_state_with_two_sessions();
    for session_id in [&session_a, &session_b] {
        let _ = request_ok(
            &state,
            Request::SessionActivate {
                session_id: session_id.to_string(),
                cols: 120,
                rows: 32,
            },
        );
        state.apply_session_event(SessionEvent::Output {
            session_id: session_id.to_string(),
            generation: 1,
            chunk: format!("output-{session_id}\n"),
            ts: 1,
        });
    }

    let _ = request_ok(&state, Request::WorkspaceHistoryClearAll);
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
        "chatminald-explorer-symlink-{}-{}",
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
