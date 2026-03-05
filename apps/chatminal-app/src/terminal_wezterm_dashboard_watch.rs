use std::io::{Write, stdout};
use std::time::{Duration, Instant};

use crossterm::terminal::size;

use crate::config::parse_usize;
use crate::ipc::ChatminalClient;
use crate::terminal_pane_adapter::SessionPaneRegistry;
use crate::terminal_workspace_ascii_renderer::{
    fit_dashboard_for_terminal, render_terminal_workspace_ascii,
};
use crate::terminal_workspace_binding_runtime::{
    WorkspaceBindingState, apply_event_to_workspace_binding_state,
    bootstrap_workspace_binding_state,
};
use crate::terminal_workspace_view_model::build_terminal_workspace_view_model;

pub fn run_dashboard_watch_wezterm(
    client: &ChatminalClient,
    args: &[String],
    pane_registry: &mut SessionPaneRegistry,
) -> Result<(), String> {
    let seconds = parse_usize(args.get(2), 30).max(1);
    let refresh_ms = parse_usize(args.get(3), 500).max(100);
    let preview_lines = parse_usize(args.get(4), 200);
    let cols = parse_usize(args.get(5), 120);
    let rows = parse_usize(args.get(6), 32);
    let max_pane_preview_lines = parse_usize(args.get(7), 20);

    let mut state =
        bootstrap_workspace_binding_state(client, pane_registry, preview_lines, cols, rows)?;
    let deadline = Instant::now() + Duration::from_secs(seconds as u64);
    let tick = Duration::from_millis(refresh_ms as u64);
    let mut next_render = Instant::now();
    let mut out = stdout();

    while Instant::now() < deadline {
        if state.is_stale() {
            state = bootstrap_workspace_binding_state(
                client,
                pane_registry,
                preview_lines,
                cols,
                rows,
            )?;
        }

        let now = Instant::now();
        if now >= next_render {
            let dashboard = render_watch_dashboard(&state, max_pane_preview_lines);
            let (terminal_cols, terminal_rows) = size().unwrap_or((120, 32));
            let fitted = fit_dashboard_for_terminal(
                &dashboard,
                terminal_cols as usize,
                terminal_rows as usize,
            );
            let fitted = normalize_newlines_for_raw_mode(&fitted);
            write!(out, "\x1b[2J\x1b[H{fitted}")
                .map_err(|err| format!("write dashboard failed: {err}"))?;
            out.flush()
                .map_err(|err| format!("flush dashboard failed: {err}"))?;
            next_render = now + tick;
        }

        let now = Instant::now();
        let wait_to_next = next_render.saturating_duration_since(now);
        let wait = wait_to_next.min(Duration::from_millis(100));
        if let Some(event) = client.recv_event(wait)? {
            apply_event_to_workspace_binding_state(&mut state, pane_registry, event);
        }
    }

    Ok(())
}

fn render_watch_dashboard(state: &WorkspaceBindingState, max_pane_preview_lines: usize) -> String {
    let pane_snapshots = state.adapter.all_pane_snapshots();
    let view_model = build_terminal_workspace_view_model(&state.workspace, &pane_snapshots);
    let mut dashboard =
        render_terminal_workspace_ascii(&view_model, &pane_snapshots, max_pane_preview_lines);
    if !state.hydrate_errors.is_empty() {
        dashboard.push_str("\n\nWarnings:\n");
        for error in &state.hydrate_errors {
            dashboard.push_str("- ");
            dashboard.push_str(error);
            dashboard.push('\n');
        }
    }
    dashboard
}

fn normalize_newlines_for_raw_mode(input: &str) -> String {
    input.replace('\n', "\r\n")
}
