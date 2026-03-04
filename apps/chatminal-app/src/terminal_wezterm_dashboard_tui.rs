use std::io::{Write, stdout};
use std::time::{Duration, Instant};

use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode};
use crossterm::style::Print;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{execute, queue};

use crate::config::parse_usize;
use crate::ipc::ChatminalClient;
use crate::terminal_pane_adapter::SessionPaneRegistry;
use crate::terminal_workspace_ascii_renderer::render_terminal_workspace_ascii;
use crate::terminal_workspace_binding_runtime::{
    apply_event_to_workspace_binding_state, bootstrap_workspace_binding_state,
};
use crate::terminal_workspace_view_model::build_terminal_workspace_view_model;

struct TuiGuard;

impl TuiGuard {
    fn enter() -> Result<Self, String> {
        enable_raw_mode().map_err(|err| format!("enable raw mode failed: {err}"))?;
        if let Err(err) = execute!(stdout(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(format!("enter alternate screen failed: {err}"));
        }
        Ok(Self)
    }
}

impl Drop for TuiGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

pub fn run_dashboard_tui_wezterm(
    client: &ChatminalClient,
    args: &[String],
    pane_registry: &mut SessionPaneRegistry,
) -> Result<(), String> {
    let refresh_ms = parse_usize(args.get(2), 120).clamp(40, 2_000);
    let preview_lines = parse_usize(args.get(3), 200).clamp(20, 5_000);
    let cols = parse_usize(args.get(4), 120).clamp(20, 400);
    let rows = parse_usize(args.get(5), 32).clamp(5, 200);
    let max_pane_preview_lines = parse_usize(args.get(6), 20).clamp(5, 200);

    let _guard = TuiGuard::enter()?;
    let mut out = stdout();
    let mut state =
        bootstrap_workspace_binding_state(client, pane_registry, preview_lines, cols, rows)?;
    let mut last_render = Instant::now()
        .checked_sub(Duration::from_millis(refresh_ms as u64))
        .unwrap_or_else(Instant::now);

    loop {
        if state.is_stale() {
            state = bootstrap_workspace_binding_state(client, pane_registry, preview_lines, cols, rows)?;
        }

        let now = Instant::now();
        if now.duration_since(last_render) >= Duration::from_millis(refresh_ms as u64) {
            render_dashboard_frame(&mut out, &state, max_pane_preview_lines)?;
            last_render = now;
        }

        if event::poll(Duration::from_millis(10))
            .map_err(|err| format!("poll input failed: {err}"))?
            && let CrosstermEvent::Key(key) =
                event::read().map_err(|err| format!("read input failed: {err}"))?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    state = bootstrap_workspace_binding_state(
                        client,
                        pane_registry,
                        preview_lines,
                        cols,
                        rows,
                    )?;
                }
                _ => {}
            }
        }

        if let Some(event) = client.recv_event(Duration::from_millis(25))? {
            apply_event_to_workspace_binding_state(&mut state, pane_registry, event);
        }
    }

    Ok(())
}

fn render_dashboard_frame(
    out: &mut std::io::Stdout,
    state: &crate::terminal_workspace_binding_runtime::WorkspaceBindingState,
    max_pane_preview_lines: usize,
) -> Result<(), String> {
    let pane_snapshots = state.pane_snapshots();
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

    dashboard.push_str("\n\nKeys: q/esc = quit | r = reload snapshot");

    queue!(
        out,
        MoveTo(0, 0),
        Clear(ClearType::All),
        Print(dashboard),
        Print("\n")
    )
    .map_err(|err| format!("render dashboard failed: {err}"))?;
    out.flush()
        .map_err(|err| format!("flush dashboard failed: {err}"))?;
    Ok(())
}
