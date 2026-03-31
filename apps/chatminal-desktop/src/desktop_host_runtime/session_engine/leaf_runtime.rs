use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::io::Write;

use chatminal_terminal_core::color::ColorPalette;
#[cfg(test)]
use chatminal_terminal_core::ScreenSnapshot;
use chatminal_terminal_core::{
    Terminal as CoreTerminal, TerminalConfiguration as CoreTerminalConfiguration, TerminalSize,
};
use engine_term::{
    KeyCode as IoKeyCode, KeyModifiers as IoKeyModifiers, MouseEvent as IoMouseEvent,
    Terminal as IoTerminal, TerminalSize as IoTerminalSize,
};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty};

use super::leaf_runtime_command::prepare_leaf_command;
use super::leaf_runtime_threads::{
    command_label, sanitize_zsh_prompt_spacer, spawn_reader_waiter_loop, to_pty_size,
};
use super::output_history::OutputHistory;
use super::{RuntimeId, TerminalInstanceId, TerminalInstanceProcessState};

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
        session_id: Arc<str>,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        chunk: String,
    },
    Exited {
        session_id: Arc<str>,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        exit_code: Option<i32>,
    },
    Error {
        session_id: Arc<str>,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        message: String,
    },
}

#[derive(Debug)]
struct LeafTerminalConfig;

impl CoreTerminalConfiguration for LeafTerminalConfig {
    fn scrollback_size(&self) -> usize {
        10_000
    }
    fn color_palette(&self) -> ColorPalette {
        ColorPalette
    }
}

#[derive(Clone)]
struct SharedPtyWriter {
    inner: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
}

impl SharedPtyWriter {
    fn new(inner: Arc<Mutex<Option<Box<dyn Write + Send>>>>) -> Self {
        Self { inner }
    }
}

impl Write for SharedPtyWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.inner.lock().unwrap();
        let Some(writer) = guard.as_mut() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "pty writer closed",
            ));
        };
        writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self.inner.lock().unwrap();
        let Some(writer) = guard.as_mut() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "pty writer closed",
            ));
        };
        writer.flush()
    }
}

pub struct TerminalInstanceRuntime {
    terminal: Arc<Mutex<CoreTerminal>>,
    io_terminal: Arc<Mutex<IoTerminal>>,
    output_history: Arc<Mutex<OutputHistory>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
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
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| format!("take writer failed: {err}"))?;
        let writer = Arc::new(Mutex::new(Some(writer)));
        let terminal = Arc::new(Mutex::new(CoreTerminal::new(
            spawn.size,
            Arc::new(LeafTerminalConfig),
            "Chatminal",
            env!("CARGO_PKG_VERSION"),
            Box::new(std::io::sink()),
        )));
        let io_terminal = Arc::new(Mutex::new(IoTerminal::new(
            IoTerminalSize {
                rows: spawn.size.rows,
                cols: spawn.size.cols,
                pixel_width: spawn.size.pixel_width,
                pixel_height: spawn.size.pixel_height,
                dpi: spawn.size.dpi,
            },
            Arc::new(config::TermConfig::new()),
            "Chatminal",
            config::engine_version(),
            Box::new(SharedPtyWriter::new(Arc::clone(&writer))),
        )));
        let output_history = Arc::new(Mutex::new(OutputHistory::new()));
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
        spawn_reader_waiter_loop(
            Arc::clone(&terminal),
            Arc::clone(&io_terminal),
            Arc::clone(&output_history),
            spawn.clone(),
            events,
            reader,
            Arc::clone(&child),
        );
        log::debug!(
            "bootstrapped terminal instance runtime pid={:?}",
            process_state.process_id
        );
        Ok(Self {
            terminal,
            io_terminal,
            output_history,
            master: Mutex::new(pair.master),
            child,
            writer,
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
        self.output_history.lock().unwrap().replay()
    }

    pub fn resize(&self, size: TerminalSize) -> Result<(), String> {
        self.master
            .lock()
            .unwrap()
            .resize(to_pty_size(size))
            .map_err(|err| format!("resize pty failed: {err}"))?;
        self.terminal.lock().unwrap().resize(size);
        self.io_terminal.lock().unwrap().resize(IoTerminalSize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: size.pixel_width,
            pixel_height: size.pixel_height,
            dpi: size.dpi,
        });
        Ok(())
    }

    pub fn write_input(&self, data: impl AsRef<[u8]>) -> Result<(), String> {
        let mut writer_guard = self.writer.lock().unwrap();
        let Some(writer) = writer_guard.as_mut() else {
            return Err("terminal instance runtime input channel closed".into());
        };
        writer
            .write_all(data.as_ref())
            .and_then(|_| writer.flush())
            .map_err(|err| format!("terminal instance runtime input write failed: {err}"))
    }

    pub fn key_down(&self, key: IoKeyCode, mods: IoKeyModifiers) -> Result<(), String> {
        self.io_terminal
            .lock()
            .unwrap()
            .key_down(key, mods)
            .map_err(|err| format!("terminal instance runtime key_down failed: {err:#}"))
    }

    pub fn key_up(&self, key: IoKeyCode, mods: IoKeyModifiers) -> Result<(), String> {
        self.io_terminal
            .lock()
            .unwrap()
            .key_up(key, mods)
            .map_err(|err| format!("terminal instance runtime key_up failed: {err:#}"))
    }

    pub fn send_paste(&self, text: &str) -> Result<(), String> {
        self.io_terminal
            .lock()
            .unwrap()
            .send_paste(text)
            .map_err(|err| format!("terminal instance runtime paste failed: {err:#}"))
    }

    pub fn mouse_event(&self, event: IoMouseEvent) -> Result<(), String> {
        self.io_terminal
            .lock()
            .unwrap()
            .mouse_event(event)
            .map_err(|err| format!("terminal instance runtime mouse_event failed: {err:#}"))
    }

    pub fn kill(&self) {
        self.writer.lock().unwrap().take();
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
