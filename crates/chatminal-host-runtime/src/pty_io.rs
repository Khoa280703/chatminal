use crate::localpane::LocalPane;
use crate::pane::{pane_id_for_pane, Pane, PaneId};
use crate::{
    current_host_exit_behavior, current_host_output_parser_config, notify_runtime_any_thread,
    prune_dead_windows_on_main_thread, remove_pane_on_main_thread, terminal_by_id,
    HostRuntimeEvent,
};
use anyhow::Context;
use chatminal_runtime::SessionTerminalHandle;
use config::ExitBehavior;
use filedescriptor::{poll, pollfd, socketpair, AsRawSocketDescriptor, FileDescriptor, POLLIN};
#[cfg(unix)]
use libc::{c_int, SOL_SOCKET, SO_RCVBUF, SO_SNDBUF};
use metrics::histogram;
use std::convert::TryFrom;
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::raw::c_int;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::{Duration, Instant};
use termwiz::escape::csi::{DecPrivateMode, DecPrivateModeCode, Device, Mode, Sgr};
use termwiz::escape::{Action, CSI};
#[cfg(windows)]
use winapi::um::winsock2::{SOL_SOCKET, SO_RCVBUF, SO_SNDBUF};

const BUFSIZE: usize = 256 * 1024;

#[derive(Clone, Copy)]
struct OutputParserConfigSnapshot {
    buffer_size: usize,
    coalesce_delay_ms: u64,
}

#[derive(Clone, Copy)]
struct PtyIoConfigSnapshot {
    parser: OutputParserConfigSnapshot,
}

#[derive(Clone)]
struct PtyIoDispatcher {
    on_output: Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>,
    on_cleanup: Arc<dyn Fn(PaneId, Option<ExitBehavior>) + Send + Sync>,
    on_inline_error_output: Arc<dyn Fn(PaneId, String) + Send + Sync>,
}

type PtyCleanupHook = Arc<dyn Fn(PaneId, Option<ExitBehavior>, ExitBehavior) + Send + Sync>;
type PtyInlineErrorOutputHook =
    Arc<dyn Fn(PaneId, String, Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>) + Send + Sync>;

#[derive(Clone)]
pub(crate) struct PtyIoHooks {
    on_output: Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>,
    on_cleanup: PtyCleanupHook,
    on_inline_error_output: PtyInlineErrorOutputHook,
}

impl PtyIoHooks {
    pub(crate) fn noop() -> Self {
        Self {
            on_output: Arc::new(|_| {}),
            on_cleanup: Arc::new(|_, _, _| {}),
            on_inline_error_output: Arc::new(|_, _, _| {}),
        }
    }

    // Product default for host-runtime PTY side effects.
    pub(crate) fn host_default() -> Self {
        Self {
            on_output: Arc::new(dispatch_default_output_for_terminal_handle),
            on_cleanup: Arc::new(dispatch_default_exit_cleanup_for_pane),
            on_inline_error_output: Arc::new(|pane_id, message, on_output| {
                dispatch_inline_output_for_pane(pane_id, message, on_output);
            }),
        }
    }

    // Explicit compat seam kept for legacy callers/tests.
    pub(crate) fn compat_default() -> Self {
        Self::host_default()
    }

    pub(crate) fn with_output(
        on_output: Option<Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>>,
    ) -> Self {
        let mut hooks = Self::noop();
        if let Some(on_output) = on_output {
            hooks.on_output = on_output;
        }
        hooks
    }

    pub(crate) fn set_output(
        &mut self,
        on_output: Option<Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>>,
    ) {
        if let Some(on_output) = on_output {
            self.on_output = on_output;
        }
    }

    pub(crate) fn set_cleanup(
        &mut self,
        on_cleanup: Option<Arc<dyn Fn(PaneId, Option<ExitBehavior>) + Send + Sync>>,
    ) {
        if let Some(on_cleanup) = on_cleanup {
            self.on_cleanup = Arc::new(move |pane_id, exit_behavior, _default_exit_behavior| {
                on_cleanup(pane_id, exit_behavior);
            });
        }
    }

    pub(crate) fn set_inline_error_output(
        &mut self,
        on_inline_error_output: Option<Arc<dyn Fn(PaneId, String) + Send + Sync>>,
    ) {
        if let Some(on_inline_error_output) = on_inline_error_output {
            self.on_inline_error_output = Arc::new(move |pane_id, message, _on_output| {
                on_inline_error_output(pane_id, message);
            });
        }
    }
}

impl Default for PtyIoHooks {
    fn default() -> Self {
        Self::noop()
    }
}

fn pty_io_config_snapshot() -> PtyIoConfigSnapshot {
    let parser = current_host_output_parser_config();
    PtyIoConfigSnapshot {
        parser: OutputParserConfigSnapshot {
            buffer_size: parser.buffer_size,
            coalesce_delay_ms: parser.coalesce_delay_ms,
        },
    }
}

fn pty_io_config_snapshot_for_pane(pane: &Arc<dyn Pane>) -> PtyIoConfigSnapshot {
    if let Some(local_pane) = pane.downcast_ref::<LocalPane>() {
        return PtyIoConfigSnapshot {
            parser: OutputParserConfigSnapshot {
                buffer_size: local_pane.output_parser_buffer_size(),
                coalesce_delay_ms: local_pane.output_parser_coalesce_delay_ms(),
            },
        };
    }

    pty_io_config_snapshot()
}

pub(crate) fn dispatch_inline_output_for_pane(
    pane_id: PaneId,
    message: String,
    on_output: Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>,
) {
    let mut parser = termwiz::escape::parser::Parser::new();
    let mut actions = vec![Action::CSI(CSI::Sgr(Sgr::Reset))];
    parser.parse(message.as_bytes(), |action| actions.push(action));

    promise::spawn::spawn_into_main_thread(async move {
        if let Some(pane) = terminal_by_id(pane_id) {
            pane.perform_actions(actions);
            on_output(SessionTerminalHandle::new(pane_id as u64));
        }
    })
    .detach();
}

pub(crate) fn dispatch_default_output_for_terminal_handle(terminal_handle: SessionTerminalHandle) {
    let Some(pane_id) = usize::try_from(terminal_handle.as_u64()).ok() else {
        return;
    };
    notify_runtime_any_thread(HostRuntimeEvent::PaneOutput(pane_id));
}

pub(crate) fn dispatch_default_exit_cleanup_for_pane(
    pane_id: PaneId,
    exit_behavior: Option<ExitBehavior>,
    default_exit_behavior: ExitBehavior,
) {
    match exit_behavior.unwrap_or(default_exit_behavior) {
        ExitBehavior::Hold | ExitBehavior::CloseOnCleanExit => {
            log::trace!("checking for dead windows after EOF on pane {}", pane_id);
            prune_dead_windows_on_main_thread();
        }
        ExitBehavior::Close => {
            remove_pane_on_main_thread(pane_id);
        }
    }
}

fn default_pty_io_dispatcher(hooks: PtyIoHooks, _config: PtyIoConfigSnapshot) -> PtyIoDispatcher {
    let on_output = hooks.on_output;
    let cleanup_hook = hooks.on_cleanup;
    let inline_error_output_hook = hooks.on_inline_error_output;
    let on_cleanup = Arc::new(
        move |pane_id: PaneId, exit_behavior: Option<ExitBehavior>| {
            cleanup_hook(pane_id, exit_behavior, current_host_exit_behavior());
        },
    );
    let on_output_for_inline_error = Arc::clone(&on_output);
    let on_inline_error_output = Arc::new(move |pane_id: PaneId, message: String| {
        inline_error_output_hook(pane_id, message, Arc::clone(&on_output_for_inline_error));
    });

    PtyIoDispatcher {
        on_output,
        on_cleanup,
        on_inline_error_output,
    }
}

fn send_actions_to_pane(
    pane: &Weak<dyn Pane>,
    dead: &Arc<AtomicBool>,
    actions: Vec<Action>,
    dispatcher: &PtyIoDispatcher,
) {
    let start = Instant::now();
    match pane.upgrade() {
        Some(pane) => {
            let terminal_handle = pane.terminal_handle();
            pane.perform_actions(actions);
            histogram!("send_actions_to_mux.perform_actions.latency").record(start.elapsed());
            (dispatcher.on_output)(terminal_handle);
        }
        None => {
            dead.store(true, Ordering::Relaxed);
        }
    }
    histogram!("send_actions_to_mux.rate").record(1.);
}

fn parse_buffered_data(
    pane: Weak<dyn Pane>,
    dead: &Arc<AtomicBool>,
    mut rx: FileDescriptor,
    config: OutputParserConfigSnapshot,
    dispatcher: PtyIoDispatcher,
) {
    let mut buf = vec![0; config.buffer_size];
    let mut parser = termwiz::escape::parser::Parser::new();
    let mut actions = vec![];
    let mut hold = false;
    let mut action_size = 0;
    let delay = Duration::from_millis(config.coalesce_delay_ms);
    let mut deadline = None;

    loop {
        match rx.read(&mut buf) {
            Ok(size) if size == 0 => {
                dead.store(true, Ordering::Relaxed);
                break;
            }
            Err(_) => {
                dead.store(true, Ordering::Relaxed);
                break;
            }
            Ok(size) => {
                parser.parse(&buf[0..size], |action| {
                    let mut flush = false;
                    match &action {
                        Action::CSI(CSI::Mode(Mode::SetDecPrivateMode(DecPrivateMode::Code(
                            DecPrivateModeCode::SynchronizedOutput,
                        )))) => {
                            hold = true;
                            if !actions.is_empty() {
                                send_actions_to_pane(
                                    &pane,
                                    dead,
                                    std::mem::take(&mut actions),
                                    &dispatcher,
                                );
                                action_size = 0;
                            }
                        }
                        Action::CSI(CSI::Mode(Mode::ResetDecPrivateMode(
                            DecPrivateMode::Code(DecPrivateModeCode::SynchronizedOutput),
                        ))) => {
                            hold = false;
                            flush = true;
                        }
                        Action::CSI(CSI::Device(dev)) if matches!(**dev, Device::SoftReset) => {
                            hold = false;
                            flush = true;
                        }
                        _ => {}
                    };
                    action.append_to(&mut actions);

                    if flush && !actions.is_empty() {
                        send_actions_to_pane(
                            &pane,
                            dead,
                            std::mem::take(&mut actions),
                            &dispatcher,
                        );
                        action_size = 0;
                    }
                });
                action_size += size;
                if !actions.is_empty() && !hold {
                    if action_size < buf.len() {
                        let poll_delay = match deadline {
                            None => {
                                deadline.replace(Instant::now() + delay);
                                Some(delay)
                            }
                            Some(target) => target.checked_duration_since(Instant::now()),
                        };
                        if poll_delay.is_some() {
                            let mut pfd = [pollfd {
                                fd: rx.as_socket_descriptor(),
                                events: POLLIN,
                                revents: 0,
                            }];
                            if let Ok(1) = poll(&mut pfd, poll_delay) {
                                continue;
                            }
                        }
                    }

                    send_actions_to_pane(&pane, dead, std::mem::take(&mut actions), &dispatcher);
                    deadline = None;
                    action_size = 0;
                }
            }
        }
    }

    if !actions.is_empty() {
        send_actions_to_pane(&pane, dead, std::mem::take(&mut actions), &dispatcher);
    }
}

fn set_socket_buffer(fd: &mut FileDescriptor, option: i32, size: usize) -> anyhow::Result<()> {
    let size = size as c_int;
    let socklen = std::mem::size_of_val(&size);
    unsafe {
        let res = libc::setsockopt(
            fd.as_socket_descriptor(),
            SOL_SOCKET,
            option,
            &size as *const c_int as *const _,
            socklen as _,
        );
        if res == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error()).context("setsockopt")
        }
    }
}

fn allocate_socketpair() -> anyhow::Result<(FileDescriptor, FileDescriptor)> {
    let (mut tx, mut rx) = socketpair().context("socketpair")?;
    set_socket_buffer(&mut tx, SO_SNDBUF, BUFSIZE)
        .context("SO_SNDBUF")
        .ok();
    set_socket_buffer(&mut rx, SO_RCVBUF, BUFSIZE)
        .context("SO_RCVBUF")
        .ok();
    Ok((tx, rx))
}

fn read_from_pane_pty(
    pane: Weak<dyn Pane>,
    banner: Option<String>,
    mut reader: Box<dyn Read>,
    config: PtyIoConfigSnapshot,
    dispatcher: PtyIoDispatcher,
) {
    let mut buf = vec![0; BUFSIZE];
    let dead = Arc::new(AtomicBool::new(false));

    let (pane_id, exit_behavior) = match pane.upgrade() {
        Some(pane) => (pane_id_for_pane(pane.as_ref()), pane.exit_behavior()),
        None => return,
    };

    let (mut tx, rx) = match allocate_socketpair() {
        Ok(pair) => pair,
        Err(err) => {
            log::error!("read_from_pane_pty: Unable to allocate a socketpair: {err:#}");
            (dispatcher.on_inline_error_output)(
                pane_id,
                format!(
                    "⚠️  Chatminal: read_from_pane_pty: Unable to allocate a socketpair: {err:#}"
                ),
            );
            return;
        }
    };

    std::thread::spawn({
        let dead = Arc::clone(&dead);
        let dispatcher = dispatcher.clone();
        move || parse_buffered_data(pane, &dead, rx, config.parser, dispatcher)
    });

    if let Some(banner) = banner {
        tx.write_all(banner.as_bytes()).ok();
    }

    while !dead.load(Ordering::Relaxed) {
        match reader.read(&mut buf) {
            Ok(size) if size == 0 => {
                log::trace!("read_pty EOF: pane_id {}", pane_id);
                break;
            }
            Err(err) => {
                log::error!("read_pty failed: pane {} {:?}", pane_id, err);
                break;
            }
            Ok(size) => {
                histogram!("read_from_pane_pty.bytes.rate").record(size as f64);
                log::trace!("read_pty pane {pane_id} read {size} bytes");
                if let Err(err) = tx.write_all(&buf[..size]) {
                    log::error!(
                        "read_pty failed to write to parser: pane {} {:?}",
                        pane_id,
                        err
                    );
                    break;
                }
            }
        }
    }

    (dispatcher.on_cleanup)(pane_id, exit_behavior);
    dead.store(true, Ordering::Relaxed);
}

pub(crate) fn start_pane_pty_reader(pane: &Arc<dyn Pane>, hooks: PtyIoHooks) -> anyhow::Result<()> {
    if let Some(reader) = pane.reader()? {
        let config = pty_io_config_snapshot_for_pane(pane);
        let pane = Arc::downgrade(pane);
        let dispatcher = default_pty_io_dispatcher(hooks, config);
        thread::spawn(move || read_from_pane_pty(pane, None, reader, config, dispatcher));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::PtyIoHooks;
    use chatminal_runtime::SessionTerminalHandle;
    use config::ExitBehavior;
    use std::sync::{Arc, Mutex};

    #[test]
    fn with_output_keeps_explicit_output_owner() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_clone = Arc::clone(&seen);
        let hooks = PtyIoHooks::with_output(Some(Arc::new(move |handle| {
            seen_clone.lock().unwrap().push(handle.as_u64());
        })));

        (hooks.on_output)(SessionTerminalHandle::new(42));

        assert_eq!(seen.lock().unwrap().as_slice(), &[42]);
    }

    #[test]
    fn with_output_does_not_install_hidden_inline_error_fallbacks() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_clone = Arc::clone(&seen);
        let hooks = PtyIoHooks::with_output(Some(Arc::new(move |handle| {
            seen_clone.lock().unwrap().push(handle.as_u64());
        })));

        (hooks.on_inline_error_output)(9, "spawn failed".to_string(), Arc::clone(&hooks.on_output));

        assert!(seen.lock().unwrap().is_empty());
    }

    #[test]
    fn default_does_not_install_hidden_inline_error_output_owner() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_clone = Arc::clone(&seen);
        let hooks = PtyIoHooks::default();

        (hooks.on_inline_error_output)(
            9,
            "spawn failed".to_string(),
            Arc::new(move |handle| {
                seen_clone.lock().unwrap().push(handle.as_u64());
            }),
        );

        assert!(seen.lock().unwrap().is_empty());
    }

    #[test]
    fn set_cleanup_wraps_custom_cleanup_owner() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_clone = Arc::clone(&seen);
        let mut hooks = PtyIoHooks::compat_default();
        hooks.set_cleanup(Some(Arc::new(move |pane_id, exit_behavior| {
            seen_clone.lock().unwrap().push((pane_id, exit_behavior));
        })));

        (hooks.on_cleanup)(7, Some(ExitBehavior::Hold), ExitBehavior::Close);

        assert_eq!(
            seen.lock().unwrap().as_slice(),
            &[(7, Some(ExitBehavior::Hold))]
        );
    }

    #[test]
    fn set_inline_error_output_wraps_custom_inline_owner() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_clone = Arc::clone(&seen);
        let mut hooks = PtyIoHooks::compat_default();
        hooks.set_inline_error_output(Some(Arc::new(move |pane_id, message| {
            seen_clone.lock().unwrap().push((pane_id, message));
        })));

        (hooks.on_inline_error_output)(9, "spawn failed".to_string(), Arc::new(|_| {}));

        assert_eq!(
            seen.lock().unwrap().as_slice(),
            &[(9, "spawn failed".to_string())]
        );
    }
}
