use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use chatminal_terminal_core::TerminalSize;
use portable_pty::CommandBuilder;

use super::LeafRuntimeRegistry;
use crate::{LayoutNodeId, LeafId, SessionCoreState, SessionLayoutSnapshot, SurfaceId};

fn shell_command(script: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("/bin/sh");
    command.arg("-lc");
    command.arg(script);
    command
}

#[test]
fn registry_spawn_updates_core_state_process_metadata() {
    let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
    core_state.lock().unwrap().sync_surface_layout(
        "session-a",
        SurfaceId::new(7),
        &SessionLayoutSnapshot::single_leaf(LayoutNodeId::new(1), LeafId::new(11), None),
    );
    let registry = LeafRuntimeRegistry::default();
    let (events_tx, _events_rx) = mpsc::sync_channel(32);

    let _runtime = registry
        .spawn_for_surface(
            &core_state,
            "session-a",
            1,
            SurfaceId::new(7),
            LeafId::new(11),
            shell_command("sleep 1"),
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            events_tx,
        )
        .expect("spawn runtime for surface");

    let process = core_state
        .lock()
        .unwrap()
        .surface(SurfaceId::new(7))
        .and_then(|surface| surface.leaves.get(&LeafId::new(11)))
        .and_then(|leaf| leaf.process.clone());
    assert!(process.and_then(|value| value.process_id).is_some());
    assert!(registry.runtime(LeafId::new(11)).is_some());
}

#[test]
fn registry_remove_clears_core_state_process_metadata() {
    let core_state = Arc::new(Mutex::new(SessionCoreState::default()));
    core_state.lock().unwrap().sync_surface_layout(
        "session-a",
        SurfaceId::new(7),
        &SessionLayoutSnapshot::single_leaf(LayoutNodeId::new(1), LeafId::new(11), None),
    );
    let registry = LeafRuntimeRegistry::default();
    let (events_tx, _events_rx) = mpsc::sync_channel(32);

    let _runtime = registry
        .spawn_for_surface(
            &core_state,
            "session-a",
            1,
            SurfaceId::new(7),
            LeafId::new(11),
            shell_command("sleep 5"),
            TerminalSize {
                rows: 12,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            events_tx,
        )
        .expect("spawn runtime for surface");
    registry
        .remove_for_surface(&core_state, SurfaceId::new(7), LeafId::new(11))
        .expect("remove runtime");

    let process = core_state
        .lock()
        .unwrap()
        .surface(SurfaceId::new(7))
        .and_then(|surface| surface.leaves.get(&LeafId::new(11)))
        .and_then(|leaf| leaf.process.clone());
    assert!(process.is_none());
    assert!(registry.runtime(LeafId::new(11)).is_none());
}
