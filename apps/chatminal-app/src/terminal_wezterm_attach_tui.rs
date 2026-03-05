use std::io::stdout;
use std::time::{Duration, Instant};

use chatminal_protocol::{Request, Response};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
};

use crate::config::InputPipelineMode;
use crate::config::parse_usize;
use crate::input::{
    is_attach_exit_key, map_key_event_to_pty_input, map_key_event_to_pty_input_legacy,
};
use crate::ipc::ChatminalClient;
use crate::terminal_pane_adapter::{SessionPaneRegistry, dispatch_event_with_registry};
use crate::terminal_session_commands::{
    activate_session_with_snapshot, resize_session, write_input_for_session,
};
use crate::terminal_wezterm_attach_frame_renderer::render_attach_frame;
use crate::terminal_wezterm_core::WeztermTerminalPaneAdapter;

const SCROLLBACK_SIZE: usize = 5_000;

struct AttachTuiGuard;

impl AttachTuiGuard {
    fn enter() -> Result<Self, String> {
        enable_raw_mode().map_err(|err| format!("enable raw mode failed: {err}"))?;
        if let Err(err) = execute!(stdout(), EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(format!("enter alternate screen failed: {err}"));
        }
        Ok(Self)
    }
}

impl Drop for AttachTuiGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), Show, LeaveAlternateScreen);
    }
}

pub fn run_attach_tui_wezterm(
    client: &ChatminalClient,
    args: &[String],
    pane_registry: &mut SessionPaneRegistry,
    input_pipeline_mode: InputPipelineMode,
) -> Result<(), String> {
    let (term_cols, term_rows) = size().unwrap_or((120, 32));
    let mut cursor = 2usize;
    let session_id_arg = args.get(cursor).and_then(|value| {
        if value.parse::<usize>().is_ok() {
            None
        } else {
            Some(value.clone())
        }
    });
    if session_id_arg.is_some() {
        cursor = cursor.saturating_add(1);
    }

    let cols = parse_usize(args.get(cursor), term_cols as usize).max(20);
    let rows = parse_usize(
        args.get(cursor.saturating_add(1)),
        (term_rows as usize).saturating_sub(1).max(5),
    )
    .max(5);
    let preview_lines = parse_usize(args.get(cursor.saturating_add(2)), 400).clamp(50, 10_000);

    let session_id = resolve_attach_session_id(client, session_id_arg)?;
    let mut adapter = WeztermTerminalPaneAdapter::new(cols, rows, SCROLLBACK_SIZE);
    let activation = activate_session_with_snapshot(
        client,
        pane_registry,
        &mut adapter,
        &session_id,
        cols,
        rows,
        preview_lines,
    )?;

    let _guard = AttachTuiGuard::enter()?;
    let mut out = stdout();
    let mut current_cols = cols;
    let mut current_rows = rows;
    let mut last_render = Instant::now()
        .checked_sub(Duration::from_millis(120))
        .unwrap_or_else(Instant::now);
    let mut dirty = true;

    loop {
        let (raw_cols, raw_rows) =
            size().unwrap_or((current_cols as u16, (current_rows + 1) as u16));
        let next_cols = (raw_cols as usize).max(20);
        let next_rows = (raw_rows as usize).saturating_sub(1).max(5);
        if next_cols != current_cols || next_rows != current_rows {
            resize_session(
                client,
                pane_registry,
                &mut adapter,
                &session_id,
                next_cols,
                next_rows,
            )?;
            current_cols = next_cols;
            current_rows = next_rows;
            dirty = true;
        }

        if dirty || last_render.elapsed() >= Duration::from_millis(100) {
            render_attach_frame(
                &mut out,
                &adapter,
                &activation.pane_id,
                &session_id,
                current_cols,
                current_rows,
            )?;
            dirty = false;
            last_render = Instant::now();
        }

        if event::poll(Duration::from_millis(12))
            .map_err(|err| format!("poll input failed: {err}"))?
        {
            match event::read().map_err(|err| format!("read input failed: {err}"))? {
                CrosstermEvent::Key(key) => {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    if is_attach_exit_key(key) {
                        break;
                    }
                    let mapped = if input_pipeline_mode == InputPipelineMode::Legacy {
                        map_key_event_to_pty_input_legacy(key)
                    } else {
                        map_key_event_to_pty_input(key)
                    };
                    if let Some(data) = mapped {
                        write_input_for_session(
                            client,
                            pane_registry,
                            &mut adapter,
                            &session_id,
                            &data,
                        )?;
                    }
                }
                CrosstermEvent::Paste(data) => {
                    if !data.is_empty() {
                        write_input_for_session(
                            client,
                            pane_registry,
                            &mut adapter,
                            &session_id,
                            &data,
                        )?;
                    }
                }
                CrosstermEvent::Resize(_, _) => {
                    dirty = true;
                }
                _ => {}
            }
        }

        if let Some(event) = client.recv_event(Duration::from_millis(15))? {
            dispatch_event_with_registry(&mut adapter, pane_registry, event);
            dirty = true;
        }
    }

    Ok(())
}

fn resolve_attach_session_id(
    client: &ChatminalClient,
    preferred: Option<String>,
) -> Result<String, String> {
    if let Some(session_id) = preferred {
        return Ok(session_id);
    }

    let response = client.request(Request::WorkspaceLoad, Duration::from_secs(4))?;
    let workspace = match response {
        Response::Workspace(value) => value,
        other => {
            return Err(format!(
                "unexpected response for workspace_load: {:?}",
                other
            ));
        }
    };
    if let Some(active) = workspace.active_session_id {
        return Ok(active);
    }
    workspace
        .sessions
        .first()
        .map(|value| value.session_id.clone())
        .ok_or_else(|| "no session available to attach".to_string())
}
