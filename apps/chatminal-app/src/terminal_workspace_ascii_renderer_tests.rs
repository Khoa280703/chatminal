use crate::session_terminal_emulator::TerminalSnapshotSummary;
use crate::terminal_workspace_ascii_renderer::{
    fit_dashboard_for_terminal, render_terminal_workspace_ascii,
};
use crate::terminal_workspace_view_model::{
    SidebarProfileItem, SidebarSessionItem, TerminalWorkspaceViewModel,
};

#[test]
fn renders_active_terminal_preview_with_tail_limit() {
    let view_model = TerminalWorkspaceViewModel {
        profiles: vec![SidebarProfileItem {
            profile_id: "p-1".to_string(),
            name: "Default".to_string(),
            is_active: true,
        }],
        sessions: vec![SidebarSessionItem {
            session_id: "s-1".to_string(),
            profile_id: "p-1".to_string(),
            name: "Main".to_string(),
            status: "running".to_string(),
            is_active: true,
            terminal_id: Some("terminal-1".to_string()),
        }],
        active_profile_id: Some("p-1".to_string()),
        active_session_id: Some("s-1".to_string()),
        active_terminal_id: Some("terminal-1".to_string()),
        status_line: "profiles=1 sessions=1 terminals=1".to_string(),
    };

    let terminal_snapshots = vec![TerminalSnapshotSummary {
        terminal_id: "terminal-1".to_string(),
        session_id: "s-1".to_string(),
        cols: 120,
        rows: 32,
        visible_text: "line-1\nline-2\nline-3\nline-4".to_string(),
    }];

    let rendered = render_terminal_workspace_ascii(&view_model, &terminal_snapshots, 2);
    assert!(rendered.contains("Status: profiles=1 sessions=1 terminals=1"));
    assert!(rendered.contains("Active Terminal:"));
    assert!(rendered.contains("line-3"));
    assert!(rendered.contains("line-4"));
    assert!(!rendered.contains("line-1"));
}

#[test]
fn renders_none_when_active_terminal_missing() {
    let view_model = TerminalWorkspaceViewModel {
        profiles: Vec::new(),
        sessions: Vec::new(),
        active_profile_id: None,
        active_session_id: None,
        active_terminal_id: None,
        status_line: "profiles=0 sessions=0 terminals=0".to_string(),
    };

    let rendered = render_terminal_workspace_ascii(&view_model, &[], 20);
    assert!(rendered.contains("Active Terminal:"));
    assert!(rendered.contains("(none)"));
}

#[test]
fn fit_dashboard_for_terminal_truncates_by_size() {
    let input = "1234567890ABCDEFG\nline-2\nline-3";
    let rendered = fit_dashboard_for_terminal(input, 10, 3);
    let lines = rendered.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "123456789…");
    assert_eq!(lines[1], "line-2");
}

#[test]
fn fit_dashboard_for_terminal_normalizes_carriage_returns() {
    let input = "progress 10%\rprogress 40%\rprogress 100%\nready";
    let rendered = fit_dashboard_for_terminal(input, 40, 10);
    assert!(rendered.contains("progress 10%"));
    assert!(rendered.contains("progress 40%"));
    assert!(rendered.contains("progress 100%"));
    assert!(rendered.contains("ready"));
}
