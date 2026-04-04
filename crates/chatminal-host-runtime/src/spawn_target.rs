//! A SpawnTarget is the execution backend used by the mux to create panes/tabs.
//! The active desktop product path installs a single primary backend.

use crate::localpane::LocalPane;
use crate::pane::{alloc_pane_id, pane_id_for_pane, Pane, PaneId};
use crate::tab::{SplitRequest, Tab};
use crate::{
    active_identity, build_runtime_entry_tab, current_host_runtime_config,
    register_attached_runtime_entry_tab, register_pane_with_default_side_effects_and_io_hooks,
    remove_tab_by_id, resolve_pane_id, tab_by_id, LocalPaneHooks, PtyIoHooks,
};
use anyhow::{Context, Error};
use async_trait::async_trait;
use chatminal_runtime::{RuntimeId, SessionTerminalHandle};
use config::keyassignment::SpawnCommand;
use config::{ConfigHandle, ExecTarget, ExitBehavior, SerialTarget};
use downcast_rs::{impl_downcast, Downcast};
use engine_term::Alert;
use engine_term::TerminalSize;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, ExitStatus, MasterPty, PtySize, PtySystem};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum SplitSource {
    Spawn {
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    },
    MoveTerminal(SessionTerminalHandle),
}

#[async_trait(?Send)]
pub trait SpawnTarget: Downcast + Send + Sync {
    /// Spawn a new command within this target.
    async fn spawn(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Arc<Tab>> {
        let pane = self
            .spawn_pane(size, command, command_dir)
            .await
            .context("spawn")?;

        let tab = build_runtime_entry_tab(&pane, size, None);
        register_attached_runtime_entry_tab(&tab)?;

        Ok(tab)
    }

    #[deprecated(
        note = "Use session-native split; engine split retained for runtime compatibility"
    )]
    #[allow(deprecated)]
    async fn split_pane(
        &self,
        source: SplitSource,
        runtime_id: RuntimeId,
        terminal_handle: SessionTerminalHandle,
        split_request: SplitRequest,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        let tab = usize::try_from(runtime_id.as_u64())
            .map_err(|_| anyhow::anyhow!("Invalid runtime id {}", runtime_id.as_u64()))?;
        let pane_id = usize::try_from(terminal_handle.as_u64())
            .map_err(|_| anyhow::anyhow!("invalid terminal handle {}", terminal_handle.as_u64()))?;
        let tab = match tab_by_id(tab) {
            Some(t) => t,
            None => anyhow::bail!("Invalid tab id {}", tab),
        };

        let pane_index = match tab
            .iter_panes_ignoring_zoom()
            .iter()
            .find(|p| pane_id_for_pane(p.pane.as_ref()) == pane_id)
        {
            Some(p) => p.index,
            None => anyhow::bail!("invalid pane id {}", pane_id),
        };

        let split_size = match tab.compute_split_size(pane_index, split_request) {
            Some(s) => s,
            None => anyhow::bail!("invalid pane index {}", pane_index),
        };

        let pane = match source {
            SplitSource::Spawn {
                command,
                command_dir,
            } => {
                self.spawn_pane(split_size.second, command, command_dir)
                    .await?
            }
            SplitSource::MoveTerminal(src_terminal_handle) => {
                let src_pane_id = usize::try_from(src_terminal_handle.as_u64()).map_err(|_| {
                    anyhow::anyhow!("invalid terminal handle {}", src_terminal_handle.as_u64())
                })?;
                let src_tab = resolve_pane_id(src_pane_id)
                    .ok_or_else(|| anyhow::anyhow!("pane {} not found", src_pane_id))?;
                let src_tab = match tab_by_id(src_tab) {
                    Some(t) => t,
                    None => anyhow::bail!("Invalid tab id {}", src_tab),
                };

                let pane = src_tab.remove_pane(src_pane_id).ok_or_else(|| {
                    anyhow::anyhow!("pane {} not found in its containing tab!?", src_pane_id)
                })?;

                if src_tab.is_dead() {
                    remove_tab_by_id(src_tab.tab_id());
                }

                pane
            }
        };

        // pane_index may have changed if src_pane was also in the same tab
        let final_pane_index = match tab
            .iter_panes_ignoring_zoom()
            .iter()
            .find(|p| pane_id_for_pane(p.pane.as_ref()) == pane_id)
        {
            Some(p) => p.index,
            None => anyhow::bail!("invalid pane id {}", pane_id),
        };

        tab.split_and_insert(final_pane_index, split_request, Arc::clone(&pane))?;
        Ok(pane)
    }

    async fn spawn_pane(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>>;

    /// Returns the name of the target.
    /// Should be a short identifier.
    fn spawn_target_name(&self) -> &str;
}
impl_downcast!(SpawnTarget);

pub struct LocalSpawnTarget {
    pty_system: Mutex<Box<dyn PtySystem + Send>>,
    name: String,
    hooks: LocalSpawnHooks,
}

#[derive(Clone)]
pub struct LocalSpawnHooks {
    localpane_hooks: LocalPaneHooks,
    pty_io_hooks: PtyIoHooks,
}

impl LocalSpawnHooks {
    pub fn noop() -> Self {
        Self {
            localpane_hooks: LocalPaneHooks::default(),
            pty_io_hooks: PtyIoHooks::noop(),
        }
    }

    // Product default for host-runtime-owned spawn/localpane side effects.
    pub fn host_default() -> Self {
        Self {
            localpane_hooks: LocalPaneHooks::host_default(),
            pty_io_hooks: PtyIoHooks::host_default(),
        }
    }

    // Explicit compat seam for callers/tests.
    pub fn compat_default() -> Self {
        Self {
            localpane_hooks: LocalPaneHooks::compat_default(),
            pty_io_hooks: PtyIoHooks::compat_default(),
        }
    }

    pub(crate) fn localpane_hooks(&self) -> LocalPaneHooks {
        self.localpane_hooks.clone()
    }

    pub(crate) fn pty_io_hooks(&self) -> PtyIoHooks {
        self.pty_io_hooks.clone()
    }

    pub fn with_output_callback(
        mut self,
        on_output: Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>,
    ) -> Self {
        self.pty_io_hooks.set_output(Some(on_output));
        self
    }

    pub fn with_cleanup_callback(
        mut self,
        on_cleanup: Arc<dyn Fn(SessionTerminalHandle, Option<ExitBehavior>) + Send + Sync>,
    ) -> Self {
        self.pty_io_hooks
            .set_cleanup(Some(Arc::new(move |pane_id, exit_behavior| {
                on_cleanup(SessionTerminalHandle::new(pane_id as u64), exit_behavior);
            })));
        self
    }

    pub fn with_inline_error_output_callback(
        mut self,
        on_inline_error_output: Arc<dyn Fn(SessionTerminalHandle, String) + Send + Sync>,
    ) -> Self {
        self.pty_io_hooks
            .set_inline_error_output(Some(Arc::new(move |pane_id, message| {
                on_inline_error_output(SessionTerminalHandle::new(pane_id as u64), message);
            })));
        self
    }

    pub fn with_input_callback(mut self, on_input: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.localpane_hooks.set_input(on_input);
        self
    }

    pub fn with_inline_output_callback(
        mut self,
        on_inline_output: Arc<dyn Fn(SessionTerminalHandle, String) + Send + Sync>,
    ) -> Self {
        self.localpane_hooks
            .set_inline_output(Arc::new(move |pane_id, message| {
                on_inline_output(SessionTerminalHandle::new(pane_id as u64), message);
            }));
        self
    }

    pub fn with_alert_callback(
        mut self,
        on_alert: Arc<dyn Fn(SessionTerminalHandle, Alert) + Send + Sync>,
    ) -> Self {
        self.localpane_hooks
            .set_alert(Arc::new(move |pane_id, alert| {
                on_alert(SessionTerminalHandle::new(pane_id as u64), alert);
            }));
        self
    }

    pub fn with_child_exit_cleanup_callback(
        mut self,
        on_child_exit_cleanup: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        self.localpane_hooks
            .set_child_exit_cleanup(Arc::new(move |_, _| on_child_exit_cleanup()));
        self
    }
}

impl Default for LocalSpawnHooks {
    fn default() -> Self {
        Self::noop()
    }
}

impl LocalSpawnTarget {
    pub fn new(name: &str) -> Result<Self, Error> {
        Ok(Self::with_pty_system_and_hooks(
            name,
            native_pty_system(),
            LocalSpawnHooks::default(),
        ))
    }

    pub fn new_with_hooks(name: &str, hooks: LocalSpawnHooks) -> Result<Self, Error> {
        Ok(Self::with_pty_system_and_hooks(
            name,
            native_pty_system(),
            hooks,
        ))
    }

    fn resolve_exec_target_config(&self, config: &ConfigHandle) -> Option<ExecTarget> {
        config
            .exec_targets
            .iter()
            .find(|ed| ed.name == self.name)
            .cloned()
    }

    pub fn with_pty_system(name: &str, pty_system: Box<dyn PtySystem + Send>) -> Self {
        Self::with_pty_system_and_hooks(name, pty_system, LocalSpawnHooks::default())
    }

    pub fn with_pty_system_and_hooks(
        name: &str,
        pty_system: Box<dyn PtySystem + Send>,
        hooks: LocalSpawnHooks,
    ) -> Self {
        Self {
            pty_system: Mutex::new(pty_system),
            name: name.to_string(),
            hooks,
        }
    }

    pub fn new_exec_target(exec_target: ExecTarget) -> anyhow::Result<Self> {
        Self::new(&exec_target.name)
    }

    pub fn new_serial_target(serial_target: SerialTarget) -> anyhow::Result<Self> {
        Self::new_serial_target_with_hooks(serial_target, LocalSpawnHooks::default())
    }

    pub fn new_serial_target_with_hooks(
        serial_target: SerialTarget,
        hooks: LocalSpawnHooks,
    ) -> anyhow::Result<Self> {
        let port = serial_target.port.as_ref().unwrap_or(&serial_target.name);
        let mut serial = portable_pty::serial::SerialTty::new(&port);
        if let Some(baud) = serial_target.baud {
            serial.set_baud_rate(baud as u32);
        }
        let pty_system = Box::new(serial);
        Ok(Self::with_pty_system_and_hooks(
            &serial_target.name,
            pty_system,
            hooks,
        ))
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

    async fn fixup_command(
        &self,
        cmd: &mut CommandBuilder,
        config: &ConfigHandle,
    ) -> anyhow::Result<RuntimeEnvironmentPolicy> {
        if let Some(ed) = self.resolve_exec_target_config(config) {
            let mut args = vec![];
            let mut set_environment_variables = HashMap::new();
            for arg in cmd.get_argv() {
                args.push(
                    arg.to_str()
                        .ok_or_else(|| anyhow::anyhow!("command argument is not utf8"))?
                        .to_string(),
                );
            }
            for (k, v) in cmd.iter_full_env_as_str() {
                set_environment_variables.insert(k.to_string(), v.to_string());
            }
            let cwd = match cmd.get_cwd() {
                Some(cwd) => Some(PathBuf::from(cwd)),
                None => None,
            };
            let spawn_command = SpawnCommand {
                label: None,
                args: if args.is_empty() { None } else { Some(args) },
                set_environment_variables,
                cwd,
                position: None,
            };

            let spawn_command = config::with_lua_config_on_main_thread(|lua| async {
                let lua = lua.ok_or_else(|| anyhow::anyhow!("missing lua context"))?;
                let value = config::lua::emit_async_callback(
                    &*lua,
                    (ed.fixup_command.clone(), (spawn_command.clone())),
                )
                .await?;
                let cmd: SpawnCommand =
                    luahelper::from_lua_value_dynamic(value).with_context(|| {
                        format!(
                            "interpreting SpawnCommand result from ExecTarget {}",
                            ed.name
                        )
                    })?;
                Ok(cmd)
            })
            .await
            .with_context(|| format!("calling ExecTarget {} function", ed.name))?;

            // Reinterpret the SpawnCommand into the builder

            cmd.get_argv_mut().clear();
            if let Some(args) = &spawn_command.args {
                for arg in args {
                    cmd.get_argv_mut().push(arg.into());
                }
            }
            cmd.env_clear();
            for (k, v) in &spawn_command.set_environment_variables {
                cmd.env(k, v);
            }
            cmd.clear_cwd();
            if let Some(cwd) = &spawn_command.cwd {
                cmd.cwd(cwd);
            }
        } else if Path::new("/.flatpak-info").exists() {
            // We're running inside a flatpak sandbox.
            // Run the command outside the sandbox via flatpak-spawn
            let mut args = vec![
                "flatpak-spawn".to_string(),
                "--host".to_string(),
                "--watch-bus".to_string(),
            ];
            if let Some(cwd) = cmd.get_cwd() {
                args.push(format!("--directory={}", Path::new(cwd).display()));
            }

            let is_default_prog = cmd.is_default_prog();

            // Note: CHATMINAL_UNIX_SOCKET, CHATMINAL_CONFIG_(FILE|DIR) and other env
            // vars are not included in this.
            // We can't include them: their paths are only meaningful in the sandbox
            // and cannot be reasonably accessed from outside it in the shell.
            for (k, v) in cmd.iter_extra_env_as_str() {
                args.push(format!("--env={k}={v}"));
            }

            for arg in cmd.get_argv() {
                args.push(
                    arg.to_str()
                        .ok_or_else(|| anyhow::anyhow!("command argument is not utf8"))?
                        .to_string(),
                );
            }

            if is_default_prog {
                // We can't read $SHELL from inside the sandbox, so ask the host.
                let output = std::process::Command::new("flatpak-spawn")
                    .args(["--host", "sh", "-c", "echo $SHELL"])
                    .output()?;
                let shell = String::from_utf8_lossy(&output.stdout);

                args.push(shell.trim().to_string());
                // Assume we can pass `-l` for a login shell
                args.push("-l".to_string());
            }

            // Avoid setting up the controlling tty as that is not compatible
            // with flatpak:
            // <https://github.com/flatpak/flatpak/issues/3697>
            // <https://github.com/flatpak/flatpak/issues/3285>
            cmd.set_controlling_tty(false);

            // Re-apply to the builder
            cmd.get_argv_mut().clear();
            for arg in args {
                cmd.get_argv_mut().push(arg.into());
            }
            cmd.clear_cwd();
            log::trace!("made: {cmd:#?}");
            return Ok(RuntimeEnvironmentPolicy::PaneOnly);
        } else if let Some(dir) = cmd.get_cwd() {
            // I'm not normally a fan of existence checking, but not checking here
            // can be painful; in the case where a tab is local but has connected
            // to a remote system and that remote has used OSC 7 to set a path
            // that doesn't exist on the local system, process spawning can fail.
            // Another situation is `sudo -i` has the pane with set to a cwd
            // that is not accessible to the user.
            if let Err(err) = Path::new(&dir).read_dir() {
                log::warn!(
                    "Directory {:?} is not readable and will not be \
                     used for the command we are spawning: {:#}",
                    dir,
                    err
                );
                cmd.clear_cwd();
            }
        }
        Ok(RuntimeEnvironmentPolicy::Full)
    }

    async fn build_command(
        &self,
        config: &ConfigHandle,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        pane_id: PaneId,
    ) -> anyhow::Result<CommandBuilder> {
        let unix_socket = std::env::var("CHATMINAL_UNIX_SOCKET").ok();
        let ssh_auth_sock = preferred_ssh_auth_sock();
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
            cmd.cwd(dir);
        }
        let runtime_environment_policy = self.fixup_command(&mut cmd, &config).await?;
        apply_runtime_environment(
            &mut cmd,
            pane_id,
            unix_socket.as_deref(),
            ssh_auth_sock.as_deref(),
            runtime_environment_policy,
        );
        Ok(cmd)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeEnvironmentPolicy {
    Full,
    PaneOnly,
}

fn preferred_ssh_auth_sock() -> Option<String> {
    active_identity()
        .and_then(|identity| identity.ssh_auth_sock.clone())
        .or_else(|| std::env::var("SSH_AUTH_SOCK").ok())
}

fn apply_runtime_environment(
    cmd: &mut CommandBuilder,
    pane_id: PaneId,
    unix_socket: Option<&str>,
    ssh_auth_sock: Option<&str>,
    policy: RuntimeEnvironmentPolicy,
) {
    cmd.env("CHATMINAL_PANE", pane_id.to_string());
    if policy == RuntimeEnvironmentPolicy::Full {
        if let Some(unix_socket) = unix_socket {
            cmd.env("CHATMINAL_UNIX_SOCKET", unix_socket);
        }
        if let Some(ssh_auth_sock) = ssh_auth_sock {
            cmd.env("SSH_AUTH_SOCK", ssh_auth_sock);
        }
    }
}

/// Allows sharing the writer between the Pane and the Terminal.
/// This could potentially be eliminated in the future if we can
/// teach the Pane impl to reference the writer in the Termninal,
/// but the Pane trait returns a RefMut and that makes it a bit
/// awkward at the moment.
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

/// Wraps the underlying pty; we use this as a marker for when
/// the spawn attempt failed in order to hold the pane open
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

/// A fake child process for the case where the spawn attempt
/// failed. It reports as immediately terminated.
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

#[async_trait(?Send)]
impl SpawnTarget for LocalSpawnTarget {
    async fn spawn_pane(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        let pane_id = alloc_pane_id();
        let config = current_host_runtime_config();
        let cmd = self
            .build_command(&config, command, command_dir, pane_id)
            .await
            .context("build_command")?;
        let pair = self
            .pty_system
            .lock()
            .openpty(crate::terminal_size_to_pty_size(size)?)?;

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

        let mut terminal = engine_term::Terminal::new(
            size,
            std::sync::Arc::new(config::TermConfig::with_config(config.clone())),
            "Chatminal",
            config::engine_version(),
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
                // Show the error to the user in the new pane
                write!(writer, "{err:#}").ok();

                // and return a dummy pane that has exited
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

        register_pane_with_default_side_effects_and_io_hooks(&pane, self.hooks.pty_io_hooks())?;

        Ok(pane)
    }
    fn spawn_target_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_runtime_environment, preferred_ssh_auth_sock, RuntimeEnvironmentPolicy};
    use crate::{initialize_host_runtime, shutdown_host_runtime, ClientId};
    use portable_pty::CommandBuilder;
    use std::ffi::OsStr;
    use std::sync::Arc;

    #[test]
    fn runtime_environment_is_reapplied_after_fixup_paths() {
        let mut cmd = CommandBuilder::new("/bin/sh");
        cmd.env_clear();

        apply_runtime_environment(
            &mut cmd,
            42,
            Some("/tmp/chatminal.sock"),
            Some("/tmp/ssh-agent.sock"),
            RuntimeEnvironmentPolicy::Full,
        );

        assert_eq!(
            cmd.get_env("CHATMINAL_UNIX_SOCKET"),
            Some(OsStr::new("/tmp/chatminal.sock"))
        );
        assert_eq!(cmd.get_env("CHATMINAL_PANE"), Some(OsStr::new("42")));
        assert_eq!(
            cmd.get_env("SSH_AUTH_SOCK"),
            Some(OsStr::new("/tmp/ssh-agent.sock"))
        );
    }

    #[test]
    fn pane_only_runtime_environment_skips_socket_paths() {
        let mut cmd = CommandBuilder::new("/bin/sh");
        cmd.env_clear();

        apply_runtime_environment(
            &mut cmd,
            42,
            Some("/tmp/chatminal.sock"),
            Some("/tmp/ssh-agent.sock"),
            RuntimeEnvironmentPolicy::PaneOnly,
        );

        assert_eq!(cmd.get_env("CHATMINAL_PANE"), Some(OsStr::new("42")));
        assert!(cmd.get_env("CHATMINAL_UNIX_SOCKET").is_none());
        assert!(cmd.get_env("SSH_AUTH_SOCK").is_none());
    }

    #[test]
    fn preferred_ssh_auth_sock_prefers_active_identity_snapshot() {
        let _guard = crate::HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        shutdown_host_runtime();
        let mux = initialize_host_runtime(None).expect("init host runtime");
        let previous = std::env::var_os("SSH_AUTH_SOCK");
        std::env::set_var("SSH_AUTH_SOCK", "/tmp/env-agent.sock");

        let client = Arc::new(ClientId {
            hostname: "localhost".to_string(),
            username: "khoa".to_string(),
            pid: 1,
            epoch: 2,
            id: 3,
            ssh_auth_sock: Some("/tmp/identity-agent.sock".to_string()),
        });
        mux.register_client(Arc::clone(&client));
        mux.replace_identity(Some(client));

        assert_eq!(
            preferred_ssh_auth_sock().as_deref(),
            Some("/tmp/identity-agent.sock")
        );

        mux.replace_identity(None);
        shutdown_host_runtime();
        match previous {
            Some(value) => std::env::set_var("SSH_AUTH_SOCK", value),
            None => std::env::remove_var("SSH_AUTH_SOCK"),
        }
    }
}
