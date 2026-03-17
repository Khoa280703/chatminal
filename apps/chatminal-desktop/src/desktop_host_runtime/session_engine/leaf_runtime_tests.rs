use std::sync::mpsc;
use std::time::{Duration, Instant};

use chatminal_terminal_core::{ScreenLine, TerminalSize};
use portable_pty::CommandBuilder;

use super::{TerminalInstanceRuntime, TerminalInstanceRuntimeEvent, TerminalInstanceRuntimeSpawn};
use super::super::{TerminalInstanceId, RuntimeId};

fn shell_command(script: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("/bin/sh");
    command.arg("-lc");
    command.arg(script);
    command
}

fn runtime_spawn(script: &str) -> TerminalInstanceRuntimeSpawn {
    TerminalInstanceRuntimeSpawn {
        session_id: "session-a".into(),
        generation: 1,
        runtime_id: RuntimeId::new(7),
        terminal_instance_id: TerminalInstanceId::new(11),
        command: shell_command(script),
        size: TerminalSize {
            rows: 12,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 96,
        },
        initial_scrollback: None,
    }
}

#[test]
fn leaf_runtime_captures_output_into_terminal_state() {
    let (events_tx, events_rx) = mpsc::sync_channel(32);
    let runtime = TerminalInstanceRuntime::spawn(
        runtime_spawn("printf 'terminal-instance-runtime-smoke'"),
        events_tx,
    )
        .expect("spawn terminal instance runtime");

    let started = Instant::now();
    let mut saw_output = false;
    while started.elapsed() < Duration::from_secs(3) {
        if let Ok(TerminalInstanceRuntimeEvent::Output { chunk, .. }) =
            events_rx.recv_timeout(Duration::from_millis(200))
        {
            if chunk.contains("terminal-instance-runtime-smoke") {
                saw_output = true;
                break;
            }
        }
    }
    assert!(saw_output);

    let rendered = runtime
        .screen()
        .lines_in_phys_range(0..runtime.screen().scrollback_rows())
        .iter()
        .map(ScreenLine::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("terminal-instance-runtime-smoke"));
    assert!(runtime.replay_output().contains("terminal-instance-runtime-smoke"));
}

#[test]
fn leaf_runtime_resize_updates_terminal_snapshot() {
    let (events_tx, _events_rx) = mpsc::sync_channel(32);
    let runtime =
        TerminalInstanceRuntime::spawn(runtime_spawn("sleep 1"), events_tx).expect("spawn sleeping runtime");
    runtime
        .resize(TerminalSize {
            rows: 30,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 96,
        })
        .expect("resize runtime");

    assert_eq!(runtime.screen().physical_rows, 30);
}

#[test]
fn leaf_runtime_seeds_terminal_from_initial_scrollback() {
    let (events_tx, _events_rx) = mpsc::sync_channel(32);
    let mut spawn = runtime_spawn("sleep 1");
    spawn.initial_scrollback = Some("restored-line-1\nrestored-line-2\n".to_string());

    let runtime = TerminalInstanceRuntime::spawn(spawn, events_tx).expect("spawn runtime");
    let rendered = runtime
        .screen()
        .lines_in_phys_range(0..runtime.screen().scrollback_rows())
        .iter()
        .map(ScreenLine::as_str)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("restored-line-1"));
    assert!(rendered.contains("restored-line-2"));
    assert!(runtime.replay_output().contains("restored-line-1"));
}
