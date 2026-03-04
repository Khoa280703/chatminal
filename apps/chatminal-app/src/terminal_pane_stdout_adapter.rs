use chatminal_protocol::{
    DaemonHealthEvent, Event, PtyErrorEvent, PtyExitedEvent, PtyOutputEvent, SessionSnapshot,
    SessionUpdatedEvent, WorkspaceUpdatedEvent,
};

use super::TerminalPaneAdapter;

pub struct StdoutJsonTerminalPaneAdapter;

impl TerminalPaneAdapter for StdoutJsonTerminalPaneAdapter {
    fn on_pty_output(&mut self, event: PtyOutputEvent) {
        print_event(Event::PtyOutput(event));
    }

    fn on_pty_exited(&mut self, event: PtyExitedEvent) {
        print_event(Event::PtyExited(event));
    }

    fn on_pty_error(&mut self, event: PtyErrorEvent) {
        print_event(Event::PtyError(event));
    }

    fn on_session_output(&mut self, session_id: &str, pane_id: &str, event: &PtyOutputEvent) {
        print_named_payload(
            "session_output",
            serde_json::json!({
                "session_id": session_id,
                "pane_id": pane_id,
                "seq": event.seq,
                "chunk_len": event.chunk.len(),
                "ts": event.ts
            }),
        );
    }

    fn on_session_activated(&mut self, session_id: &str, pane_id: &str, cols: usize, rows: usize) {
        print_named_payload(
            "session_activated",
            serde_json::json!({
                "session_id": session_id,
                "pane_id": pane_id,
                "cols": cols,
                "rows": rows
            }),
        );
    }

    fn on_session_snapshot(&mut self, session_id: &str, pane_id: &str, snapshot: &SessionSnapshot) {
        print_named_payload(
            "session_snapshot",
            serde_json::json!({
                "session_id": session_id,
                "pane_id": pane_id,
                "seq": snapshot.seq,
                "content_len": snapshot.content.len()
            }),
        );
    }

    fn on_session_input(&mut self, session_id: &str, pane_id: &str, byte_len: usize) {
        print_named_payload(
            "session_input",
            serde_json::json!({
                "session_id": session_id,
                "pane_id": pane_id,
                "byte_len": byte_len
            }),
        );
    }

    fn on_session_resize(&mut self, session_id: &str, pane_id: &str, cols: usize, rows: usize) {
        print_named_payload(
            "session_resize",
            serde_json::json!({
                "session_id": session_id,
                "pane_id": pane_id,
                "cols": cols,
                "rows": rows
            }),
        );
    }

    fn on_session_updated(&mut self, event: SessionUpdatedEvent) {
        print_event(Event::SessionUpdated(event));
    }

    fn on_workspace_updated(&mut self, event: WorkspaceUpdatedEvent) {
        print_event(Event::WorkspaceUpdated(event));
    }

    fn on_daemon_health(&mut self, event: DaemonHealthEvent) {
        print_event(Event::DaemonHealth(event));
    }
}

fn print_event(event: Event) {
    match serde_json::to_string_pretty(&event) {
        Ok(encoded) => println!("{encoded}"),
        Err(err) => eprintln!("chatminal-app: encode event failed: {err}"),
    }
}

fn print_named_payload<T: serde::Serialize>(name: &str, payload: T) {
    match serde_json::to_string_pretty(&serde_json::json!({
        "type": name,
        "payload": payload
    })) {
        Ok(encoded) => println!("{encoded}"),
        Err(err) => eprintln!("chatminal-app: encode adapter payload failed: {err}"),
    }
}
