use chatminal_protocol::{ProfileInfo, SessionInfo, SessionStatus, WorkspaceState};

use crate::session_terminal_emulator::TerminalSnapshotSummary;
use crate::terminal_workspace_view_model::build_terminal_workspace_view_model;

#[test]
fn builds_active_profile_session_and_terminal() {
    let workspace = WorkspaceState {
        profiles: vec![
            ProfileInfo {
                profile_id: "p-1".to_string(),
                name: "Default".to_string(),
            },
            ProfileInfo {
                profile_id: "p-2".to_string(),
                name: "Ops".to_string(),
            },
        ],
        active_profile_id: Some("p-1".to_string()),
        sessions: vec![SessionInfo {
            session_id: "s-1".to_string(),
            profile_id: "p-1".to_string(),
            name: "Main".to_string(),
            cwd: "/tmp".to_string(),
            status: SessionStatus::Running,
            persist_history: true,
            seq: 10,
        }],
        active_session_id: Some("s-1".to_string()),
    };
    let terminals = vec![TerminalSnapshotSummary {
        terminal_id: "terminal-1".to_string(),
        session_id: "s-1".to_string(),
        cols: 120,
        rows: 32,
        visible_text: "hello".to_string(),
    }];

    let view = build_terminal_workspace_view_model(&workspace, &terminals);
    assert_eq!(view.active_profile_id.as_deref(), Some("p-1"));
    assert_eq!(view.active_session_id.as_deref(), Some("s-1"));
    assert_eq!(view.active_terminal_id.as_deref(), Some("terminal-1"));
    assert!(view.status_line.contains("profiles=2"));
    assert!(view.status_line.contains("sessions=1"));
    assert!(view.status_line.contains("terminals=1"));
}

#[test]
fn active_terminal_is_none_when_session_has_no_terminal() {
    let workspace = WorkspaceState {
        profiles: vec![ProfileInfo {
            profile_id: "p-1".to_string(),
            name: "Default".to_string(),
        }],
        active_profile_id: Some("p-1".to_string()),
        sessions: vec![SessionInfo {
            session_id: "s-1".to_string(),
            profile_id: "p-1".to_string(),
            name: "Main".to_string(),
            cwd: "/tmp".to_string(),
            status: SessionStatus::Disconnected,
            persist_history: false,
            seq: 0,
        }],
        active_session_id: Some("s-1".to_string()),
    };
    let terminals = Vec::<TerminalSnapshotSummary>::new();

    let view = build_terminal_workspace_view_model(&workspace, &terminals);
    assert_eq!(view.active_terminal_id, None);
}
