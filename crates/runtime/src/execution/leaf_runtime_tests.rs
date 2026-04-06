use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::terminal_text_utils::visible_terminal_fragment;
use config::current_config_handle;
use terminal_emulator::TerminalSize;
use portable_pty::CommandBuilder;

use super::super::{RuntimeId, TerminalInstanceId};
use super::{TerminalInstanceRuntime, TerminalInstanceRuntimeEvent, TerminalInstanceRuntimeSpawn};

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
        config: current_config_handle(),
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

    assert!(
        runtime
            .replay_output()
            .contains("terminal-instance-runtime-smoke")
    );
}

#[test]
fn leaf_runtime_resize_updates_pty_size() {
    let (events_tx, _events_rx) = mpsc::sync_channel(32);
    let runtime = TerminalInstanceRuntime::spawn(runtime_spawn("sleep 1"), events_tx)
        .expect("spawn sleeping runtime");
    runtime
        .resize(TerminalSize {
            rows: 30,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 96,
        })
        .expect("resize runtime");
}

#[test]
fn leaf_runtime_seeds_terminal_from_initial_scrollback() {
    let (events_tx, _events_rx) = mpsc::sync_channel(32);
    let mut spawn = runtime_spawn("sleep 1");
    spawn.initial_scrollback = Some("restored-line-1\nrestored-line-2\n".to_string());

    let runtime = TerminalInstanceRuntime::spawn(spawn, events_tx).expect("spawn runtime");

    assert!(runtime.replay_output().contains("restored-line-1"));
    assert!(runtime.replay_output().contains("restored-line-2"));
}

#[test]
fn leaf_runtime_ignores_zsh_prompt_redraw_artifact_after_restore() {
    let (events_tx, events_rx) = mpsc::sync_channel(32);
    let mut spawn = runtime_spawn(
        "printf '\\033[1m\\033[7m%%\\033[27m\\033[1m\\033[0m     \\r \\r\\033[0muser@host ~ %% '",
    );
    spawn.initial_scrollback = Some("user@host ~ % ".to_string());

    let runtime = TerminalInstanceRuntime::spawn(spawn, events_tx).expect("spawn runtime");

    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(3) {
        if let Ok(TerminalInstanceRuntimeEvent::Exited { .. }) =
            events_rx.recv_timeout(Duration::from_millis(200))
        {
            break;
        }
    }

    let replay = runtime.replay_output();
    assert_eq!(visible_terminal_fragment(&replay), "user@host ~ % ");
}

#[test]
fn leaf_runtime_write_input_reaches_child_immediately() {
    let (events_tx, events_rx) = mpsc::sync_channel(32);
    let runtime = TerminalInstanceRuntime::spawn(
        runtime_spawn("stty raw -echo; dd bs=1 count=1 2>/dev/null"),
        events_tx,
    )
    .expect("spawn runtime");

    runtime.write_input(b"Z").expect("write input");

    let started = Instant::now();
    let mut saw_output = false;
    while started.elapsed() < Duration::from_secs(3) {
        if let Ok(TerminalInstanceRuntimeEvent::Output { chunk, .. }) =
            events_rx.recv_timeout(Duration::from_millis(200))
        {
            if chunk.contains('Z') {
                saw_output = true;
                break;
            }
        }
    }

    assert!(saw_output, "expected child to echo immediate input back");
}

#[test]
fn leaf_runtime_closes_input_after_process_exit() {
    let (events_tx, events_rx) = mpsc::sync_channel(32);
    let runtime = TerminalInstanceRuntime::spawn(runtime_spawn("printf 'done'"), events_tx)
        .expect("spawn runtime");

    let started = Instant::now();
    let mut saw_exit = false;
    while started.elapsed() < Duration::from_secs(3) {
        match events_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(TerminalInstanceRuntimeEvent::Exited { .. }) => {
                saw_exit = true;
                break;
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    assert!(saw_exit, "expected runtime to exit");
    assert!(runtime.write_input(b"late").is_err());
}

#[test]
fn leaf_runtime_emits_exit_event_with_child_status() {
    let (events_tx, events_rx) = mpsc::sync_channel(32);
    let _runtime = TerminalInstanceRuntime::spawn(runtime_spawn("exit 7"), events_tx)
        .expect("spawn runtime that exits");

    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(3) {
        if let Ok(TerminalInstanceRuntimeEvent::Exited { exit_code, .. }) =
            events_rx.recv_timeout(Duration::from_millis(200))
        {
            assert_eq!(exit_code, Some(7));
            return;
        }
    }

    panic!("expected exited event with exit code");
}
