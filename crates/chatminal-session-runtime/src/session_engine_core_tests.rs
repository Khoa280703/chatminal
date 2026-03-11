use std::sync::{Arc, Mutex};
use std::time::Duration;

use chatminal_terminal_core::TerminalSize;
use portable_pty::CommandBuilder;

use crate::{SessionCoreState, SessionRuntimeEvent, StatefulSessionEngine};

fn shell_command(script: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("/bin/sh");
    command.arg("-lc");
    command.arg(script);
    command
}

#[test]
fn detached_surface_spawn_registers_runtime_and_layout() {
    let engine = StatefulSessionEngine::new((), Arc::new(Mutex::new(SessionCoreState::default())));
    let events = engine.event_hub().subscribe();

    let state = engine
        .spawn_detached_surface(
            "session-a",
            1,
            shell_command("sleep 1"),
            TerminalSize {
                rows: 20,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
        )
        .expect("spawn detached surface");
    assert_eq!(
        events.recv_timeout(Duration::from_secs(1)).unwrap(),
        Some(SessionRuntimeEvent::SurfaceAttached {
            session_id: "session-a".into(),
            surface_id: state.snapshot.surface_id,
        })
    );

    let core = engine.core_state_handle();
    let core = core.lock().unwrap();
    let surface = core
        .surface(state.snapshot.surface_id)
        .expect("surface state");
    let active_leaf = state.snapshot.active_leaf_id.expect("active leaf id");
    assert_eq!(surface.session_id, "session-a");
    assert_eq!(surface.active_leaf_id, Some(active_leaf));
    assert!(
        surface
            .leaves
            .get(&active_leaf)
            .and_then(|leaf| leaf.process.clone())
            .is_some()
    );
    assert!(
        engine
            .leaf_runtime_registry()
            .runtime(active_leaf)
            .is_some()
    );
}

#[test]
fn detached_surface_close_cleans_runtime_and_core_state() {
    let engine = StatefulSessionEngine::new((), Arc::new(Mutex::new(SessionCoreState::default())));
    let state = engine
        .spawn_detached_surface(
            "session-a",
            1,
            shell_command("sleep 5"),
            TerminalSize {
                rows: 20,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
        )
        .expect("spawn detached surface");
    let active_leaf = state.snapshot.active_leaf_id.expect("active leaf id");

    assert!(engine.close_detached_surface(state.snapshot.surface_id));
    assert!(
        engine
            .core_state_handle()
            .lock()
            .unwrap()
            .surface(state.snapshot.surface_id)
            .is_none()
    );
    assert!(
        engine
            .leaf_runtime_registry()
            .runtime(active_leaf)
            .is_none()
    );
}

#[test]
fn detached_surface_output_can_be_replayed_from_registry() {
    let engine = StatefulSessionEngine::new((), Arc::new(Mutex::new(SessionCoreState::default())));
    let state = engine
        .spawn_detached_surface(
            "session-a",
            1,
            shell_command("printf 'replay-smoke'"),
            TerminalSize {
                rows: 20,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
        )
        .expect("spawn detached surface");
    let leaf_id = state.snapshot.active_leaf_id.expect("active leaf");

    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(3) {
        if engine
            .replay_leaf_output(leaf_id)
            .is_some_and(|output| output.contains("replay-smoke"))
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("leaf output replay did not contain expected text");
}
