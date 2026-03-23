use std::io::{Read, Write};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use chatminal_terminal_core::{Terminal, TerminalSize};
use portable_pty::{Child, CommandBuilder, PtySize};

use super::leaf_runtime::{TerminalInstanceRuntimeEvent, TerminalInstanceRuntimeSpawn};

pub(crate) fn spawn_reader_loop(
    terminal: Arc<Mutex<Terminal>>,
    output_history: Arc<Mutex<Vec<String>>>,
    spawn: TerminalInstanceRuntimeSpawn,
    events: std_mpsc::SyncSender<TerminalInstanceRuntimeEvent>,
    mut reader: Box<dyn Read + Send>,
) {
    thread::spawn(move || {
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let sanitized = sanitize_zsh_prompt_spacer(&buffer[..read]);
                    terminal.lock().unwrap().advance_bytes(&sanitized);
                    let chunk = String::from_utf8_lossy(&sanitized).to_string();
                    output_history.lock().unwrap().push(chunk.clone());
                    let _ = events.send(TerminalInstanceRuntimeEvent::Output {
                        session_id: spawn.session_id.clone(),
                        generation: spawn.generation,
                        runtime_id: spawn.runtime_id,
                        terminal_instance_id: spawn.terminal_instance_id,
                        chunk,
                    });
                }
                Err(err) => {
                    let _ = events.send(TerminalInstanceRuntimeEvent::Error {
                        session_id: spawn.session_id.clone(),
                        generation: spawn.generation,
                        runtime_id: spawn.runtime_id,
                        terminal_instance_id: spawn.terminal_instance_id,
                        message: format!("pty read failed: {err}"),
                    });
                    break;
                }
            }
        }
    });
}

pub(crate) fn spawn_writer_loop(
    spawn: TerminalInstanceRuntimeSpawn,
    events: std_mpsc::SyncSender<TerminalInstanceRuntimeEvent>,
    mut writer: Box<dyn Write + Send>,
    input_rx: std_mpsc::Receiver<Vec<u8>>,
) {
    thread::spawn(move || {
        while let Ok(chunk) = input_rx.recv() {
            if let Err(err) = writer.write_all(&chunk).and_then(|_| writer.flush()) {
                let _ = events.send(TerminalInstanceRuntimeEvent::Error {
                    session_id: spawn.session_id.clone(),
                    generation: spawn.generation,
                    runtime_id: spawn.runtime_id,
                    terminal_instance_id: spawn.terminal_instance_id,
                    message: format!("pty write failed: {err}"),
                });
                break;
            }
        }
    });
}

pub(crate) fn spawn_waiter_loop(
    spawn: TerminalInstanceRuntimeSpawn,
    events: std_mpsc::SyncSender<TerminalInstanceRuntimeEvent>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
) {
    thread::spawn(move || {
        loop {
            let status = child
                .lock()
                .ok()
                .and_then(|mut guard| guard.try_wait().ok())
                .flatten();
            if let Some(status) = status {
                let _ = events.send(TerminalInstanceRuntimeEvent::Exited {
                    session_id: spawn.session_id,
                    generation: spawn.generation,
                    runtime_id: spawn.runtime_id,
                    terminal_instance_id: spawn.terminal_instance_id,
                    exit_code: Some(status.exit_code() as i32),
                });
                break;
            }
            thread::sleep(std::time::Duration::from_millis(120));
        }
    });
}

pub(crate) fn to_pty_size(size: TerminalSize) -> PtySize {
    PtySize {
        rows: size.rows.clamp(2, u16::MAX as usize) as u16,
        cols: size.cols.clamp(2, u16::MAX as usize) as u16,
        pixel_width: size.pixel_width as u16,
        pixel_height: size.pixel_height as u16,
    }
}

pub(crate) fn command_label(command: &CommandBuilder) -> Option<String> {
    command
        .get_argv()
        .first()
        .map(|value| value.to_string_lossy().to_string())
}

pub(crate) fn sanitize_zsh_prompt_spacer(bytes: &[u8]) -> Vec<u8> {
    const PREFIX: &[u8] = b"\x1b[1m\x1b[7m%\x1b[27m\x1b[1m\x1b[0m";
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i..].starts_with(PREFIX) {
            let mut j = i + PREFIX.len();
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            if bytes.get(j) == Some(&b'\r') {
                j += 1;
                if bytes.get(j) == Some(&b' ') {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'\r') {
                    i = j + 1;
                    continue;
                }
            }
        }

        out.push(bytes[i]);
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::sanitize_zsh_prompt_spacer;

    #[test]
    fn strips_zsh_prompt_spacer_artifact() {
        let raw = b"\x1b[1m\x1b[7m%\x1b[27m\x1b[1m\x1b[0m     \r \r\x1b[0muser@host % ";
        let sanitized = sanitize_zsh_prompt_spacer(raw);
        assert_eq!(sanitized, b"\x1b[0muser@host % ");
    }

    #[test]
    fn keeps_normal_prompt_output_untouched() {
        let raw = b"\r\r\x1b[0m\x1b[27m\x1b[24m\x1b[Juser@host % ";
        let sanitized = sanitize_zsh_prompt_spacer(raw);
        assert_eq!(sanitized, raw);
    }
}
