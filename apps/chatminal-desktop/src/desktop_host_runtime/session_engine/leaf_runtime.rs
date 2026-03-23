use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};

use chatminal_terminal_core::color::ColorPalette;
#[cfg(test)]
use chatminal_terminal_core::ScreenSnapshot;
use chatminal_terminal_core::{Terminal, TerminalConfiguration, TerminalSize};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty};

use super::leaf_runtime_command::prepare_leaf_command;
use super::leaf_runtime_threads::{
    command_label, sanitize_zsh_prompt_spacer, spawn_reader_loop, spawn_waiter_loop,
    spawn_writer_loop, to_pty_size,
};
use super::{RuntimeId, TerminalInstanceId, TerminalInstanceProcessState};

const INPUT_QUEUE_CAPACITY: usize = 256;

#[derive(Clone, Debug)]
pub struct TerminalInstanceRuntimeSpawn {
    pub session_id: String,
    pub generation: u64,
    pub runtime_id: RuntimeId,
    pub terminal_instance_id: TerminalInstanceId,
    pub command: CommandBuilder,
    pub size: TerminalSize,
    pub initial_scrollback: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalInstanceRuntimeEvent {
    Output {
        session_id: String,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        chunk: String,
    },
    Exited {
        session_id: String,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        exit_code: Option<i32>,
    },
    Error {
        session_id: String,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        message: String,
    },
}

#[derive(Debug)]
struct LeafTerminalConfig;

impl TerminalConfiguration for LeafTerminalConfig {
    fn scrollback_size(&self) -> usize {
        10_000
    }
    fn color_palette(&self) -> ColorPalette {
        ColorPalette
    }
}

pub struct TerminalInstanceRuntime {
    terminal: Arc<Mutex<Terminal>>,
    output_history: Arc<Mutex<Vec<String>>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    input_tx: Mutex<Option<std_mpsc::SyncSender<Vec<u8>>>>,
}

impl TerminalInstanceRuntime {
    pub fn spawn(
        spawn: TerminalInstanceRuntimeSpawn,
        events: std_mpsc::SyncSender<TerminalInstanceRuntimeEvent>,
    ) -> Result<Self, String> {
        let pty = native_pty_system();
        let pair = pty
            .openpty(to_pty_size(spawn.size))
            .map_err(|err| format!("open pty failed: {err}"))?;
        let mut command = spawn.command.clone();
        prepare_leaf_command(&mut command)?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| format!("spawn command failed: {err}"))?;
        let child = Arc::new(Mutex::new(child));
        let process_state = TerminalInstanceProcessState {
            process_id: child.lock().ok().and_then(|guard| guard.process_id()),
            command_label: command_label(&spawn.command),
        };
        let terminal = Arc::new(Mutex::new(Terminal::new(
            spawn.size,
            Arc::new(LeafTerminalConfig),
            "Chatminal",
            env!("CARGO_PKG_VERSION"),
            Box::new(std::io::sink()),
        )));
        let output_history = Arc::new(Mutex::new(Vec::new()));
        if let Some(scrollback) = spawn
            .initial_scrollback
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            let sanitized = sanitize_zsh_prompt_spacer(scrollback.as_bytes());
            terminal.lock().unwrap().advance_bytes(&sanitized);
            output_history
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&sanitized).to_string());
        }
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| format!("clone reader failed: {err}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| format!("take writer failed: {err}"))?;
        let (input_tx, input_rx) = std_mpsc::sync_channel::<Vec<u8>>(INPUT_QUEUE_CAPACITY);
        spawn_reader_loop(
            Arc::clone(&terminal),
            Arc::clone(&output_history),
            spawn.clone(),
            events.clone(),
            reader,
        );
        spawn_writer_loop(spawn.clone(), events.clone(), writer, input_rx);
        spawn_waiter_loop(spawn, events, Arc::clone(&child));
        log::debug!(
            "bootstrapped terminal instance runtime pid={:?}",
            process_state.process_id
        );
        Ok(Self {
            terminal,
            output_history,
            master: Mutex::new(pair.master),
            child,
            input_tx: Mutex::new(Some(input_tx)),
        })
    }

    pub fn process_state(
        &self,
        spawn: &TerminalInstanceRuntimeSpawn,
    ) -> TerminalInstanceProcessState {
        TerminalInstanceProcessState {
            process_id: self.child.lock().ok().and_then(|guard| guard.process_id()),
            command_label: command_label(&spawn.command),
        }
    }

    #[cfg(test)]
    pub fn screen(&self) -> ScreenSnapshot {
        self.terminal.lock().unwrap().screen()
    }

    pub fn replay_output(&self) -> String {
        self.output_history.lock().unwrap().join("")
    }

    pub fn resize(&self, size: TerminalSize) -> Result<(), String> {
        self.master
            .lock()
            .unwrap()
            .resize(to_pty_size(size))
            .map_err(|err| format!("resize pty failed: {err}"))?;
        self.terminal.lock().unwrap().resize(size);
        Ok(())
    }

    pub fn write_input(&self, data: impl AsRef<[u8]>) -> Result<(), String> {
        let Some(tx) = self.input_tx.lock().unwrap().as_ref().cloned() else {
            return Err("terminal instance runtime input channel closed".into());
        };
        tx.send(data.as_ref().to_vec())
            .map_err(|_| "terminal instance runtime input channel disconnected".to_string())
    }

    pub fn kill(&self) {
        self.input_tx.lock().unwrap().take();
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

impl Drop for TerminalInstanceRuntime {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
#[path = "leaf_runtime_tests.rs"]
mod tests;
