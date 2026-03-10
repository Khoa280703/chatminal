use std::collections::HashMap;
use std::time::Duration;

use chatminal_protocol::{Event, Request, Response, SessionStatus, WorkspaceState};

use crate::ipc::ChatminalClient;
use crate::terminal_pane_adapter::{
    SessionPaneRegistry, TerminalPaneAdapter, dispatch_event_with_registry,
};
use crate::terminal_pane_emulator::{PaneSnapshotSummary, TerminalPaneEmulator};
use crate::terminal_session_commands::fetch_snapshot_for_session;

pub struct WorkspaceBindingState {
    pub workspace: WorkspaceState,
    pub adapter: TerminalPaneEmulator,
    pub hydrate_errors: Vec<String>,
    stale: bool,
    event_watermark_ts: u64,
    last_workspace_event_ts: u64,
    session_last_event_ts: HashMap<String, u64>,
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
    // Watermark để chặn backlog event cũ trước lần bootstrap hiện tại.
    let bootstrap_started_at = now_millis();
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

    let mut adapter = TerminalPaneEmulator::new(cols, rows, 5_000);
    let mut hydrate_errors = Vec::new();
    let active_session_id = workspace.active_session_id.as_deref();
    let mut session_last_event_ts = HashMap::new();

    for session in &workspace.sessions {
        session_last_event_ts.insert(session.session_id.clone(), bootstrap_started_at);
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
        event_watermark_ts: bootstrap_started_at,
        last_workspace_event_ts: bootstrap_started_at,
        session_last_event_ts,
    })
}

pub fn apply_event_to_workspace_binding_state(
    state: &mut WorkspaceBindingState,
    pane_registry: &mut SessionPaneRegistry,
    event: Event,
) {
    if let Some(value) = event_ts_for_ordering(&event) {
        if value < state.event_watermark_ts {
            return;
        }
    }

    match event {
        Event::PtyOutput(value) => {
            let previous_ts = state
                .session_last_event_ts
                .get(&value.session_id)
                .copied()
                .unwrap_or(state.event_watermark_ts);
            if value.ts < previous_ts {
                return;
            }
            dispatch_event_with_registry(
                &mut state.adapter,
                pane_registry,
                Event::PtyOutput(value.clone()),
            );
            if let Some(session) = state
                .workspace
                .sessions
                .iter_mut()
                .find(|session| session.session_id == value.session_id)
            {
                session.seq = session.seq.max(value.seq);
                session.status = SessionStatus::Running;
                update_session_event_ts(
                    &mut state.session_last_event_ts,
                    &value.session_id,
                    value.ts,
                );
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
                dispatch_event_with_registry(
                    &mut state.adapter,
                    pane_registry,
                    Event::PtyExited(value.clone()),
                );
                session.status = SessionStatus::Disconnected;
            }
        }
        Event::SessionUpdated(value) => {
            let previous_ts = state
                .session_last_event_ts
                .get(&value.session_id)
                .copied()
                .unwrap_or(state.event_watermark_ts);
            if value.ts < previous_ts {
                return;
            }
            dispatch_event_with_registry(
                &mut state.adapter,
                pane_registry,
                Event::SessionUpdated(value.clone()),
            );
            if let Some(session) = state
                .workspace
                .sessions
                .iter_mut()
                .find(|session| session.session_id == value.session_id)
            {
                session.status = value.status;
                session.seq = session.seq.max(value.seq);
                session.persist_history = value.persist_history;
                state
                    .session_last_event_ts
                    .insert(value.session_id.clone(), value.ts);
            } else {
                state.stale = true;
            }
        }
        Event::WorkspaceUpdated(value) => {
            if value.ts < state.last_workspace_event_ts {
                return;
            }
            dispatch_event_with_registry(
                &mut state.adapter,
                pane_registry,
                Event::WorkspaceUpdated(value.clone()),
            );
            state.last_workspace_event_ts = value.ts;
            state.workspace.active_profile_id = value.active_profile_id;
            state.workspace.active_session_id = value.active_session_id;
            // WorkspaceUpdated không mang full payload session/profile.
            // Mark stale luôn để bootstrap lại snapshot đầy đủ ở tick kế tiếp.
            let _ = value.profile_count;
            let _ = value.session_count;
            state.stale = true;
        }
        Event::PtyError(value) => {
            dispatch_event_with_registry(&mut state.adapter, pane_registry, Event::PtyError(value));
        }
        Event::DaemonHealth(value) => {
            dispatch_event_with_registry(
                &mut state.adapter,
                pane_registry,
                Event::DaemonHealth(value),
            );
        }
    }
}

fn expect_workspace(response: Response, op: &str) -> Result<WorkspaceState, String> {
    match response {
        Response::Workspace(value) => Ok(value),
        other => Err(format!("unexpected response for {op}: {:?}", other)),
    }
}

fn update_session_event_ts(store: &mut HashMap<String, u64>, session_id: &str, ts: u64) {
    let current = store.get(session_id).copied().unwrap_or(0);
    if ts > current {
        store.insert(session_id.to_string(), ts);
    }
}

fn event_ts_for_ordering(event: &Event) -> Option<u64> {
    match event {
        Event::PtyOutput(value) => Some(value.ts),
        Event::SessionUpdated(value) => Some(value.ts),
        Event::WorkspaceUpdated(value) => Some(value.ts),
        Event::PtyExited(_) | Event::PtyError(_) | Event::DaemonHealth(_) => None,
    }
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "terminal_workspace_binding_runtime_tests.rs"]
mod terminal_workspace_binding_runtime_tests;
