use chatminal_protocol::{
    DaemonHealthEvent, Event, ProfileInfo, PtyOutputEvent, SessionInfo, SessionStatus,
    SessionUpdatedEvent, WorkspaceState, WorkspaceUpdatedEvent,
};

use crate::terminal_pane_adapter::SessionPaneRegistry;
use crate::terminal_wezterm_core::WeztermTerminalPaneAdapter;
use crate::terminal_workspace_binding_runtime::{
    WorkspaceBindingState, apply_event_to_workspace_binding_state,
};

fn sample_state() -> WorkspaceBindingState {
    WorkspaceBindingState {
        workspace: WorkspaceState {
            profiles: vec![ProfileInfo {
                profile_id: "p1".to_string(),
                name: "Default".to_string(),
            }],
            active_profile_id: Some("p1".to_string()),
            sessions: vec![SessionInfo {
                session_id: "s1".to_string(),
                profile_id: "p1".to_string(),
                name: "Main".to_string(),
                cwd: "/tmp".to_string(),
                status: SessionStatus::Disconnected,
                persist_history: true,
                seq: 0,
            }],
            active_session_id: Some("s1".to_string()),
        },
        adapter: WeztermTerminalPaneAdapter::new(120, 32, 5_000),
        hydrate_errors: Vec::new(),
        stale: false,
        event_watermark_ts: 0,
        last_workspace_event_ts: 0,
        session_last_event_ts: std::collections::HashMap::from([("s1".to_string(), 0)]),
    }
}

fn sample_state_with_two_sessions() -> WorkspaceBindingState {
    WorkspaceBindingState {
        workspace: WorkspaceState {
            profiles: vec![ProfileInfo {
                profile_id: "p1".to_string(),
                name: "Default".to_string(),
            }],
            active_profile_id: Some("p1".to_string()),
            sessions: vec![
                SessionInfo {
                    session_id: "s1".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Main".to_string(),
                    cwd: "/tmp".to_string(),
                    status: SessionStatus::Running,
                    persist_history: true,
                    seq: 0,
                },
                SessionInfo {
                    session_id: "s2".to_string(),
                    profile_id: "p1".to_string(),
                    name: "Secondary".to_string(),
                    cwd: "/tmp".to_string(),
                    status: SessionStatus::Disconnected,
                    persist_history: false,
                    seq: 0,
                },
            ],
            active_session_id: Some("s1".to_string()),
        },
        adapter: WeztermTerminalPaneAdapter::new(120, 32, 5_000),
        hydrate_errors: Vec::new(),
        stale: false,
        event_watermark_ts: 0,
        last_workspace_event_ts: 0,
        session_last_event_ts: std::collections::HashMap::from([
            ("s1".to_string(), 0),
            ("s2".to_string(), 0),
        ]),
    }
}

#[test]
fn output_event_updates_known_session_state_without_marking_stale() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::PtyOutput(PtyOutputEvent {
            session_id: "s1".to_string(),
            chunk: "hello\n".to_string(),
            seq: 5,
            ts: 1,
        }),
    );

    let session = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("session exists");
    assert_eq!(session.seq, 5);
    assert_eq!(session.status, SessionStatus::Running);
    assert!(!state.is_stale());
}

#[test]
fn unknown_session_output_marks_state_stale() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::PtyOutput(PtyOutputEvent {
            session_id: "missing".to_string(),
            chunk: "hello\n".to_string(),
            seq: 1,
            ts: 1,
        }),
    );
    assert!(state.is_stale());
}

#[test]
fn unknown_session_output_with_older_timestamp_than_watermark_is_ignored() {
    let mut state = sample_state();
    state.event_watermark_ts = 100;
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::PtyOutput(PtyOutputEvent {
            session_id: "missing".to_string(),
            chunk: "hello\n".to_string(),
            seq: 1,
            ts: 90,
        }),
    );
    assert!(!state.is_stale());
}

#[test]
fn workspace_count_mismatch_marks_state_stale() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::WorkspaceUpdated(WorkspaceUpdatedEvent {
            active_profile_id: Some("p1".to_string()),
            active_session_id: Some("s1".to_string()),
            profile_count: 1,
            session_count: 2,
            ts: 1,
        }),
    );
    assert!(state.is_stale());
}

#[test]
fn workspace_updated_with_same_counts_still_marks_state_stale() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::WorkspaceUpdated(WorkspaceUpdatedEvent {
            active_profile_id: Some("p1".to_string()),
            active_session_id: Some("s1".to_string()),
            profile_count: 1,
            session_count: 1,
            ts: 1,
        }),
    );
    assert!(state.is_stale());
}

#[test]
fn session_updated_for_missing_session_marks_state_stale() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "missing".to_string(),
            status: SessionStatus::Running,
            seq: 3,
            persist_history: true,
            ts: 1,
        }),
    );
    assert!(state.is_stale());
}

#[test]
fn session_updated_for_missing_session_with_older_timestamp_is_ignored() {
    let mut state = sample_state();
    state.event_watermark_ts = 200;
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "missing".to_string(),
            status: SessionStatus::Running,
            seq: 3,
            persist_history: true,
            ts: 150,
        }),
    );
    assert!(!state.is_stale());
}

#[test]
fn session_updated_for_known_session_refreshes_status_without_stale() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "s1".to_string(),
            status: SessionStatus::Running,
            seq: 9,
            persist_history: false,
            ts: 1,
        }),
    );

    let session = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("session exists");
    assert_eq!(session.status, SessionStatus::Running);
    assert_eq!(session.seq, 9);
    assert!(!session.persist_history);
    assert!(!state.is_stale());
}

#[test]
fn session_updated_out_of_order_does_not_decrease_seq() {
    let mut state = sample_state();
    state.workspace.sessions[0].seq = 10;
    let mut registry = SessionPaneRegistry::new();

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "s1".to_string(),
            status: SessionStatus::Running,
            seq: 3,
            persist_history: true,
            ts: 1,
        }),
    );

    let session = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("session exists");
    assert_eq!(session.seq, 10);
    assert_eq!(session.status, SessionStatus::Running);
    assert!(!state.is_stale());
}

#[test]
fn session_updated_with_older_timestamp_is_ignored() {
    let mut state = sample_state();
    state.workspace.sessions[0].seq = 10;
    state.workspace.sessions[0].persist_history = false;
    state.session_last_event_ts.insert("s1".to_string(), 100);
    let mut registry = SessionPaneRegistry::new();

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "s1".to_string(),
            status: SessionStatus::Running,
            seq: 20,
            persist_history: true,
            ts: 90,
        }),
    );

    let session = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("session exists");
    assert_eq!(session.seq, 10);
    assert!(!session.persist_history);
    assert!(!state.is_stale());
}

#[test]
fn pty_exited_for_known_session_switches_to_disconnected() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::PtyExited(chatminal_protocol::PtyExitedEvent {
            session_id: "s1".to_string(),
            exit_code: Some(0),
            reason: "done".to_string(),
        }),
    );

    let session = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("session exists");
    assert_eq!(session.status, SessionStatus::Disconnected);
    assert!(!state.is_stale());
}

#[test]
fn pty_exited_for_missing_session_is_ignored_without_stale() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::PtyExited(chatminal_protocol::PtyExitedEvent {
            session_id: "missing".to_string(),
            exit_code: Some(0),
            reason: "stale".to_string(),
        }),
    );

    assert!(!state.is_stale());
    let session = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("session exists");
    assert_eq!(session.status, SessionStatus::Disconnected);
}

#[test]
fn daemon_health_event_is_non_mutating() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();
    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::DaemonHealth(DaemonHealthEvent {
            connected_clients: 1,
            session_count: 1,
            running_sessions: 0,
            ts: 1,
        }),
    );
    assert!(!state.is_stale());
}

#[test]
fn workspace_switch_event_marks_state_stale_for_reconnect_rehydrate() {
    let mut state = sample_state_with_two_sessions();
    let mut registry = SessionPaneRegistry::new();

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::PtyOutput(PtyOutputEvent {
            session_id: "s1".to_string(),
            chunk: "echo old\n".to_string(),
            seq: 4,
            ts: 1,
        }),
    );
    assert!(!state.is_stale());

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::WorkspaceUpdated(WorkspaceUpdatedEvent {
            active_profile_id: Some("p1".to_string()),
            active_session_id: Some("s2".to_string()),
            profile_count: 1,
            session_count: 2,
            ts: 2,
        }),
    );
    assert!(state.is_stale());
    assert_eq!(state.workspace.active_session_id.as_deref(), Some("s2"));
}

#[test]
fn workspace_updated_with_older_timestamp_is_ignored() {
    let mut state = sample_state_with_two_sessions();
    state.last_workspace_event_ts = 200;
    state.workspace.active_session_id = Some("s1".to_string());
    let mut registry = SessionPaneRegistry::new();

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::WorkspaceUpdated(WorkspaceUpdatedEvent {
            active_profile_id: Some("p1".to_string()),
            active_session_id: Some("s2".to_string()),
            profile_count: 1,
            session_count: 2,
            ts: 150,
        }),
    );

    assert_eq!(state.workspace.active_session_id.as_deref(), Some("s1"));
    assert!(!state.is_stale());
}

#[test]
fn session_timestamp_guard_is_scoped_per_session() {
    let mut state = sample_state_with_two_sessions();
    let mut registry = SessionPaneRegistry::new();

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::PtyOutput(PtyOutputEvent {
            session_id: "s1".to_string(),
            chunk: "fresh\n".to_string(),
            seq: 12,
            ts: 200,
        }),
    );

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "s1".to_string(),
            status: SessionStatus::Disconnected,
            seq: 99,
            persist_history: false,
            ts: 150,
        }),
    );

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "s2".to_string(),
            status: SessionStatus::Running,
            seq: 7,
            persist_history: true,
            ts: 150,
        }),
    );

    let s1 = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("s1 exists");
    assert_eq!(s1.status, SessionStatus::Running);
    assert_eq!(s1.seq, 12);
    assert!(s1.persist_history);

    let s2 = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s2")
        .expect("s2 exists");
    assert_eq!(s2.status, SessionStatus::Running);
    assert_eq!(s2.seq, 7);
    assert!(s2.persist_history);
}

#[test]
fn stale_reconnect_session_update_is_ignored_after_newer_output() {
    let mut state = sample_state();
    let mut registry = SessionPaneRegistry::new();

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "s1".to_string(),
            status: SessionStatus::Running,
            seq: 10,
            persist_history: false,
            ts: 300,
        }),
    );

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::PtyOutput(PtyOutputEvent {
            session_id: "s1".to_string(),
            chunk: "new generation\n".to_string(),
            seq: 11,
            ts: 400,
        }),
    );

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "s1".to_string(),
            status: SessionStatus::Disconnected,
            seq: 20,
            persist_history: true,
            ts: 350,
        }),
    );

    let session = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("s1 exists");
    assert_eq!(session.status, SessionStatus::Running);
    assert_eq!(session.seq, 11);
    assert!(!session.persist_history);
    assert!(!state.is_stale());
}

#[test]
fn session_updated_with_same_timestamp_still_applies_latest_payload() {
    let mut state = sample_state();
    state.workspace.sessions[0].seq = 10;
    state.workspace.sessions[0].persist_history = false;
    state.session_last_event_ts.insert("s1".to_string(), 300);
    let mut registry = SessionPaneRegistry::new();

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::SessionUpdated(SessionUpdatedEvent {
            session_id: "s1".to_string(),
            status: SessionStatus::Running,
            seq: 12,
            persist_history: true,
            ts: 300,
        }),
    );

    let session = state
        .workspace
        .sessions
        .iter()
        .find(|value| value.session_id == "s1")
        .expect("s1 exists");
    assert_eq!(session.status, SessionStatus::Running);
    assert_eq!(session.seq, 12);
    assert!(session.persist_history);
    assert!(!state.is_stale());
}

#[test]
fn workspace_updated_with_same_timestamp_is_applied() {
    let mut state = sample_state_with_two_sessions();
    state.last_workspace_event_ts = 500;
    state.workspace.active_session_id = Some("s1".to_string());
    let mut registry = SessionPaneRegistry::new();

    apply_event_to_workspace_binding_state(
        &mut state,
        &mut registry,
        Event::WorkspaceUpdated(WorkspaceUpdatedEvent {
            active_profile_id: Some("p1".to_string()),
            active_session_id: Some("s2".to_string()),
            profile_count: 1,
            session_count: 2,
            ts: 500,
        }),
    );

    assert_eq!(state.workspace.active_session_id.as_deref(), Some("s2"));
    assert!(state.is_stale());
}
