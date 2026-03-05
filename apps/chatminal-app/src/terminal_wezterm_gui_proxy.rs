use std::io::{IsTerminal, Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chatminal_protocol::{Request, Response, WorkspaceState};
use crossterm::terminal;

use crate::config::parse_usize;
use crate::ipc::ChatminalClient;
use crate::window_wezterm_gui::chatminal_ipc_mux_domain::{
    ChatminalIpcMuxDomain, DomainEventAction, clamp_preview_lines,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(4);
const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(10);
const RESIZE_POLL_INTERVAL: Duration = Duration::from_millis(120);
const INPUT_DRAIN_BUDGET: usize = 32;
const EVENT_DRAIN_BUDGET: usize = 128;
const EXIT_DRAIN_POLL_TIMEOUT: Duration = Duration::from_millis(5);
const EXIT_DRAIN_MAX_DURATION: Duration = Duration::from_millis(80);

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self, String> {
        terminal::enable_raw_mode().map_err(|err| format!("enable raw mode failed: {err}"))?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

pub fn run_proxy_wezterm_session(client: &ChatminalClient, args: &[String]) -> Result<(), String> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(
            "proxy-wezterm-session requires an interactive TTY (run via wezterm start ...)"
                .to_string(),
        );
    }

    let session_id = resolve_target_session_id(client, args.get(2).map(String::as_str))?;
    let preview_lines = clamp_preview_lines(parse_usize(args.get(3), 2_000));

    let (mut cols, mut rows) = read_terminal_size();
    activate_session(client, &session_id, cols, rows)?;

    let snapshot = fetch_snapshot(client, &session_id, preview_lines)?;
    if !snapshot.content.is_empty() {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(snapshot.content.as_bytes())
            .map_err(|err| format!("write snapshot failed: {err}"))?;
        stdout
            .flush()
            .map_err(|err| format!("flush snapshot failed: {err}"))?;
    }
    let mut mux_domain = ChatminalIpcMuxDomain::new(session_id.clone(), snapshot.seq);

    let _raw_mode = RawModeGuard::new()?;
    let (input_tx, input_rx) = mpsc::sync_channel::<Vec<u8>>(1024);
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        let mut buf = [0u8; 8192];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(read) => {
                    if input_tx.send(buf[..read].to_vec()).is_err() {
                        break;
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });

    let mut last_resize_poll = Instant::now();
    let mut stdout = std::io::stdout().lock();
    loop {
        for _ in 0..INPUT_DRAIN_BUDGET {
            let Ok(payload) = input_rx.try_recv() else {
                break;
            };
            mux_domain.queue_input_payload(&payload);
            if mux_domain.should_flush_input_batch() {
                if flush_input_batch(client, &session_id, &mut mux_domain)? {
                    return Ok(());
                }
            }
        }
        if process_session_events(client, &mut mux_domain, &mut stdout)? {
            return Ok(());
        }
        if flush_input_batch(client, &session_id, &mut mux_domain)? {
            return Ok(());
        }

        if last_resize_poll.elapsed() >= RESIZE_POLL_INTERVAL {
            let (next_cols, next_rows) = read_terminal_size();
            if next_cols != cols || next_rows != rows {
                cols = next_cols;
                rows = next_rows;
                let response = client.request(
                    Request::SessionResize {
                        session_id: session_id.clone(),
                        cols,
                        rows,
                    },
                    REQUEST_TIMEOUT,
                )?;
                expect_empty(response, "session_resize")?;
            }
            last_resize_poll = Instant::now();
        }
    }
}

fn resolve_target_session_id(
    client: &ChatminalClient,
    explicit: Option<&str>,
) -> Result<String, String> {
    if let Some(value) = explicit {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let workspace = client.request(Request::WorkspaceLoad, REQUEST_TIMEOUT)?;
    let workspace = expect_workspace(workspace, "workspace_load")?;
    workspace
        .active_session_id
        .or_else(|| {
            workspace
                .sessions
                .first()
                .map(|value| value.session_id.clone())
        })
        .map(Ok)
        .unwrap_or_else(|| create_default_session(client))
}

fn create_default_session(client: &ChatminalClient) -> Result<String, String> {
    let response = client.request(
        Request::SessionCreate {
            name: Some("Shell".to_string()),
            cols: 120,
            rows: 32,
            cwd: None,
            persist_history: Some(false),
        },
        REQUEST_TIMEOUT,
    )?;
    match response {
        Response::SessionCreate(value) => Ok(value.session_id),
        other => Err(format!(
            "unexpected response for session_create (auto bootstrap): {:?}",
            other
        )),
    }
}

fn flush_input_batch(
    client: &ChatminalClient,
    session_id: &str,
    mux_domain: &mut ChatminalIpcMuxDomain,
) -> Result<bool, String> {
    let Some(data) = mux_domain.take_input_batch() else {
        return Ok(false);
    };
    let response = match client.request(
        Request::SessionInputWrite {
            session_id: session_id.to_string(),
            data,
        },
        REQUEST_TIMEOUT,
    ) {
        Ok(value) => value,
        Err(err) => {
            if is_graceful_detach_error(&err) {
                return Ok(true);
            }
            return Err(err);
        }
    };
    expect_empty(response, "session_input_write")?;
    Ok(false)
}

fn is_graceful_detach_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("session is not running") || normalized.contains("session not found")
}

fn activate_session(
    client: &ChatminalClient,
    session_id: &str,
    cols: usize,
    rows: usize,
) -> Result<(), String> {
    let response = client.request(
        Request::SessionActivate {
            session_id: session_id.to_string(),
            cols,
            rows,
        },
        REQUEST_TIMEOUT,
    )?;
    expect_empty(response, "session_activate")
}

fn fetch_snapshot(
    client: &ChatminalClient,
    session_id: &str,
    preview_lines: usize,
) -> Result<chatminal_protocol::SessionSnapshot, String> {
    let response = client.request(
        Request::SessionSnapshotGet {
            session_id: session_id.to_string(),
            preview_lines: Some(preview_lines),
        },
        REQUEST_TIMEOUT,
    )?;
    match response {
        Response::SessionSnapshot(value) => Ok(value),
        other => Err(format!(
            "unexpected response for session_snapshot_get: {:?}",
            other
        )),
    }
}

fn expect_workspace(response: Response, op: &str) -> Result<WorkspaceState, String> {
    match response {
        Response::Workspace(value) => Ok(value),
        other => Err(format!("unexpected response for {op}: {:?}", other)),
    }
}

fn expect_empty(response: Response, op: &str) -> Result<(), String> {
    match response {
        Response::Empty => Ok(()),
        other => Err(format!("unexpected response for {op}: {:?}", other)),
    }
}

fn read_terminal_size() -> (usize, usize) {
    let (cols, rows) = terminal::size().unwrap_or((120, 32));
    (cols.max(20) as usize, rows.max(5) as usize)
}

fn handle_session_event(
    mux_domain: &mut ChatminalIpcMuxDomain,
    event: chatminal_protocol::Event,
    stdout: &mut impl Write,
) -> Result<bool, String> {
    match mux_domain.consume_event(event) {
        DomainEventAction::Output(chunk) => {
            stdout
                .write_all(chunk.as_bytes())
                .map_err(|err| format!("write output failed: {err}"))?;
            stdout
                .flush()
                .map_err(|err| format!("flush output failed: {err}"))?;
            Ok(false)
        }
        DomainEventAction::Error(message) => {
            let mut stderr = std::io::stderr().lock();
            let _ = writeln!(stderr, "chatminal proxy error: {message}");
            let _ = stderr.flush();
            Ok(false)
        }
        DomainEventAction::ExitRequested => Ok(true),
        DomainEventAction::Ignore => Ok(false),
    }
}

fn process_session_events(
    client: &ChatminalClient,
    mux_domain: &mut ChatminalIpcMuxDomain,
    stdout: &mut impl Write,
) -> Result<bool, String> {
    if let Some(event) = client.recv_event(EVENT_POLL_TIMEOUT)? {
        if handle_session_event(mux_domain, event, stdout)? {
            drain_tail_output_after_exit(client, mux_domain, stdout)?;
            return Ok(true);
        }
    }
    for _ in 0..EVENT_DRAIN_BUDGET {
        let Some(event) = client.recv_event(Duration::from_millis(0))? else {
            break;
        };
        if handle_session_event(mux_domain, event, stdout)? {
            drain_tail_output_after_exit(client, mux_domain, stdout)?;
            return Ok(true);
        }
    }
    Ok(false)
}

fn drain_tail_output_after_exit(
    client: &ChatminalClient,
    mux_domain: &mut ChatminalIpcMuxDomain,
    stdout: &mut impl Write,
) -> Result<(), String> {
    let deadline = Instant::now() + EXIT_DRAIN_MAX_DURATION;
    loop {
        if Instant::now() >= deadline {
            break;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let timeout = remaining.min(EXIT_DRAIN_POLL_TIMEOUT);
        match client.recv_event(timeout)? {
            Some(event) => {
                let _ = handle_session_event(mux_domain, event, stdout)?;
            }
            None => continue,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_graceful_detach_error;

    #[test]
    fn graceful_detach_error_detector_matches_expected_messages() {
        assert!(is_graceful_detach_error("session is not running"));
        assert!(is_graceful_detach_error("request failed: session not found"));
        assert!(!is_graceful_detach_error("request timeout"));
    }
}
