use std::time::Duration;

use chatminal_protocol::{Event, Request, Response, SessionStatus, WorkspaceState};

use crate::ipc::ChatminalClient;
use crate::terminal_pane_adapter::{
    SessionPaneRegistry, TerminalPaneAdapter, dispatch_event_with_registry,
};
use crate::terminal_session_commands::fetch_snapshot_for_session;
use crate::terminal_wezterm_core::{PaneSnapshotSummary, WeztermTerminalPaneAdapter};

pub struct WorkspaceBindingState {
    pub workspace: WorkspaceState,
    pub adapter: WeztermTerminalPaneAdapter,
    pub hydrate_errors: Vec<String>,
    stale: bool,
}

impl WorkspaceBindingState {
    pub fn pane_snapshots(&self) -> Vec<PaneSnapshotSummary> {
        self.adapter.all_pane_snapshots()
    }

    pub fn is_stale(&self) -> bool {
        self.stale
    }
}

pub fn bootstrap_workspace_binding_state(
    client: &ChatminalClient,
    pane_registry: &mut SessionPaneRegistry,
    preview_lines: usize,
    cols: usize,
    rows: usize,
) -> Result<WorkspaceBindingState, String> {
    let workspace = expect_workspace(
        client.request(Request::WorkspaceLoad, Duration::from_secs(4))?,
        "workspace_load",
    )?;
    let session_ids = workspace
        .sessions
        .iter()
        .map(|value| value.session_id.clone())
        .collect::<Vec<_>>();
    pane_registry.prune_to_sessions(&session_ids);

    let mut adapter = WeztermTerminalPaneAdapter::new(cols, rows, 5_000);
    let mut hydrate_errors = Vec::new();
    let active_session_id = workspace.active_session_id.as_deref();

    for session in &workspace.sessions {
        let pane_id = if active_session_id == Some(session.session_id.as_str()) {
            pane_registry.activate_session(&session.session_id)
        } else {
            pane_registry.ensure_pane_for_session(&session.session_id)
        };
        adapter.on_session_activated(&session.session_id, &pane_id, cols, rows);

        match fetch_snapshot_for_session(client, &session.session_id, preview_lines) {
            Ok(snapshot) => adapter.on_session_snapshot(&session.session_id, &pane_id, &snapshot),
            Err(err) => hydrate_errors.push(format!(
                "session '{}' snapshot hydrate failed: {err}",
                session.session_id
            )),
        }
    }

    Ok(WorkspaceBindingState {
        workspace,
        adapter,
        hydrate_errors,
        stale: false,
    })
}

pub fn apply_event_to_workspace_binding_state(
    state: &mut WorkspaceBindingState,
    pane_registry: &mut SessionPaneRegistry,
    event: Event,
) {
    dispatch_event_with_registry(&mut state.adapter, pane_registry, event.clone());
    match event {
        Event::PtyOutput(value) => {
            if let Some(session) = state
                .workspace
                .sessions
                .iter_mut()
                .find(|session| session.session_id == value.session_id)
            {
                session.seq = session.seq.max(value.seq);
                session.status = SessionStatus::Running;
            } else {
                state.stale = true;
            }
        }
        Event::PtyExited(value) => {
            if let Some(session) = state
                .workspace
                .sessions
                .iter_mut()
                .find(|session| session.session_id == value.session_id)
            {
                session.status = SessionStatus::Disconnected;
            } else {
                state.stale = true;
            }
        }
        Event::SessionUpdated(value) => {
            if let Some(session) = state
                .workspace
                .sessions
                .iter_mut()
                .find(|session| session.session_id == value.session_id)
            {
                session.status = value.status;
                session.seq = value.seq;
                session.persist_history = value.persist_history;
            } else {
                state.stale = true;
            }
        }
        Event::WorkspaceUpdated(value) => {
            state.workspace.active_profile_id = value.active_profile_id;
            state.workspace.active_session_id = value.active_session_id;
            // WorkspaceUpdated không mang full payload session/profile.
            // Mark stale luôn để bootstrap lại snapshot đầy đủ ở tick kế tiếp.
            let _ = value.profile_count;
            let _ = value.session_count;
            state.stale = true;
        }
        Event::PtyError(_) | Event::DaemonHealth(_) => {}
    }
}

fn expect_workspace(response: Response, op: &str) -> Result<WorkspaceState, String> {
    match response {
        Response::Workspace(value) => Ok(value),
        other => Err(format!("unexpected response for {op}: {:?}", other)),
    }
}

#[cfg(test)]
#[path = "terminal_workspace_binding_runtime_tests.rs"]
mod terminal_workspace_binding_runtime_tests;
