use crate::pane::{Pane, PaneId, pane_id_for_pane};
use crate::workspace_ids::SessionTerminalHandle;
use anyhow::Context;
use config::ExitBehavior;
use filedescriptor::{AsRawSocketDescriptor, FileDescriptor, POLLIN, poll, pollfd, socketpair};
#[cfg(unix)]
use libc::{SO_RCVBUF, SO_SNDBUF, SOL_SOCKET, c_int};
use metrics::histogram;
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::raw::c_int;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::{Duration, Instant};
use termwiz::escape::csi::{DecPrivateMode, DecPrivateModeCode, Device, Mode};
use termwiz::escape::{Action, CSI};
#[cfg(windows)]
use winapi::um::winsock2::{SO_RCVBUF, SO_SNDBUF, SOL_SOCKET};

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
    pub(crate) on_output: Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>,
    pub(crate) on_cleanup: PtyCleanupHook,
    pub(crate) on_inline_error_output: PtyInlineErrorOutputHook,
}

impl PtyIoHooks {
    pub(crate) fn noop() -> Self {
        Self {
            on_output: Arc::new(|_| {}),
            on_cleanup: Arc::new(|_, _, _| {}),
            on_inline_error_output: Arc::new(|_, _, _| {}),
        }
    }
}

impl Default for PtyIoHooks {
    fn default() -> Self {
        Self::noop()
    }
}

fn default_config_snapshot() -> PtyIoConfigSnapshot {
    let config = config::configuration();
    PtyIoConfigSnapshot {
        parser: OutputParserConfigSnapshot {
            buffer_size: config.output_parser_buffer_size,
            coalesce_delay_ms: config.output_parser_coalesce_delay_ms,
        },
    }
}

fn config_snapshot_for_pane(pane: &Arc<dyn Pane>) -> PtyIoConfigSnapshot {
    use crate::localpane::LocalPane;
    if let Some(local_pane) = pane.downcast_ref::<LocalPane>() {
        return PtyIoConfigSnapshot {
            parser: OutputParserConfigSnapshot {
                buffer_size: local_pane.output_parser_buffer_size(),
                coalesce_delay_ms: local_pane.output_parser_coalesce_delay_ms(),
            },
        };
    }
    default_config_snapshot()
}

fn build_dispatcher(hooks: PtyIoHooks) -> PtyIoDispatcher {
    let on_output = hooks.on_output;
    let cleanup_hook = hooks.on_cleanup;
    let inline_error_output_hook = hooks.on_inline_error_output;
    let on_cleanup = Arc::new(
        move |pane_id: PaneId, exit_behavior: Option<ExitBehavior>| {
            // Use noop default_exit_behavior when no host runtime is available
            cleanup_hook(
                pane_id,
                exit_behavior,
                config::configuration().exit_behavior,
            );
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

#[cfg(unix)]
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

#[cfg(windows)]
fn set_socket_buffer(fd: &mut FileDescriptor, option: i32, size: usize) -> anyhow::Result<()> {
    let size = size as c_int;
    let socklen = std::mem::size_of_val(&size) as i32;
    unsafe {
        let res = winapi::um::winsock2::setsockopt(
            fd.as_socket_descriptor(),
            SOL_SOCKET,
            option,
            &size as *const c_int as *const _,
            socklen,
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
        let config = config_snapshot_for_pane(pane);
        let pane = Arc::downgrade(pane);
        let dispatcher = build_dispatcher(hooks);
        thread::spawn(move || read_from_pane_pty(pane, None, reader, config, dispatcher));
    }
    Ok(())
}
