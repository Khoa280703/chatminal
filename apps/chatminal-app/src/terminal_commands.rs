use std::time::Duration;

use chatminal_protocol::{Request, Response};
use serde_json::{Value, json};

use crate::config::parse_usize;
use crate::ipc::ChatminalClient;
use crate::session_terminal_adapter::{SessionTerminalRegistry, pump_events_with_registry};
use crate::session_terminal_emulator::SessionTerminalEmulator;
use crate::terminal_session_commands::activate_session_with_snapshot;
use crate::terminal_workspace_ascii_renderer::render_terminal_workspace_ascii;
use crate::terminal_workspace_binding_runtime::bootstrap_workspace_binding_state;
use crate::terminal_workspace_view_model::build_terminal_workspace_view_model;

pub fn handle_activate_terminal(
    client: &ChatminalClient,
    args: &[String],
    terminal_registry: &mut SessionTerminalRegistry,
) -> Result<Value, String> {
    let session_id = args
        .get(2)
        .cloned()
        .ok_or_else(|| "missing session id".to_string())?;
    let cols = parse_usize(args.get(3), 120);
    let rows = parse_usize(args.get(4), 32);
    let preview_lines = parse_usize(args.get(5), 200);
    let mut adapter = SessionTerminalEmulator::new(cols, rows, 5_000);
    let activation = activate_session_with_snapshot(
        client,
        terminal_registry,
        &mut adapter,
        &session_id,
        cols,
        rows,
        preview_lines,
    )?;
    let terminal_snapshot = adapter.terminal_snapshot(&activation.terminal_id);
    Ok(json!({
        "activation": activation,
        "terminal_snapshot": terminal_snapshot
    }))
}

pub fn handle_events_terminal(
    client: &ChatminalClient,
    args: &[String],
    terminal_registry: &mut SessionTerminalRegistry,
) -> Result<Value, String> {
    let seconds = parse_usize(args.get(2), 10);
    let workspace_response = client.request(Request::WorkspaceLoad, Duration::from_secs(4))?;
    match workspace_response {
        Response::Workspace(_) => {}
        other => {
            return Err(format!(
                "unexpected response for workspace_load in events-terminal: {:?}",
                other
            ));
        }
    }
    let mut adapter = SessionTerminalEmulator::new(120, 32, 5_000);
    let processed = pump_events_with_registry(
        client,
        terminal_registry,
        &mut adapter,
        Duration::from_secs(seconds as u64),
    )?;
    Ok(json!({
        "processed_events": processed,
        "terminal_snapshots": adapter.all_terminal_snapshots()
    }))
}

pub fn handle_workspace_terminal(
    client: &ChatminalClient,
    args: &[String],
    terminal_registry: &mut SessionTerminalRegistry,
) -> Result<Value, String> {
    let preview_lines = parse_usize(args.get(2), 200);
    let cols = parse_usize(args.get(3), 120);
    let rows = parse_usize(args.get(4), 32);
    let state =
        bootstrap_workspace_binding_state(client, terminal_registry, preview_lines, cols, rows)?;
    let terminal_snapshots = state.terminal_snapshots();
    let view_model = build_terminal_workspace_view_model(&state.workspace, &terminal_snapshots);
    Ok(json!({
        "workspace": state.workspace,
        "terminal_snapshots": terminal_snapshots,
        "view_model": view_model,
        "hydrate_errors": state.hydrate_errors
    }))
}

pub fn handle_dashboard_terminal(
    client: &ChatminalClient,
    args: &[String],
    terminal_registry: &mut SessionTerminalRegistry,
) -> Result<Value, String> {
    let preview_lines = parse_usize(args.get(2), 200);
    let cols = parse_usize(args.get(3), 120);
    let rows = parse_usize(args.get(4), 32);
    let max_terminal_preview_lines = parse_usize(args.get(5), 20);
    let state =
        bootstrap_workspace_binding_state(client, terminal_registry, preview_lines, cols, rows)?;
    let terminal_snapshots = state.terminal_snapshots();
    let view_model = build_terminal_workspace_view_model(&state.workspace, &terminal_snapshots);
    let dashboard =
        render_terminal_workspace_ascii(&view_model, &terminal_snapshots, max_terminal_preview_lines);
    Ok(json!({
        "dashboard": dashboard,
        "workspace": state.workspace,
        "view_model": view_model,
        "hydrate_errors": state.hydrate_errors
    }))
}
