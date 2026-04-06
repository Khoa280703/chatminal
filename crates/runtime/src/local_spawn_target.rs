//! Minimal local PTY spawn target for desktop use cases (local shell, serial targets).
//! Does not depend on host_runtime global state.

use crate::localpane::LocalPane;
use crate::localpane_hooks::LocalPaneHooks;
use crate::pane::{Pane, PaneId, alloc_pane_id};
use crate::pty_io::{PtyIoHooks, start_pane_pty_reader};
use anyhow::Context;
use config::{ConfigHandle, SerialTarget};
use parking_lot::Mutex;
use portable_pty::{CommandBuilder, ExitStatus, MasterPty, PtySize, PtySystem, native_pty_system};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use terminal_emulator::TerminalSize;

// ---------------------------------------------------------------------------
// LocalSpawnHooks
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct LocalSpawnHooks {
    pub(crate) localpane_hooks: LocalPaneHooks,
    pub(crate) pty_io_hooks: PtyIoHooks,
}

impl LocalSpawnHooks {
    pub fn noop() -> Self {
        Self {
            localpane_hooks: LocalPaneHooks::noop(),
            pty_io_hooks: PtyIoHooks::noop(),
        }
    }

    pub fn localpane_hooks(&self) -> LocalPaneHooks {
        self.localpane_hooks.clone()
    }

    pub(crate) fn pty_io_hooks(&self) -> PtyIoHooks {
        self.pty_io_hooks.clone()
    }
}

impl Default for LocalSpawnHooks {
    fn default() -> Self {
        Self::noop()
    }
}

// ---------------------------------------------------------------------------
// LocalSpawnTarget
// ---------------------------------------------------------------------------

pub struct LocalSpawnTarget {
    pty_system: Mutex<Box<dyn PtySystem + Send>>,
    name: String,
    hooks: LocalSpawnHooks,
}

impl LocalSpawnTarget {
    pub fn new_with_hooks(name: &str, hooks: LocalSpawnHooks) -> anyhow::Result<Self> {
        Ok(Self {
            pty_system: Mutex::new(native_pty_system()),
            name: name.to_string(),
            hooks,
        })
    }

    pub fn new_serial_target_with_hooks(
        serial_target: SerialTarget,
        hooks: LocalSpawnHooks,
    ) -> anyhow::Result<Self> {
        let port = serial_target.port.as_ref().unwrap_or(&serial_target.name);
        let mut serial = portable_pty::serial::SerialTty::new(port);
        if let Some(baud) = serial_target.baud {
            serial.set_baud_rate(baud as u32);
        }
        Ok(Self {
            pty_system: Mutex::new(Box::new(serial)),
            name: serial_target.name.clone(),
            hooks,
        })
    }

    #[cfg(unix)]
    fn is_conpty(&self) -> bool {
        false
    }

    #[cfg(windows)]
    fn is_conpty(&self) -> bool {
        let pty_system = self.pty_system.lock();
        let pty_system: &dyn PtySystem = &**pty_system;
        pty_system
            .downcast_ref::<portable_pty::win::conpty::ConPtySystem>()
            .is_some()
    }

    pub async fn spawn_pane(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        let pane_id = alloc_pane_id();
        let config = config::configuration();
        let cmd = self
            .build_command(&config, command, command_dir, pane_id)
            .await
            .context("build_command")?;

        let pair = self
            .pty_system
            .lock()
            .openpty(terminal_size_to_pty_size(size)?)?;

        let command_line = cmd
            .as_unix_command_line()
            .unwrap_or_else(|err| format!("error rendering command line: {:?}", err));
        let command_description = format!(
            "\"{}\" on target \"{}\"",
            if command_line.is_empty() {
                cmd.get_shell()
            } else {
                command_line
            },
            self.name
        );
        let child_result = pair.slave.spawn_command(cmd);
        let mut writer = WriterWrapper::new(pair.master.take_writer()?);

        let mut terminal = terminal_emulator::Terminal::new(
            size,
            std::sync::Arc::new(config::TermConfig::with_config(config.clone())),
            "Chatminal",
            config::terminal_version(),
            Box::new(writer.clone()),
        );
        if self.is_conpty() {
            terminal.enable_conpty_quirks();
        }

        let pane: Arc<dyn Pane> = match child_result {
            Ok(child) => Arc::new(LocalPane::new_with_hooks(
                pane_id,
                terminal,
                child,
                pair.master,
                Box::new(writer),
                command_description,
                config.clone(),
                self.hooks.localpane_hooks(),
            )),
            Err(err) => {
                write!(writer, "{err:#}").ok();
                Arc::new(LocalPane::new_with_hooks(
                    pane_id,
                    terminal,
                    Box::new(FailedProcessSpawn {}),
                    Box::new(FailedSpawnPty {
                        inner: Mutex::new(pair.master),
                    }),
                    Box::new(writer),
                    command_description,
                    config.clone(),
                    self.hooks.localpane_hooks(),
                ))
            }
        };

        start_pane_pty_reader(&pane, self.hooks.pty_io_hooks())?;

        Ok(pane)
    }

    async fn build_command(
        &self,
        config: &ConfigHandle,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        pane_id: PaneId,
    ) -> anyhow::Result<CommandBuilder> {
        let unix_socket = std::env::var("CHATMINAL_UNIX_SOCKET").ok();
        let ssh_auth_sock = std::env::var("SSH_AUTH_SOCK").ok();
        let mut cmd = match command {
            Some(mut cmd) => {
                config.apply_cmd_defaults(
                    &mut cmd,
                    config.default_prog.as_ref(),
                    config.default_cwd.as_ref(),
                );
                cmd
            }
            None => config.build_prog(
                None,
                config.default_prog.as_ref(),
                config.default_cwd.as_ref(),
            )?,
        };
        if let Some(dir) = command_dir {
            if Path::new(&dir).read_dir().is_ok() {
                cmd.cwd(dir);
            }
        }
        apply_runtime_environment(
            &mut cmd,
            pane_id,
            unix_socket.as_deref(),
            ssh_auth_sock.as_deref(),
        );
        Ok(cmd)
    }
}

fn terminal_size_to_pty_size(size: TerminalSize) -> anyhow::Result<PtySize> {
    Ok(PtySize {
        rows: size.rows.try_into()?,
        cols: size.cols.try_into()?,
        pixel_height: size.pixel_height.try_into()?,
        pixel_width: size.pixel_width.try_into()?,
    })
}

fn apply_runtime_environment(
    cmd: &mut CommandBuilder,
    pane_id: PaneId,
    unix_socket: Option<&str>,
    ssh_auth_sock: Option<&str>,
) {
    cmd.env("CHATMINAL_PANE", pane_id.to_string());
    if let Some(unix_socket) = unix_socket {
        cmd.env("CHATMINAL_UNIX_SOCKET", unix_socket);
    }
    if let Some(ssh_auth_sock) = ssh_auth_sock {
        cmd.env("SSH_AUTH_SOCK", ssh_auth_sock);
    }
}

// ---------------------------------------------------------------------------
// Helpers (adapted from chatminal-host-runtime spawn_target.rs)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct WriterWrapper {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl WriterWrapper {
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
        }
    }
}

impl std::io::Write for WriterWrapper {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.lock().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.lock().flush()
    }
}

pub(crate) struct FailedSpawnPty {
    inner: Mutex<Box<dyn MasterPty>>,
}

impl portable_pty::MasterPty for FailedSpawnPty {
    fn resize(&self, new_size: PtySize) -> anyhow::Result<()> {
        self.inner.lock().resize(new_size)
    }
    fn get_size(&self) -> anyhow::Result<PtySize> {
        self.inner.lock().get_size()
    }
    fn try_clone_reader(&self) -> anyhow::Result<Box<dyn std::io::Read + Send + 'static>> {
        self.inner.lock().try_clone_reader()
    }
    fn take_writer(&self) -> anyhow::Result<Box<dyn std::io::Write + Send + 'static>> {
        self.inner.lock().take_writer()
    }

    #[cfg(unix)]
    fn process_group_leader(&self) -> Option<i32> {
        None
    }

    #[cfg(unix)]
    fn as_raw_fd(&self) -> Option<std::os::fd::RawFd> {
        None
    }

    #[cfg(unix)]
    fn tty_name(&self) -> Option<std::path::PathBuf> {
        None
    }
}

#[derive(Debug)]
pub(crate) struct FailedProcessSpawn {}

impl portable_pty::Child for FailedProcessSpawn {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        Ok(Some(ExitStatus::with_exit_code(1)))
    }
    fn wait(&mut self) -> std::io::Result<ExitStatus> {
        Ok(ExitStatus::with_exit_code(1))
    }
    fn process_id(&self) -> Option<u32> {
        None
    }
    #[cfg(windows)]
    fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
        None
    }
}

impl portable_pty::ChildKiller for FailedProcessSpawn {
    fn kill(&mut self) -> std::io::Result<()> {
        Ok(())
    }
    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(FailedProcessSpawn {})
    }
}
