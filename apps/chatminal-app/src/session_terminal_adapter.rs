use std::time::{Duration, Instant};

use chatminal_protocol::{
    DaemonHealthEvent, Event, PtyErrorEvent, PtyExitedEvent, PtyOutputEvent, SessionSnapshot,
    SessionUpdatedEvent, WorkspaceUpdatedEvent,
};

use crate::ipc::ChatminalClient;

#[path = "session_terminal_registry.rs"]
mod terminal_registry;
#[path = "session_terminal_stdout_adapter.rs"]
mod stdout_adapter;

pub use terminal_registry::SessionTerminalRegistry;
pub use stdout_adapter::StdoutJsonSessionTerminalAdapter;

pub trait SessionTerminalAdapter {
    fn on_pty_output(&mut self, _event: PtyOutputEvent) {}
    fn on_pty_exited(&mut self, _event: PtyExitedEvent) {}
    fn on_pty_error(&mut self, _event: PtyErrorEvent) {}
    fn on_session_output(&mut self, _session_id: &str, _terminal_id: &str, _event: &PtyOutputEvent) {}
    fn on_session_activated(
        &mut self,
        _session_id: &str,
        _terminal_id: &str,
        _cols: usize,
        _rows: usize,
    ) {
    }
    fn on_session_snapshot(
        &mut self,
        _session_id: &str,
        _terminal_id: &str,
        _snapshot: &SessionSnapshot,
    ) {
    }
    fn on_session_input(&mut self, _session_id: &str, _terminal_id: &str, _byte_len: usize) {}
    fn on_session_resize(&mut self, _session_id: &str, _terminal_id: &str, _cols: usize, _rows: usize) {
    }
    fn on_session_updated(&mut self, _event: SessionUpdatedEvent) {}
    fn on_workspace_updated(&mut self, _event: WorkspaceUpdatedEvent) {}
    fn on_daemon_health(&mut self, _event: DaemonHealthEvent) {}
}

#[allow(dead_code)]
pub fn dispatch_event(adapter: &mut dyn SessionTerminalAdapter, event: Event) {
    let mut registry = SessionTerminalRegistry::new();
    dispatch_event_with_registry(adapter, &mut registry, event);
}

pub fn dispatch_event_with_registry(
    adapter: &mut dyn SessionTerminalAdapter,
    registry: &mut SessionTerminalRegistry,
    event: Event,
) {
    match event {
        Event::PtyOutput(value) => {
            let terminal_id = registry.ensure_terminal_for_session(&value.session_id);
            adapter.on_session_output(&value.session_id, &terminal_id, &value);
            adapter.on_pty_output(value);
        }
        Event::PtyExited(value) => adapter.on_pty_exited(value),
        Event::PtyError(value) => adapter.on_pty_error(value),
        Event::SessionUpdated(value) => adapter.on_session_updated(value),
        Event::WorkspaceUpdated(value) => adapter.on_workspace_updated(value),
        Event::DaemonHealth(value) => adapter.on_daemon_health(value),
    }
}

pub fn pump_events(
    client: &ChatminalClient,
    adapter: &mut dyn SessionTerminalAdapter,
    duration: Duration,
) -> Result<usize, String> {
    let mut registry = SessionTerminalRegistry::new();
    pump_events_with_registry(client, &mut registry, adapter, duration)
}

pub fn pump_events_with_registry(
    client: &ChatminalClient,
    registry: &mut SessionTerminalRegistry,
    adapter: &mut dyn SessionTerminalAdapter,
    duration: Duration,
) -> Result<usize, String> {
    let deadline = Instant::now() + duration;
    let mut processed = 0usize;
    while Instant::now() < deadline {
        let Some(event) = client.recv_event(Duration::from_millis(300))? else {
            continue;
        };
        dispatch_event_with_registry(adapter, registry, event);
        processed += 1;
    }
    Ok(processed)
}
