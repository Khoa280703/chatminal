use std::time::Duration;

use chatminal_protocol::{Request, Response, SessionSnapshot};
use serde::Serialize;

use crate::ipc::ChatminalClient;
use crate::terminal_pane_adapter::{SessionPaneRegistry, TerminalPaneAdapter};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize)]
pub struct SessionActivationResult {
    pub session_id: String,
    pub pane_id: String,
    pub cols: usize,
    pub rows: usize,
    pub snapshot_seq: u64,
    pub snapshot_len: usize,
}

pub fn activate_session_with_snapshot(
    client: &ChatminalClient,
    registry: &mut SessionPaneRegistry,
    adapter: &mut dyn TerminalPaneAdapter,
    session_id: &str,
    cols: usize,
    rows: usize,
    preview_lines: usize,
) -> Result<SessionActivationResult, String> {
    expect_empty(
        client.request(
            Request::SessionActivate {
                session_id: session_id.to_string(),
                cols,
                rows,
            },
            REQUEST_TIMEOUT,
        )?,
        "session_activate",
    )?;

    let pane_id = registry.activate_session(session_id);
    adapter.on_session_activated(session_id, &pane_id, cols, rows);

    let snapshot = fetch_snapshot_for_session(client, session_id, preview_lines)?;
    adapter.on_session_snapshot(session_id, &pane_id, &snapshot);

    Ok(SessionActivationResult {
        session_id: session_id.to_string(),
        pane_id,
        cols,
        rows,
        snapshot_seq: snapshot.seq,
        snapshot_len: snapshot.content.len(),
    })
}

pub fn write_input_for_session(
    client: &ChatminalClient,
    registry: &mut SessionPaneRegistry,
    adapter: &mut dyn TerminalPaneAdapter,
    session_id: &str,
    data: &str,
) -> Result<(), String> {
    write_input_for_session_with_timeout(
        client,
        registry,
        adapter,
        session_id,
        data,
        REQUEST_TIMEOUT,
    )
}

pub fn write_input_for_session_with_timeout(
    client: &ChatminalClient,
    registry: &mut SessionPaneRegistry,
    adapter: &mut dyn TerminalPaneAdapter,
    session_id: &str,
    data: &str,
    timeout: Duration,
) -> Result<(), String> {
    expect_empty(
        client.request(
            Request::SessionInputWrite {
                session_id: session_id.to_string(),
                data: data.to_string(),
            },
            timeout,
        )?,
        "session_input_write",
    )?;

    let pane_id = registry.ensure_pane_for_session(session_id);
    adapter.on_session_input(session_id, &pane_id, data.len());
    Ok(())
}

pub fn resize_session(
    client: &ChatminalClient,
    registry: &mut SessionPaneRegistry,
    adapter: &mut dyn TerminalPaneAdapter,
    session_id: &str,
    cols: usize,
    rows: usize,
) -> Result<(), String> {
    expect_empty(
        client.request(
            Request::SessionResize {
                session_id: session_id.to_string(),
                cols,
                rows,
            },
            REQUEST_TIMEOUT,
        )?,
        "session_resize",
    )?;

    let pane_id = registry.ensure_pane_for_session(session_id);
    adapter.on_session_resize(session_id, &pane_id, cols, rows);
    Ok(())
}

pub fn fetch_snapshot_for_session(
    client: &ChatminalClient,
    session_id: &str,
    preview_lines: usize,
) -> Result<SessionSnapshot, String> {
    let response = client.request(
        Request::SessionSnapshotGet {
            session_id: session_id.to_string(),
            preview_lines: Some(preview_lines),
        },
        REQUEST_TIMEOUT,
    )?;
    expect_snapshot(response, "session_snapshot_get")
}

fn expect_empty(response: Response, op: &str) -> Result<(), String> {
    match response {
        Response::Empty => Ok(()),
        other => Err(format!("unexpected response for {op}: {:?}", other)),
    }
}

fn expect_snapshot(response: Response, op: &str) -> Result<SessionSnapshot, String> {
    match response {
        Response::SessionSnapshot(value) => Ok(value),
        other => Err(format!("unexpected response for {op}: {:?}", other)),
    }
}
