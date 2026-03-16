use chatminal_protocol::{PtyOutputEvent, SessionSnapshot};

use crate::session_terminal_adapter::SessionTerminalAdapter;
use crate::session_terminal_emulator::SessionTerminalEmulator;

#[test]
fn snapshot_and_output_are_applied_to_session_terminal_emulator() {
    let mut adapter = SessionTerminalEmulator::new(80, 24, 5_000);
    adapter.on_session_activated("s-1", "terminal-1", 80, 24);
    adapter.on_session_snapshot(
        "s-1",
        "terminal-1",
        &SessionSnapshot {
            content: "hello\r\n".to_string(),
            seq: 1,
        },
    );
    adapter.on_session_output(
        "s-1",
        "terminal-1",
        &PtyOutputEvent {
            session_id: "s-1".to_string(),
            chunk: "world\r\n".to_string(),
            seq: 2,
            ts: 0,
        },
    );

    let snapshot = adapter
        .terminal_snapshot("terminal-1")
        .expect("terminal snapshot should exist");
    assert!(snapshot.visible_text.contains("hello"));
    assert!(snapshot.visible_text.contains("world"));
}

#[test]
fn resize_updates_terminal_dimensions() {
    let mut adapter = SessionTerminalEmulator::new(120, 32, 5_000);
    adapter.on_session_activated("s-2", "terminal-2", 120, 32);
    adapter.on_session_resize("s-2", "terminal-2", 140, 40);

    let snapshot = adapter
        .terminal_snapshot("terminal-2")
        .expect("terminal snapshot should exist");
    assert_eq!(snapshot.cols, 140);
    assert_eq!(snapshot.rows, 40);
}
