use std::sync::{Arc, Mutex};
use std::time::Duration;

use engine_term::TerminalSize;
use portable_pty::CommandBuilder;

use super::super::{
    RuntimeId, SessionCoreState, SessionRuntimeEvent, StatefulSessionEngine, TerminalInstanceId,
};

fn shell_command(script: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("/bin/sh");
    command.arg("-lc");
    command.arg(script);
    command
}

#[test]
fn detached_runtime_spawn_registers_runtime_and_layout() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let events = engine.subscribe();

    let state = engine
        .spawn_detached_runtime(
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
            None,
        )
        .expect("spawn detached runtime");
    assert_eq!(
        events.recv_timeout(Duration::from_secs(1)).unwrap(),
        Some(SessionRuntimeEvent::RuntimeAttached {
            session_id: "session-a".into(),
            runtime_id: state.snapshot.runtime_id,
        })
    );

    let core = engine.core_state_handle();
    let core = core.lock().unwrap();
    let runtime = core
        .runtime(state.snapshot.runtime_id)
        .expect("runtime state");
    let active_terminal_instance = state
        .snapshot
        .active_terminal_instance_id
        .expect("active leaf id");
    assert_eq!(runtime.session_id, "session-a");
    assert_eq!(
        runtime.active_terminal_instance_id,
        Some(active_terminal_instance)
    );
    assert!(runtime
        .leaves
        .get(&active_terminal_instance)
        .and_then(|leaf| leaf.process.clone())
        .is_some());
    assert!(engine
        .leaf_runtime_registry()
        .runtime(active_terminal_instance)
        .is_some());
}

#[test]
fn detached_runtime_close_cleans_runtime_and_core_state() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let state = engine
        .spawn_detached_runtime(
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
            None,
        )
        .expect("spawn detached runtime");
    let active_terminal_instance = state
        .snapshot
        .active_terminal_instance_id
        .expect("active leaf id");

    assert!(engine.close_detached_runtime(state.snapshot.runtime_id));
    assert!(engine
        .core_state_handle()
        .lock()
        .unwrap()
        .runtime(state.snapshot.runtime_id)
        .is_none());
    assert!(engine
        .leaf_runtime_registry()
        .runtime(active_terminal_instance)
        .is_none());
}

#[test]
fn snapshot_runtime_from_core_returns_none_for_unknown_runtime() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    assert!(engine
        .snapshot_runtime_from_core(RuntimeId::new(99))
        .is_none());
}

#[test]
fn focus_terminal_instance_native_updates_active_leaf_and_publishes_event() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let events = engine.subscribe();

    let state = engine
        .spawn_detached_runtime(
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
            None,
        )
        .expect("spawn");

    // Drain the RuntimeAttached event
    let _ = events.recv_timeout(Duration::from_secs(1));

    let runtime_id = state.snapshot.runtime_id;
    let terminal_instance_id = state
        .snapshot
        .active_terminal_instance_id
        .expect("active leaf");

    let focused = engine
        .focus_terminal_instance_native("session-a", runtime_id, terminal_instance_id)
        .expect("focus leaf");
    assert_eq!(
        focused.snapshot.active_terminal_instance_id,
        Some(terminal_instance_id)
    );

    let event = events.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        event,
        Some(SessionRuntimeEvent::TerminalInstanceFocused {
            session_id: "session-a".into(),
            runtime_id,
            terminal_instance_id,
        })
    );
}

#[test]
fn focus_terminal_instance_native_returns_none_for_unknown_leaf() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let state = engine
        .spawn_detached_runtime(
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
            None,
        )
        .expect("spawn");
    let runtime_id = state.snapshot.runtime_id;
    assert!(engine
        .focus_terminal_instance_native("session-a", runtime_id, TerminalInstanceId::new(9999))
        .is_none());
}

#[test]
fn focus_runtime_native_publishes_runtime_focused_event() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let events = engine.subscribe();
    let state = engine
        .spawn_detached_runtime(
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
            None,
        )
        .expect("spawn");
    let _ = events.recv_timeout(Duration::from_secs(1)); // drain RuntimeAttached

    let runtime_id = state.snapshot.runtime_id;
    let snapshot = engine
        .focus_runtime_native("session-a", runtime_id)
        .expect("focus runtime");
    assert_eq!(snapshot.snapshot.runtime_id, runtime_id);

    let event = events.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        event,
        Some(SessionRuntimeEvent::RuntimeFocused {
            session_id: "session-a".into(),
            runtime_id,
        })
    );
}

#[test]
fn close_runtime_native_removes_state_and_publishes_closed_event() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let events = engine.subscribe();
    let state = engine
        .spawn_detached_runtime(
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
            None,
        )
        .expect("spawn");
    let _ = events.recv_timeout(Duration::from_secs(1)); // drain RuntimeAttached

    let runtime_id = state.snapshot.runtime_id;
    let terminal_instance_id = state.snapshot.active_terminal_instance_id.expect("leaf");

    assert!(engine.close_runtime_native("session-a", runtime_id));

    assert!(engine
        .core_state_handle()
        .lock()
        .unwrap()
        .runtime(runtime_id)
        .is_none());
    assert!(engine
        .leaf_runtime_registry()
        .runtime(terminal_instance_id)
        .is_none());

    // Drain events until RuntimeClosed (TerminalInstanceOutput/TerminalInstanceExited may arrive first)
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let event = events.recv_timeout(remaining).unwrap();
        match event {
            Some(SessionRuntimeEvent::RuntimeClosed {
                session_id: ref sid,
                runtime_id: sid2,
            }) if &**sid == "session-a" && sid2 == runtime_id => break,
            Some(_) => continue,
            None => panic!("timed out waiting for RuntimeClosed event"),
        }
    }
}

#[test]
fn leaf_process_metadata_is_cleared_after_runtime_exit() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let events = engine.subscribe();
    let state = engine
        .spawn_detached_runtime(
            "session-a",
            1,
            shell_command("printf 'done'"),
            TerminalSize {
                rows: 20,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            None,
        )
        .expect("spawn");

    let _ = events.recv_timeout(Duration::from_secs(1)); // drain RuntimeAttached

    let runtime_id = state.snapshot.runtime_id;
    let terminal_instance_id = state.snapshot.active_terminal_instance_id.expect("leaf");
    let started = std::time::Instant::now();
    let mut saw_exit = false;
    while started.elapsed() < Duration::from_secs(3) {
        match events.recv_timeout(Duration::from_millis(200)).unwrap() {
            Some(SessionRuntimeEvent::TerminalInstanceExited {
                runtime_id: event_runtime_id,
                terminal_instance_id: event_terminal_instance_id,
                ..
            }) if event_runtime_id == runtime_id
                && event_terminal_instance_id == terminal_instance_id =>
            {
                saw_exit = true;
                break;
            }
            Some(_) | None => {}
        }
    }

    assert!(saw_exit, "expected terminal instance exit event");
    assert!(engine
        .core_state_handle()
        .lock()
        .unwrap()
        .runtime(runtime_id)
        .and_then(|runtime| runtime.leaves.get(&terminal_instance_id))
        .and_then(|leaf| leaf.process.as_ref())
        .is_none());
}

#[test]
fn ensure_session_runtime_native_focuses_existing_runtime_without_spawn() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let events = engine.subscribe();

    // First call spawns new runtime
    let state1 = engine
        .ensure_session_runtime_native(
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
            None,
        )
        .expect("first ensure");
    let _ = events.recv_timeout(Duration::from_secs(1)); // drain RuntimeAttached

    // Second call should focus existing runtime (not spawn new)
    let state2 = engine
        .ensure_session_runtime_native(
            "session-a",
            2,
            shell_command("sleep 5"),
            TerminalSize {
                rows: 20,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            None,
        )
        .expect("second ensure");

    assert_eq!(state1.snapshot.runtime_id, state2.snapshot.runtime_id);

    let event = events.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        event,
        Some(SessionRuntimeEvent::RuntimeFocused {
            session_id: "session-a".into(),
            runtime_id: state1.snapshot.runtime_id,
        })
    );
}

#[test]
fn detached_runtime_output_can_be_replayed_from_registry() {
    let engine = StatefulSessionEngine::new(Arc::new(Mutex::new(SessionCoreState::default())));
    let state = engine
        .spawn_detached_runtime(
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
            None,
        )
        .expect("spawn detached runtime");
    let terminal_instance_id = state
        .snapshot
        .active_terminal_instance_id
        .expect("active leaf");

    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(3) {
        if engine
            .replay_leaf_output(terminal_instance_id)
            .is_some_and(|output| output.contains("replay-smoke"))
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("leaf output replay did not contain expected text");
}
