use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use portable_pty::CommandBuilder;
use terminal_emulator::TerminalSize;

use super::super::{
    LayoutNodeId, RuntimeId, SessionCoreState, SessionLayoutSnapshot, TerminalInstanceId,
};
use super::TerminalInstanceRuntimeRegistry;

fn shell_command(script: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("/bin/sh");
    command.arg("-lc");
    command.arg(script);
    command
}

#[test]
fn registry_spawn_updates_core_state_process_metadata() {
    let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
    core_state.lock().unwrap().sync_runtime_layout(
        "session-a",
        RuntimeId::new(7),
        &SessionLayoutSnapshot::single_terminal_instance(
            LayoutNodeId::new(1),
            TerminalInstanceId::new(11),
            None,
        ),
    );
    let registry = TerminalInstanceRuntimeRegistry::default();
    let (events_tx, _events_rx) = mpsc::sync_channel(32);

    let _runtime = registry
        .spawn_for_runtime(
            &core_state,
            "session-a",
            1,
            RuntimeId::new(7),
            TerminalInstanceId::new(11),
            shell_command("sleep 1"),
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            None,
            events_tx,
        )
        .expect("spawn runtime for runtime");

    let process = core_state
        .lock()
        .unwrap()
        .runtime(RuntimeId::new(7))
        .and_then(|runtime| runtime.leaves.get(&TerminalInstanceId::new(11)))
        .and_then(|leaf| leaf.process.clone());
    assert!(process.and_then(|value| value.process_id).is_some());
    assert!(registry.runtime(TerminalInstanceId::new(11)).is_some());
}

#[test]
fn registry_remove_clears_core_state_process_metadata() {
    let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
    core_state.lock().unwrap().sync_runtime_layout(
        "session-a",
        RuntimeId::new(7),
        &SessionLayoutSnapshot::single_terminal_instance(
            LayoutNodeId::new(1),
            TerminalInstanceId::new(11),
            None,
        ),
    );
    let registry = TerminalInstanceRuntimeRegistry::default();
    let (events_tx, _events_rx) = mpsc::sync_channel(32);

    let _runtime = registry
        .spawn_for_runtime(
            &core_state,
            "session-a",
            1,
            RuntimeId::new(7),
            TerminalInstanceId::new(11),
            shell_command("sleep 5"),
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            None,
            events_tx,
        )
        .expect("spawn runtime for runtime");
    registry
        .remove_for_runtime(&core_state, RuntimeId::new(7), TerminalInstanceId::new(11))
        .expect("remove runtime");

    let process = core_state
        .lock()
        .unwrap()
        .runtime(RuntimeId::new(7))
        .and_then(|runtime| runtime.leaves.get(&TerminalInstanceId::new(11)))
        .and_then(|leaf| leaf.process.clone());
    assert!(process.is_none());
    assert!(registry.runtime(TerminalInstanceId::new(11)).is_none());
}
