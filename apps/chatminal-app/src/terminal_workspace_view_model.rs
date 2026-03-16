use std::collections::HashMap;

use chatminal_protocol::WorkspaceState;
use serde::Serialize;

use crate::session_terminal_emulator::TerminalSnapshotSummary;

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
    pub terminal_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TerminalWorkspaceViewModel {
    pub profiles: Vec<SidebarProfileItem>,
    pub sessions: Vec<SidebarSessionItem>,
    pub active_profile_id: Option<String>,
    pub active_session_id: Option<String>,
    pub active_terminal_id: Option<String>,
    pub status_line: String,
}

pub fn build_terminal_workspace_view_model(
    workspace: &WorkspaceState,
    terminal_snapshots: &[TerminalSnapshotSummary],
) -> TerminalWorkspaceViewModel {
    let terminal_by_session: HashMap<String, String> = terminal_snapshots
        .iter()
        .map(|terminal| (terminal.session_id.clone(), terminal.terminal_id.clone()))
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
            let terminal_id = terminal_by_session.get(&session.session_id).cloned();
            SidebarSessionItem {
                session_id: session.session_id.clone(),
                profile_id: session.profile_id.clone(),
                name: session.name.clone(),
                status: format!("{:?}", session.status).to_lowercase(),
                is_active: workspace.active_session_id.as_deref()
                    == Some(session.session_id.as_str()),
                terminal_id,
            }
        })
        .collect::<Vec<_>>();

    let active_terminal_id = workspace
        .active_session_id
        .as_deref()
        .and_then(|session_id| terminal_by_session.get(session_id).cloned());

    let status_line = format!(
        "profiles={} sessions={} terminals={} active_profile={} active_session={}",
        workspace.profiles.len(),
        workspace.sessions.len(),
        terminal_snapshots.len(),
        workspace.active_profile_id.as_deref().unwrap_or("none"),
        workspace.active_session_id.as_deref().unwrap_or("none")
    );

    TerminalWorkspaceViewModel {
        profiles,
        sessions,
        active_profile_id: workspace.active_profile_id.clone(),
        active_session_id: workspace.active_session_id.clone(),
        active_terminal_id,
        status_line,
    }
}

#[cfg(test)]
#[path = "terminal_workspace_view_model_tests.rs"]
mod terminal_workspace_view_model_tests;
