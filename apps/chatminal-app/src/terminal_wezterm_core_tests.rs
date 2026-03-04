use chatminal_protocol::{PtyOutputEvent, SessionSnapshot};

use crate::terminal_pane_adapter::TerminalPaneAdapter;
use crate::terminal_wezterm_core::WeztermTerminalPaneAdapter;

#[test]
fn snapshot_and_output_are_applied_to_wezterm_core() {
    let mut adapter = WeztermTerminalPaneAdapter::new(80, 24, 5_000);
    adapter.on_session_activated("s-1", "pane-1", 80, 24);
    adapter.on_session_snapshot(
        "s-1",
        "pane-1",
        &SessionSnapshot {
            content: "hello\r\n".to_string(),
            seq: 1,
        },
    );
    adapter.on_session_output(
        "s-1",
        "pane-1",
        &PtyOutputEvent {
            session_id: "s-1".to_string(),
            chunk: "world\r\n".to_string(),
            seq: 2,
            ts: 0,
        },
    );

    let snapshot = adapter
        .pane_snapshot("pane-1")
        .expect("pane snapshot should exist");
    assert!(snapshot.visible_text.contains("hello"));
    assert!(snapshot.visible_text.contains("world"));
}

#[test]
fn resize_updates_pane_dimensions() {
    let mut adapter = WeztermTerminalPaneAdapter::new(120, 32, 5_000);
    adapter.on_session_activated("s-2", "pane-2", 120, 32);
    adapter.on_session_resize("s-2", "pane-2", 140, 40);

    let snapshot = adapter
        .pane_snapshot("pane-2")
        .expect("pane snapshot should exist");
    assert_eq!(snapshot.cols, 140);
    assert_eq!(snapshot.rows, 40);
}
