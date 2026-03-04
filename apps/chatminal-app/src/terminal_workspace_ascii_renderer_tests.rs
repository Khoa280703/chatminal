use crate::terminal_wezterm_core::PaneSnapshotSummary;
use crate::terminal_workspace_ascii_renderer::render_terminal_workspace_ascii;
use crate::terminal_workspace_view_model::{
    SidebarProfileItem, SidebarSessionItem, TerminalWorkspaceViewModel,
};

#[test]
fn renders_active_pane_preview_with_tail_limit() {
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
            pane_id: Some("pane-1".to_string()),
        }],
        active_profile_id: Some("p-1".to_string()),
        active_session_id: Some("s-1".to_string()),
        active_pane_id: Some("pane-1".to_string()),
        status_line: "profiles=1 sessions=1 panes=1".to_string(),
    };

    let pane_snapshots = vec![PaneSnapshotSummary {
        pane_id: "pane-1".to_string(),
        session_id: "s-1".to_string(),
        cols: 120,
        rows: 32,
        visible_text: "line-1\nline-2\nline-3\nline-4".to_string(),
    }];

    let rendered = render_terminal_workspace_ascii(&view_model, &pane_snapshots, 2);
    assert!(rendered.contains("Status: profiles=1 sessions=1 panes=1"));
    assert!(rendered.contains("Active Pane:"));
    assert!(rendered.contains("line-3"));
    assert!(rendered.contains("line-4"));
    assert!(!rendered.contains("line-1"));
}

#[test]
fn renders_none_when_active_pane_missing() {
    let view_model = TerminalWorkspaceViewModel {
        profiles: Vec::new(),
        sessions: Vec::new(),
        active_profile_id: None,
        active_session_id: None,
        active_pane_id: None,
        status_line: "profiles=0 sessions=0 panes=0".to_string(),
    };

    let rendered = render_terminal_workspace_ascii(&view_model, &[], 20);
    assert!(rendered.contains("Active Pane:"));
    assert!(rendered.contains("(none)"));
}
