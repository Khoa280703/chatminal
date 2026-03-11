use std::sync::mpsc;
use std::time::{Duration, Instant};

use chatminal_terminal_core::{ScreenLine, TerminalSize};
use portable_pty::CommandBuilder;

use super::{LeafRuntime, LeafRuntimeEvent, LeafRuntimeSpawn};
use crate::{LeafId, SurfaceId};

fn shell_command(script: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("/bin/sh");
    command.arg("-lc");
    command.arg(script);
    command
}

fn runtime_spawn(script: &str) -> LeafRuntimeSpawn {
    LeafRuntimeSpawn {
        session_id: "session-a".into(),
        generation: 1,
        surface_id: SurfaceId::new(7),
        leaf_id: LeafId::new(11),
        command: shell_command(script),
        size: TerminalSize {
            rows: 12,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 96,
        },
    }
}

#[test]
fn leaf_runtime_captures_output_into_terminal_state() {
    let (events_tx, events_rx) = mpsc::sync_channel(32);
    let runtime = LeafRuntime::spawn(runtime_spawn("printf 'leaf-runtime-smoke'"), events_tx)
        .expect("spawn leaf runtime");

    let started = Instant::now();
    let mut saw_output = false;
    while started.elapsed() < Duration::from_secs(3) {
        if let Ok(LeafRuntimeEvent::Output { chunk, .. }) =
            events_rx.recv_timeout(Duration::from_millis(200))
        {
            if chunk.contains("leaf-runtime-smoke") {
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
    assert!(rendered.contains("leaf-runtime-smoke"));
    assert!(runtime.replay_output().contains("leaf-runtime-smoke"));
}

#[test]
fn leaf_runtime_resize_updates_terminal_snapshot() {
    let (events_tx, _events_rx) = mpsc::sync_channel(32);
    let runtime =
        LeafRuntime::spawn(runtime_spawn("sleep 1"), events_tx).expect("spawn sleeping runtime");
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
