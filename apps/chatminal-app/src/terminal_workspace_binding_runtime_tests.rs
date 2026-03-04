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
