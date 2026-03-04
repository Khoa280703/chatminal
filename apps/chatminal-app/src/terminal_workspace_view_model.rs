use std::collections::HashMap;

use chatminal_protocol::WorkspaceState;
use serde::Serialize;

use crate::terminal_wezterm_core::PaneSnapshotSummary;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SidebarProfileItem {
    pub profile_id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SidebarSessionItem {
    pub session_id: String,
    pub profile_id: String,
    pub name: String,
    pub status: String,
    pub is_active: bool,
    pub pane_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TerminalWorkspaceViewModel {
    pub profiles: Vec<SidebarProfileItem>,
    pub sessions: Vec<SidebarSessionItem>,
    pub active_profile_id: Option<String>,
    pub active_session_id: Option<String>,
    pub active_pane_id: Option<String>,
    pub status_line: String,
}

pub fn build_terminal_workspace_view_model(
    workspace: &WorkspaceState,
    pane_snapshots: &[PaneSnapshotSummary],
) -> TerminalWorkspaceViewModel {
    let pane_by_session: HashMap<String, String> = pane_snapshots
        .iter()
        .map(|pane| (pane.session_id.clone(), pane.pane_id.clone()))
        .collect();

    let profiles = workspace
        .profiles
        .iter()
        .map(|profile| SidebarProfileItem {
            profile_id: profile.profile_id.clone(),
            name: profile.name.clone(),
            is_active: workspace.active_profile_id.as_deref() == Some(profile.profile_id.as_str()),
        })
        .collect::<Vec<_>>();

    let sessions = workspace
        .sessions
        .iter()
        .map(|session| {
            let pane_id = pane_by_session.get(&session.session_id).cloned();
            SidebarSessionItem {
                session_id: session.session_id.clone(),
                profile_id: session.profile_id.clone(),
                name: session.name.clone(),
                status: format!("{:?}", session.status).to_lowercase(),
                is_active: workspace.active_session_id.as_deref() == Some(session.session_id.as_str()),
                pane_id,
            }
        })
        .collect::<Vec<_>>();

    let active_pane_id = workspace
        .active_session_id
        .as_deref()
        .and_then(|session_id| pane_by_session.get(session_id).cloned());

    let status_line = format!(
        "profiles={} sessions={} panes={} active_profile={} active_session={}",
        workspace.profiles.len(),
        workspace.sessions.len(),
        pane_snapshots.len(),
        workspace
            .active_profile_id
            .as_deref()
            .unwrap_or("none"),
        workspace
            .active_session_id
            .as_deref()
            .unwrap_or("none")
    );

    TerminalWorkspaceViewModel {
        profiles,
        sessions,
        active_profile_id: workspace.active_profile_id.clone(),
        active_session_id: workspace.active_session_id.clone(),
        active_pane_id,
        status_line,
    }
}

#[cfg(test)]
#[path = "terminal_workspace_view_model_tests.rs"]
mod terminal_workspace_view_model_tests;
