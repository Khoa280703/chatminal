use crate::client::{ClientId, ClientInfo};
use crate::pane::{CachePolicy, Pane, PaneId};
use crate::tab::{SplitRequest, Tab, TabId};
use crate::window::Window;
use anyhow::{anyhow, Context, Error};
use chatminal_runtime::{RuntimeId, SessionTerminalHandle};
use config::{configuration, ExitBehavior};
use engine_term::{Clipboard, ClipboardSelection, DownloadHandler, TerminalSize};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use percent_encoding::percent_decode_str;
use portable_pty::{CommandBuilder, PtySize};
use spawn_target::{SpawnTarget, SplitSource};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub mod activity;
pub mod client;
pub mod localpane;
mod localpane_hooks;
pub mod pane;
mod pty_io;
pub mod renderable;
pub mod spawn_target;
pub mod tab;
pub mod termwiztermtab;
pub mod window;

use localpane_hooks::LocalPaneHooks;
pub(crate) use pty_io::{
    dispatch_default_output_for_terminal_handle, dispatch_inline_output_for_pane,
    start_pane_pty_reader, PtyIoHooks,
};

use crate::activity::Activity;

pub const DEFAULT_WORKSPACE: &str = "default";

pub(crate) fn try_global_mux() -> Option<Arc<Mux>> {
    Mux::try_get()
}

pub(crate) fn with_mux<R>(func: impl FnOnce(&Mux) -> R) -> Option<R> {
    let mux = try_global_mux()?;
    Some(func(&mux))
}

fn with_control_plane<R>(func: impl FnOnce(&HostRuntimeControlPlane) -> R) -> Option<R> {
    with_mux(|mux| func(&mux.control))
}

fn with_mux_and_control_plane<R>(
    func: impl FnOnce(&Mux, &HostRuntimeControlPlane) -> R,
) -> Option<R> {
    with_mux(|mux| func(mux, &mux.control))
}

pub(crate) fn with_mux_strict<R>(func: impl FnOnce(&Mux) -> R) -> R {
    func(&Mux::get())
}

pub(crate) fn notify_mux(notification: MuxNotification) {
    let _ = with_mux(|mux| mux.notify(notification));
}

pub(crate) fn notify_mux_any_thread(notification: MuxNotification) {
    Mux::notify_from_any_thread(notification);
}

pub(crate) fn prune_dead_windows_on_main_thread() {
    promise::spawn::spawn_into_main_thread(async move {
        let _ = with_mux(|mux| mux.prune_dead_windows());
    })
    .detach();
}

pub(crate) fn remove_pane_on_main_thread(pane_id: PaneId) {
    promise::spawn::spawn_into_main_thread(async move {
        let _ = with_mux(|mux| mux.remove_pane(pane_id));
    })
    .detach();
}

pub(crate) fn terminal_by_id(pane_id: usize) -> Option<Arc<dyn Pane>> {
    with_mux(|mux| mux.get_pane(pane_id)).flatten()
}

pub fn terminal_by_handle(terminal_handle: SessionTerminalHandle) -> Option<Arc<dyn Pane>> {
    let pane_id = usize::try_from(terminal_handle.as_u64()).ok()?;
    terminal_by_id(pane_id)
}

pub fn alloc_terminal_handle_value() -> usize {
    crate::pane::alloc_pane_id()
}

/// Initialize the host runtime by creating and installing a global Mux instance.
pub fn initialize_host_runtime(
    primary_spawn_target: Option<Arc<dyn SpawnTarget>>,
) -> anyhow::Result<Arc<MuxHandle>> {
    if let Some(mux) = try_global_mux() {
        if let Some(spawn_target) = primary_spawn_target.as_ref() {
            mux.control.set_primary_spawn_target(spawn_target);
        }
        return Ok(Arc::new(MuxHandle(mux)));
    }
    let mux = Arc::new(Mux::new(primary_spawn_target));
    Mux::set_mux(&mux);
    Ok(Arc::new(MuxHandle(Arc::clone(&mux))))
}

/// Shut down the host runtime, dropping the global Mux instance.
pub fn shutdown_host_runtime() {
    Mux::shutdown();
}

/// Returns `true` if the host runtime has been initialized.
pub fn is_host_runtime_available() -> bool {
    with_control_plane(|_| ()).is_some()
}

/// Opaque handle to a freshly-initialized Mux. Provides the narrow set of
/// setup operations (register_client, replace_identity, subscribe) that callers
/// need right after initialization. The Mux type itself stays `pub(crate)`.
pub struct MuxHandle(Arc<Mux>);

impl MuxHandle {
    pub fn register_client(&self, client_id: Arc<ClientId>) {
        self.0.register_client(client_id);
    }

    pub fn replace_identity(&self, id: Option<Arc<ClientId>>) -> Option<Arc<ClientId>> {
        self.0.replace_identity(id)
    }

    pub fn set_active_workspace(&self, workspace: &str) {
        self.0.set_active_workspace(workspace);
    }

    pub fn subscribe<F>(&self, subscriber: F)
    where
        F: Fn(HostRuntimeNotification) -> bool + 'static + Send + Sync,
    {
        self.0
            .subscribe(move |notification| subscriber(notification.into()));
    }
}

pub fn root_active_runtime_id() -> Option<RuntimeId> {
    with_mux(|mux| mux.root_active_tab().map(|tab| tab.runtime_id())).flatten()
}

pub fn remove_terminal_handle(terminal_handle: SessionTerminalHandle) -> bool {
    let Some(pane_id) = usize::try_from(terminal_handle.as_u64()).ok() else {
        return false;
    };
    if let Some(mux) = try_global_mux() {
        mux.remove_pane(pane_id);
    }
    true
}

pub fn register_pane(pane: &Arc<dyn Pane>) -> Result<(), Error> {
    with_mux_strict(|mux| mux.add_pane_without_default_side_effects(pane))
}

pub(crate) fn register_pane_with_default_side_effects(pane: &Arc<dyn Pane>) -> Result<(), Error> {
    with_mux_strict(|mux| mux.add_pane(pane))
}

pub(crate) fn register_pane_with_default_side_effects_and_io_hooks(
    pane: &Arc<dyn Pane>,
    hooks: PtyIoHooks,
) -> Result<(), Error> {
    with_mux_strict(|mux| mux.add_pane_with_default_side_effects_and_io_hooks(pane, hooks))
}

/// Register a pane with the global Mux, skipping default side effects, and
/// optionally providing a callback that replaces `Mux::notify_from_any_thread`
/// for PTY output notifications on this pane.
pub fn register_pane_with_output_callback(
    pane: &Arc<dyn Pane>,
    on_pane_output: Option<Arc<dyn Fn(SessionTerminalHandle) + Send + Sync>>,
) -> Result<(), Error> {
    register_pane_with_io_hooks(pane, PtyIoHooks::with_output(on_pane_output))
}

pub(crate) fn register_pane_with_io_hooks(
    pane: &Arc<dyn Pane>,
    hooks: PtyIoHooks,
) -> Result<(), Error> {
    with_mux_strict(|mux| mux.add_pane_with_io_hooks(pane, hooks))
}

pub(crate) fn tab_by_id(tab_id: TabId) -> Option<Arc<Tab>> {
    with_mux(|mux| mux.get_tab(tab_id)).flatten()
}

pub fn runtime_entry_by_runtime_id(runtime_id: RuntimeId) -> Option<Arc<Tab>> {
    let tab_id = usize::try_from(runtime_id.as_u64()).ok()?;
    tab_by_id(tab_id)
}

pub fn runtime_entry_info_by_runtime_id(runtime_id: RuntimeId) -> Option<RuntimeEntryInfo> {
    runtime_entry_by_runtime_id(runtime_id).map(|tab| runtime_entry_info(&tab))
}

pub(crate) fn remove_tab_by_id(tab_id: TabId) {
    if let Some(mux) = try_global_mux() {
        let _ = mux.remove_tab(tab_id);
    }
}

pub fn remove_runtime_entry_by_runtime_id(runtime_id: RuntimeId) -> bool {
    let Some(tab_id) = usize::try_from(runtime_id.as_u64()).ok() else {
        return false;
    };
    remove_tab_by_id(tab_id);
    true
}

pub fn register_tab(tab: &Arc<Tab>) -> Result<(), Error> {
    with_mux_strict(|mux| mux.add_tab_and_active_pane(tab))
}

pub fn attach_tab_to_window(tab: &Arc<Tab>) -> anyhow::Result<()> {
    with_mux_strict(|mux| mux.attach_tab(tab))
}

pub fn with_root_window<R>(func: impl FnOnce(&Window) -> R) -> Option<R> {
    let mux = try_global_mux()?;
    let window = mux.root_window();
    Some(func(&window))
}

pub fn with_root_window_mut<R>(func: impl FnOnce(&mut Window) -> R) -> Option<R> {
    let mux = try_global_mux()?;
    let mut window = mux.root_window_mut();
    Some(func(&mut window))
}

pub fn root_window_workspace_name() -> Option<String> {
    with_root_window(|window| window.get_workspace().to_string())
}

pub fn root_window_title() -> Option<String> {
    with_root_window(|window| window.get_title().to_string())
}

pub fn set_root_window_workspace_name(workspace: &str) -> bool {
    let Some(mux) = try_global_mux() else {
        return false;
    };
    mux.window.write().set_workspace(workspace);
    true
}

pub fn set_root_window_title(title: &str) -> bool {
    let Some(mux) = try_global_mux() else {
        return false;
    };
    mux.window.write().set_title(title);
    true
}

pub fn focus_root_runtime_entry(runtime_id: RuntimeId) -> bool {
    let Some(tab_id) = usize::try_from(runtime_id.as_u64()).ok() else {
        return false;
    };
    with_root_window_mut(|window| {
        let Some(tab_idx) = window.idx_by_id(tab_id) else {
            return false;
        };
        window.save_and_then_set_active(tab_idx);
        true
    })
    .unwrap_or(false)
}

fn root_runtime_entries() -> Vec<Arc<Tab>> {
    with_root_window(|window| window.iter().cloned().collect()).unwrap_or_default()
}

pub fn root_runtime_ids() -> Vec<RuntimeId> {
    root_runtime_entries()
        .into_iter()
        .map(|tab| tab.runtime_id())
        .collect()
}

pub fn root_runtime_entry_infos() -> Vec<RuntimeEntryInfo> {
    root_runtime_entries()
        .into_iter()
        .map(|tab| runtime_entry_info(&tab))
        .collect()
}

pub fn root_has_runtime_entries_with_panes() -> bool {
    root_runtime_entries()
        .iter()
        .any(|tab| !tab.iter_panes_ignoring_zoom().is_empty())
}

pub fn resize_all_root_runtime_entries(size: TerminalSize) {
    for tab in root_runtime_entries() {
        tab.resize(size);
    }
}

pub(crate) fn root_active_runtime_entry() -> Option<Arc<Tab>> {
    with_root_window(|window| window.get_active().cloned()).flatten()
}

pub(crate) fn root_first_runtime_entry() -> Option<Arc<Tab>> {
    with_root_window(|window| window.get_by_idx(0).cloned()).flatten()
}

pub fn root_window_spawn_context_state() -> (TerminalSize, Option<SessionTerminalHandle>) {
    let default_size = default_initial_terminal_size();
    let size = root_first_runtime_entry()
        .map(|tab| tab.get_size())
        .unwrap_or(default_size);
    let pane = root_active_runtime_entry()
        .and_then(|tab| tab.get_active_pane())
        .map(|pane| SessionTerminalHandle::new(pane.pane_id() as u64));
    (size, pane)
}

pub fn active_workspace_name() -> Option<String> {
    with_mux_and_control_plane(|mux, control| {
        control
            .workspace_for_identity()
            .unwrap_or_else(|| mux.get_default_workspace())
    })
}

pub fn set_active_workspace_name(workspace: &str) -> bool {
    let Some(mux) = try_global_mux() else {
        return false;
    };
    if let Some(ident) = mux.control.active_identity() {
        if mux.control.set_workspace_for_client(&ident, workspace) {
            mux.notify(MuxNotification::ActiveWorkspaceChanged(ident));
        }
    }
    mux.window.write().set_workspace(workspace);
    true
}

pub fn rename_workspace(old_workspace: &str, new_workspace: &str) -> bool {
    let Some(mux) = try_global_mux() else {
        return false;
    };
    if old_workspace == new_workspace {
        return true;
    }

    mux.notify(MuxNotification::WorkspaceRenamed {
        old_workspace: old_workspace.to_string(),
        new_workspace: new_workspace.to_string(),
    });

    {
        let mut window = mux.window.write();
        if window.get_workspace() == old_workspace {
            window.set_workspace(new_workspace);
        }
    }
    mux.recompute_pane_count();
    for client_id in mux.control.rename_workspace(old_workspace, new_workspace) {
        mux.notify(MuxNotification::ActiveWorkspaceChanged(client_id));
    }
    true
}

pub(crate) fn focus_pane_and_tab(pane_id: PaneId) -> anyhow::Result<()> {
    with_mux_strict(|mux| mux.focus_pane_and_containing_tab(pane_id))
}

pub fn focus_terminal_handle(terminal_handle: SessionTerminalHandle) -> anyhow::Result<()> {
    let pane_id = usize::try_from(terminal_handle.as_u64())
        .map_err(|_| anyhow!("invalid terminal handle {}", terminal_handle.as_u64()))?;
    focus_pane_and_tab(pane_id)
}

pub fn record_focus_for_terminal_handle(terminal_handle: SessionTerminalHandle) -> bool {
    let Some(pane_id) = usize::try_from(terminal_handle.as_u64()).ok() else {
        return false;
    };
    let Some(mux) = try_global_mux() else {
        return false;
    };
    if let Some(ident) = mux.control.active_identity() {
        mux.record_focus_for_client(&ident, pane_id);
    }
    true
}

pub fn record_input_for_current_identity() -> bool {
    let Some(mux) = try_global_mux() else {
        return false;
    };
    if let Some(ident) = mux.control.active_identity() {
        mux.control.client_had_input(&ident);
    }
    true
}

pub fn active_identity() -> Option<Arc<ClientId>> {
    with_control_plane(|control| control.active_identity()).flatten()
}

pub fn active_workspace_for_client(ident: &Arc<ClientId>) -> Option<String> {
    with_mux_and_control_plane(|mux, control| {
        control
            .workspace_for_client(ident)
            .unwrap_or_else(|| mux.get_default_workspace())
    })
}

pub fn set_active_workspace_for_client(ident: &Arc<ClientId>, workspace: &str) -> bool {
    let Some(mux) = try_global_mux() else {
        return false;
    };
    if mux.control.set_workspace_for_client(ident, workspace) {
        mux.notify(MuxNotification::ActiveWorkspaceChanged(ident.clone()));
    }
    true
}

pub fn is_workspace_empty(workspace: &str) -> Option<bool> {
    with_mux(|mux| mux.is_workspace_empty(workspace))
}

pub fn iter_workspaces() -> Vec<String> {
    with_mux(|mux| mux.iter_workspaces()).unwrap_or_default()
}

pub(crate) fn resolve_pane_id(pane_id: PaneId) -> Option<TabId> {
    with_mux(|mux| mux.resolve_pane_id(pane_id)).flatten()
}

pub fn resolve_runtime_id_for_terminal_handle(
    terminal_handle: SessionTerminalHandle,
) -> Option<RuntimeId> {
    let pane_id = usize::try_from(terminal_handle.as_u64()).ok()?;
    resolve_pane_id(pane_id).map(|tab_id| RuntimeId::new(tab_id as u64))
}

pub fn runtime_entry_by_session_id(session_id: &str) -> Option<Arc<Tab>> {
    with_mux(|mux| mux.get_tab_by_chatminal_session_id(session_id)).flatten()
}

pub fn runtime_entry_info_by_session_id(session_id: &str) -> Option<RuntimeEntryInfo> {
    runtime_entry_by_session_id(session_id).map(|tab| runtime_entry_info(&tab))
}

pub fn resolve_focused_pane(client_id: &ClientId) -> Option<FocusedPaneBinding> {
    with_mux_and_control_plane(|mux, control| {
        let pane_id = control.focused_pane_for_client(client_id)?;
        let runtime_entry_id = mux.resolve_pane_id(pane_id)?;
        Some(FocusedPaneBinding::new(runtime_entry_id, pane_id))
    })
    .flatten()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusedPaneBinding {
    runtime_entry_id: TabId,
    pane_id: PaneId,
}

impl FocusedPaneBinding {
    fn new(runtime_entry_id: TabId, pane_id: PaneId) -> Self {
        Self {
            runtime_entry_id,
            pane_id,
        }
    }

    pub fn runtime_id(self) -> RuntimeId {
        RuntimeId::new(self.runtime_entry_id as u64)
    }

    pub fn terminal_handle(self) -> SessionTerminalHandle {
        SessionTerminalHandle::new(self.pane_id as u64)
    }
}

pub fn iter_panes() -> Vec<Arc<dyn Pane>> {
    with_mux(|mux| mux.iter_panes()).unwrap_or_default()
}

pub async fn spawn_tab(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    current_terminal_handle: Option<SessionTerminalHandle>,
) -> anyhow::Result<(Arc<Tab>, Arc<dyn Pane>)> {
    let mux = Mux::get();
    let current_pane_id = current_terminal_handle
        .map(|terminal_handle| usize::try_from(terminal_handle.as_u64()))
        .transpose()
        .map_err(|_| anyhow!("invalid terminal handle for spawn_tab"))?;
    mux.spawn_tab(command, command_dir, size, current_pane_id)
        .await
}

pub async fn split_pane(
    terminal_handle: SessionTerminalHandle,
    request: SplitRequest,
    source: SplitSource,
) -> anyhow::Result<(Arc<dyn Pane>, TerminalSize)> {
    let mux = Mux::get();
    let pane_id = usize::try_from(terminal_handle.as_u64())
        .map_err(|_| anyhow!("invalid terminal handle for split_pane"))?;
    mux.split_pane(pane_id, request, source).await
}

pub fn set_primary_spawn_target(spawn_target: &Arc<dyn SpawnTarget>) -> bool {
    let Some(mux) = try_global_mux() else {
        return false;
    };
    mux.control.set_primary_spawn_target(spawn_target);
    true
}

pub fn primary_spawn_target() -> Option<Arc<dyn SpawnTarget>> {
    with_control_plane(|control| control.primary_spawn_target()).flatten()
}

fn pane_chatminal_session_id(pane: &dyn Pane) -> Option<String> {
    use engine_dynamic::Value;

    match pane.get_metadata() {
        Value::Object(obj) => {
            let key = Value::String("chatminal_session_id".to_string());
            match obj.get(&key) {
                Some(Value::String(session_id)) => Some(session_id.clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn pane_terminal_instance_id(pane: &dyn Pane) -> Option<u64> {
    use engine_dynamic::Value;

    match pane.get_metadata() {
        Value::Object(obj) => {
            let key = Value::String("chatminal_terminal_instance_id".to_string());
            match obj.get(&key) {
                Some(Value::U64(terminal_instance_id)) => Some(*terminal_instance_id),
                Some(Value::I64(terminal_instance_id)) => (*terminal_instance_id).try_into().ok(),
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn terminal_handle_for_pane(pane: &dyn Pane) -> SessionTerminalHandle {
    SessionTerminalHandle::new(pane.pane_id() as u64)
}

#[derive(Clone, Debug)]
pub struct RuntimeEntryInfo {
    pub runtime_id: RuntimeId,
    pub title: String,
    pub session_id: Option<String>,
    pub active_terminal_handle: Option<SessionTerminalHandle>,
    pub active_terminal_instance_id: Option<u64>,
    pub size: TerminalSize,
}

fn runtime_entry_info(tab: &Tab) -> RuntimeEntryInfo {
    let active_pane = tab.get_active_pane();
    RuntimeEntryInfo {
        runtime_id: tab.runtime_id(),
        title: tab.get_title(),
        session_id: active_pane
            .as_deref()
            .and_then(pane_chatminal_session_id)
            .or_else(|| {
                tab.iter_panes()
                    .into_iter()
                    .find_map(|pos| pane_chatminal_session_id(pos.pane.as_ref()))
            }),
        active_terminal_handle: active_pane.as_deref().map(terminal_handle_for_pane),
        active_terminal_instance_id: active_pane.as_deref().and_then(pane_terminal_instance_id),
        size: tab.get_size(),
    }
}

#[derive(Clone, Debug)]
pub enum HostRuntimeNotification {
    PaneOutput(SessionTerminalHandle),
    PaneAdded(SessionTerminalHandle),
    PaneRemoved(SessionTerminalHandle),
    WindowInvalidated,
    WindowWorkspaceChanged,
    ActiveWorkspaceChanged(Arc<ClientId>),
    Alert {
        pane_id: SessionTerminalHandle,
        alert: engine_term::Alert,
    },
    Empty,
    AssignClipboard {
        pane_id: SessionTerminalHandle,
        selection: ClipboardSelection,
        clipboard: Option<String>,
    },
    SaveToDownloads {
        name: Option<String>,
        data: Arc<Vec<u8>>,
    },
    TabAddedToWindow {
        runtime_id: RuntimeId,
    },
    PaneFocused(SessionTerminalHandle),
    TabResized(RuntimeId),
    TabTitleChanged {
        runtime_id: RuntimeId,
        title: String,
    },
    WindowTitleChanged {
        title: String,
    },
    WorkspaceRenamed {
        old_workspace: String,
        new_workspace: String,
    },
}

impl From<MuxNotification> for HostRuntimeNotification {
    fn from(notification: MuxNotification) -> Self {
        match notification {
            MuxNotification::PaneOutput(pane_id) => {
                Self::PaneOutput(SessionTerminalHandle::new(pane_id as u64))
            }
            MuxNotification::PaneAdded(pane_id) => {
                Self::PaneAdded(SessionTerminalHandle::new(pane_id as u64))
            }
            MuxNotification::PaneRemoved(pane_id) => {
                Self::PaneRemoved(SessionTerminalHandle::new(pane_id as u64))
            }
            MuxNotification::WindowInvalidated => Self::WindowInvalidated,
            MuxNotification::WindowWorkspaceChanged => Self::WindowWorkspaceChanged,
            MuxNotification::ActiveWorkspaceChanged(client_id) => {
                Self::ActiveWorkspaceChanged(client_id)
            }
            MuxNotification::Alert { pane_id, alert } => Self::Alert {
                pane_id: SessionTerminalHandle::new(pane_id as u64),
                alert,
            },
            MuxNotification::Empty => Self::Empty,
            MuxNotification::AssignClipboard {
                pane_id,
                selection,
                clipboard,
            } => Self::AssignClipboard {
                pane_id: SessionTerminalHandle::new(pane_id as u64),
                selection,
                clipboard,
            },
            MuxNotification::SaveToDownloads { name, data } => Self::SaveToDownloads { name, data },
            MuxNotification::TabAddedToWindow { tab_id } => Self::TabAddedToWindow {
                runtime_id: RuntimeId::new(tab_id as u64),
            },
            MuxNotification::PaneFocused(pane_id) => {
                Self::PaneFocused(SessionTerminalHandle::new(pane_id as u64))
            }
            MuxNotification::TabResized(tab_id) => Self::TabResized(RuntimeId::new(tab_id as u64)),
            MuxNotification::TabTitleChanged { tab_id, title } => Self::TabTitleChanged {
                runtime_id: RuntimeId::new(tab_id as u64),
                title,
            },
            MuxNotification::WindowTitleChanged { title } => Self::WindowTitleChanged { title },
            MuxNotification::WorkspaceRenamed {
                old_workspace,
                new_workspace,
            } => Self::WorkspaceRenamed {
                old_workspace,
                new_workspace,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum MuxNotification {
    PaneOutput(PaneId),
    PaneAdded(PaneId),
    PaneRemoved(PaneId),
    WindowInvalidated,
    WindowWorkspaceChanged,
    ActiveWorkspaceChanged(Arc<ClientId>),
    Alert {
        pane_id: PaneId,
        alert: engine_term::Alert,
    },
    Empty,
    AssignClipboard {
        pane_id: PaneId,
        selection: ClipboardSelection,
        clipboard: Option<String>,
    },
    SaveToDownloads {
        name: Option<String>,
        data: Arc<Vec<u8>>,
    },
    TabAddedToWindow {
        tab_id: TabId,
    },
    PaneFocused(PaneId),
    TabResized(TabId),
    TabTitleChanged {
        tab_id: TabId,
        title: String,
    },
    WindowTitleChanged {
        title: String,
    },
    WorkspaceRenamed {
        old_workspace: String,
        new_workspace: String,
    },
}

static SUB_ID: AtomicUsize = AtomicUsize::new(0);

type MuxSubscriber = Box<dyn Fn(MuxNotification) -> bool + Send + Sync>;

struct HostRuntimeControlPlane {
    primary_spawn_target: RwLock<Option<Arc<dyn SpawnTarget>>>,
    subscribers: RwLock<HashMap<usize, MuxSubscriber>>,
    clients: RwLock<HashMap<ClientId, ClientInfo>>,
    identity: RwLock<Option<Arc<ClientId>>>,
}

impl HostRuntimeControlPlane {
    fn new(primary_spawn_target: Option<Arc<dyn SpawnTarget>>) -> Self {
        Self {
            primary_spawn_target: RwLock::new(primary_spawn_target),
            subscribers: RwLock::new(HashMap::new()),
            clients: RwLock::new(HashMap::new()),
            identity: RwLock::new(None),
        }
    }

    fn client_had_input(&self, client_id: &ClientId) {
        if let Some(info) = self.clients.write().get_mut(client_id) {
            info.update_last_input();
        }
    }

    fn register_client(&self, client_id: Arc<ClientId>) {
        self.clients
            .write()
            .insert((*client_id).clone(), ClientInfo::new(client_id));
    }

    fn replace_identity(&self, id: Option<Arc<ClientId>>) -> Option<Arc<ClientId>> {
        std::mem::replace(&mut *self.identity.write(), id)
    }

    fn active_identity(&self) -> Option<Arc<ClientId>> {
        self.identity.read().clone()
    }

    fn focused_pane_for_client(&self, client_id: &ClientId) -> Option<PaneId> {
        self.clients.read().get(client_id)?.focused_pane_id()
    }

    fn update_focus_for_client(&self, client_id: &ClientId, pane_id: PaneId) -> Option<PaneId> {
        let mut clients = self.clients.write();
        let info = clients.get_mut(client_id)?;
        let prior = info.focused_pane_id();
        info.update_focused_pane(pane_id);
        prior
    }

    fn workspace_for_identity(&self) -> Option<String> {
        let ident = self.active_identity()?;
        self.workspace_for_client(&ident)
    }

    fn workspace_for_client(&self, ident: &Arc<ClientId>) -> Option<String> {
        self.clients
            .read()
            .get(ident)
            .and_then(|info| info.active_workspace.clone())
    }

    fn set_workspace_for_client(&self, ident: &Arc<ClientId>, workspace: &str) -> bool {
        let mut clients = self.clients.write();
        let Some(info) = clients.get_mut(ident) else {
            return false;
        };
        info.active_workspace.replace(workspace.to_string());
        true
    }

    fn rename_workspace(&self, old_workspace: &str, new_workspace: &str) -> Vec<Arc<ClientId>> {
        let mut changed_clients = Vec::new();
        for client in self.clients.write().values_mut() {
            if client.active_workspace.as_deref() == Some(old_workspace) {
                client.active_workspace.replace(new_workspace.to_string());
                changed_clients.push(client.client_id.clone());
            }
        }
        changed_clients
    }

    fn subscribe<F>(&self, subscriber: F)
    where
        F: Fn(MuxNotification) -> bool + 'static + Send + Sync,
    {
        let sub_id = SUB_ID.fetch_add(1, Ordering::Relaxed);
        self.subscribers
            .write()
            .insert(sub_id, Box::new(subscriber));
    }

    fn notify(&self, notification: MuxNotification) {
        let mut subscribers = self.subscribers.write();
        subscribers.retain(|_, notify| notify(notification.clone()));
    }

    fn primary_spawn_target(&self) -> Option<Arc<dyn SpawnTarget>> {
        self.primary_spawn_target.read().as_ref().map(Arc::clone)
    }

    fn set_primary_spawn_target(&self, spawn_target: &Arc<dyn SpawnTarget>) {
        *self.primary_spawn_target.write() = Some(Arc::clone(spawn_target));
    }
}

pub(crate) struct Mux {
    tabs: RwLock<HashMap<TabId, Arc<Tab>>>,
    panes: RwLock<HashMap<PaneId, Arc<dyn Pane>>>,
    window: RwLock<Window>,
    control: HostRuntimeControlPlane,
    num_panes_by_workspace: RwLock<HashMap<String, usize>>,
    main_thread_id: std::thread::ThreadId,
}

fn default_workspace_name() -> String {
    configuration()
        .default_workspace
        .as_deref()
        .unwrap_or(DEFAULT_WORKSPACE)
        .to_string()
}

pub(crate) fn switch_to_last_active_tab_when_closing_tab() -> bool {
    configuration().switch_to_last_active_tab_when_closing_tab
}

pub(crate) fn unzoom_on_switch_pane() -> bool {
    configuration().unzoom_on_switch_pane
}

pub fn default_initial_terminal_size() -> TerminalSize {
    configuration().initial_size(0, None)
}

lazy_static::lazy_static! {
    static ref MUX: Mutex<Option<Arc<Mux>>> = Mutex::new(None);
}

impl Mux {
    pub fn new(primary_spawn_target: Option<Arc<dyn SpawnTarget>>) -> Self {
        let workspace = default_workspace_name();
        Self {
            tabs: RwLock::new(HashMap::new()),
            panes: RwLock::new(HashMap::new()),
            window: RwLock::new(Window::new(workspace, None)),
            control: HostRuntimeControlPlane::new(primary_spawn_target),
            num_panes_by_workspace: RwLock::new(HashMap::new()),
            main_thread_id: std::thread::current().id(),
        }
    }

    fn get_default_workspace(&self) -> String {
        default_workspace_name()
    }

    pub fn is_main_thread(&self) -> bool {
        std::thread::current().id() == self.main_thread_id
    }

    fn recompute_pane_count(&self) {
        let mut count = HashMap::new();
        let window = self.window.read();
        let workspace = window.get_workspace();
        for tab in window.iter() {
            *count.entry(workspace.to_string()).or_insert(0) += match tab.count_panes() {
                Some(n) => n,
                None => {
                    // Busy: abort this and we'll retry later
                    return;
                }
            };
        }
        *self.num_panes_by_workspace.write() = count;
    }

    pub(crate) fn record_focus_for_client(&self, client_id: &ClientId, pane_id: PaneId) {
        let prior = self.control.update_focus_for_client(client_id, pane_id);

        if prior == Some(pane_id) {
            return;
        }
        // Synthesize focus events
        if let Some(prior_id) = prior {
            if let Some(pane) = self.get_pane(prior_id) {
                pane.focus_changed(false);
            }
        }
        if let Some(pane) = self.get_pane(pane_id) {
            pane.focus_changed(true);
        }
    }

    /// Called by PaneFocused event handlers to reconcile a remote
    /// pane focus event and apply its effects locally
    pub(crate) fn focus_pane_and_containing_tab(&self, pane_id: PaneId) -> anyhow::Result<()> {
        let pane = self
            .get_pane(pane_id)
            .ok_or_else(|| anyhow::anyhow!("pane {pane_id} not found"))?;

        let tab_id = self
            .resolve_pane_id(pane_id)
            .ok_or_else(|| anyhow::anyhow!("can't find {pane_id} in the mux"))?;

        // Focus/activate the containing tab within its window
        {
            let mut win = self.window.write();
            let tab_idx = win
                .idx_by_id(tab_id)
                .ok_or_else(|| anyhow::anyhow!("tab {tab_id} not in root window"))?;
            win.save_and_then_set_active(tab_idx);
        }

        // Focus/activate the pane locally
        let tab = self
            .get_tab(tab_id)
            .ok_or_else(|| anyhow::anyhow!("tab {tab_id} not found"))?;

        tab.set_active_pane(&pane);

        Ok(())
    }

    pub fn register_client(&self, client_id: Arc<ClientId>) {
        self.control.register_client(client_id);
    }

    /// Returns a list of the unique workspace names known to the mux.
    /// In single-window mode this is just the root window workspace.
    pub fn iter_workspaces(&self) -> Vec<String> {
        vec![self.window.read().get_workspace().to_string()]
    }

    pub fn set_active_workspace_for_client(&self, ident: &Arc<ClientId>, workspace: &str) {
        if self.control.set_workspace_for_client(ident, workspace) {
            self.notify(MuxNotification::ActiveWorkspaceChanged(ident.clone()));
        }
    }

    /// Assigns the active workspace name for the current identity
    pub fn set_active_workspace(&self, workspace: &str) {
        if let Some(ident) = self.control.active_identity() {
            self.set_active_workspace_for_client(&ident, workspace);
        }
        self.window.write().set_workspace(workspace);
    }

    /// Replace the active identity, returning the prior one.
    pub fn replace_identity(&self, id: Option<Arc<ClientId>>) -> Option<Arc<ClientId>> {
        self.control.replace_identity(id)
    }

    pub fn subscribe<F>(&self, subscriber: F)
    where
        F: Fn(MuxNotification) -> bool + 'static + Send + Sync,
    {
        self.control.subscribe(subscriber);
    }

    pub fn notify(&self, notification: MuxNotification) {
        self.control.notify(notification);
    }

    pub fn notify_from_any_thread(notification: MuxNotification) {
        if let Some(mux) = Mux::try_get() {
            if mux.is_main_thread() {
                mux.notify(notification);
                return;
            }
        }
        promise::spawn::spawn_into_main_thread(async {
            if let Some(mux) = Mux::try_get() {
                mux.notify(notification);
            }
        })
        .detach();
    }

    pub fn primary_spawn_target(&self) -> Arc<dyn SpawnTarget> {
        self.control.primary_spawn_target().unwrap()
    }

    pub fn set_mux(mux: &Arc<Mux>) {
        MUX.lock().replace(Arc::clone(mux));
    }

    pub fn shutdown() {
        MUX.lock().take();
    }

    pub fn get() -> Arc<Mux> {
        Self::try_get().unwrap()
    }

    pub fn try_get() -> Option<Arc<Mux>> {
        MUX.lock().as_ref().map(Arc::clone)
    }

    pub(crate) fn get_pane(&self, pane_id: PaneId) -> Option<Arc<dyn Pane>> {
        self.panes.read().get(&pane_id).map(Arc::clone)
    }

    pub(crate) fn get_tab(&self, tab_id: TabId) -> Option<Arc<Tab>> {
        self.tabs.read().get(&tab_id).map(Arc::clone)
    }

    /// Resolve a tab by scanning pane metadata for the given chatminal session_id.
    /// Returns None for SSH/serial/legacy sessions that carry no chatminal session_id.
    pub fn get_tab_by_chatminal_session_id(&self, session_id: &str) -> Option<Arc<Tab>> {
        self.tabs
            .read()
            .values()
            .find(|tab| {
                tab.iter_panes_ignoring_zoom().into_iter().any(|info| {
                    pane_chatminal_session_id(&*info.pane).as_deref() == Some(session_id)
                })
            })
            .map(Arc::clone)
    }

    fn register_pane_internal(
        &self,
        pane: &Arc<dyn Pane>,
        install_default_side_effects: bool,
    ) -> Result<bool, Error> {
        if self.panes.read().contains_key(&pane.pane_id()) {
            return Ok(false);
        }

        if install_default_side_effects {
            let clipboard: Arc<dyn Clipboard> = Arc::new(MuxClipboard {
                pane_id: pane.pane_id(),
            });
            pane.set_clipboard(&clipboard);

            let downloader: Arc<dyn DownloadHandler> = Arc::new(MuxDownloader {});
            pane.set_download_handler(&downloader);
        }

        self.panes.write().insert(pane.pane_id(), Arc::clone(pane));
        Ok(true)
    }

    fn add_pane_internal(
        &self,
        pane: &Arc<dyn Pane>,
        install_default_side_effects: bool,
        hooks: PtyIoHooks,
    ) -> Result<(), Error> {
        if !self.register_pane_internal(pane, install_default_side_effects)? {
            return Ok(());
        }

        let pane_id = pane.pane_id();
        start_pane_pty_reader(pane, hooks)?;
        self.recompute_pane_count();
        self.notify(MuxNotification::PaneAdded(pane_id));
        Ok(())
    }

    pub fn add_pane(&self, pane: &Arc<dyn Pane>) -> Result<(), Error> {
        self.add_pane_internal(pane, true, PtyIoHooks::default())
    }

    pub fn add_pane_without_default_side_effects(&self, pane: &Arc<dyn Pane>) -> Result<(), Error> {
        self.add_pane_internal(pane, false, PtyIoHooks::default())
    }

    pub fn add_pane_with_io_hooks(
        &self,
        pane: &Arc<dyn Pane>,
        hooks: PtyIoHooks,
    ) -> Result<(), Error> {
        self.add_pane_internal(pane, false, hooks)
    }

    pub fn add_pane_with_default_side_effects_and_io_hooks(
        &self,
        pane: &Arc<dyn Pane>,
        hooks: PtyIoHooks,
    ) -> Result<(), Error> {
        self.add_pane_internal(pane, true, hooks)
    }

    pub fn add_tab_and_active_pane(&self, tab: &Arc<Tab>) -> Result<(), Error> {
        self.tabs.write().insert(tab.tab_id(), Arc::clone(tab));
        let pane = tab
            .get_active_pane()
            .ok_or_else(|| anyhow!("tab MUST have an active pane"))?;
        self.add_pane(&pane)
    }

    fn remove_pane_internal(&self, pane_id: PaneId) {
        log::debug!("removing pane {}", pane_id);
        let mut changed = false;
        if let Some(pane) = self.panes.write().remove(&pane_id).clone() {
            log::debug!("killing pane {}", pane_id);
            pane.kill();
            self.notify(MuxNotification::PaneRemoved(pane_id));
            changed = true;
        }

        if changed {
            self.recompute_pane_count();
        }
    }

    fn remove_tab_internal(&self, tab_id: TabId) -> Option<Arc<Tab>> {
        log::debug!("remove_tab_internal tab {}", tab_id);

        let tab = self.tabs.write().remove(&tab_id)?;

        if let Some(mut window) = self.window.try_write() {
            window.remove_by_id(tab_id);
        }

        let mut pane_ids = vec![];
        for pos in tab.iter_panes_ignoring_zoom() {
            pane_ids.push(pos.pane.pane_id());
        }
        log::debug!("panes to remove: {pane_ids:?}");
        for pane_id in pane_ids {
            self.remove_pane_internal(pane_id);
        }
        self.recompute_pane_count();

        Some(tab)
    }

    fn clear_root_window_internal(&self) {
        log::debug!("clear_root_window_internal");
        let tab_ids: Vec<TabId> = self.window.read().iter().map(|tab| tab.tab_id()).collect();
        for tab_id in tab_ids {
            self.remove_tab_internal(tab_id);
        }
        self.recompute_pane_count();
    }

    pub(crate) fn remove_pane(&self, pane_id: PaneId) {
        self.remove_pane_internal(pane_id);
        self.prune_dead_windows();
    }

    pub(crate) fn remove_tab(&self, tab_id: TabId) -> Option<Arc<Tab>> {
        let tab = self.remove_tab_internal(tab_id);
        self.prune_dead_windows();
        tab
    }

    pub fn prune_dead_windows(&self) {
        if Activity::count() > 0 {
            log::trace!("prune_dead_windows: Activity::count={}", Activity::count());
            return;
        }
        let live_tab_ids: Vec<TabId> = self.tabs.read().keys().cloned().collect();
        let mut root_window_empty = false;
        let dead_tab_ids: Vec<TabId>;

        {
            let mut window = match self.window.try_write() {
                Some(w) => w,
                None => {
                    log::trace!("prune_dead_windows: self.window already borrowed");
                    return;
                }
            };
            window.prune_dead_tabs(&live_tab_ids);
            if window.is_empty() {
                log::trace!("prune_dead_windows: root window is now empty");
                root_window_empty = true;
            }

            dead_tab_ids = self
                .tabs
                .read()
                .iter()
                .filter_map(|(&id, tab)| if tab.is_dead() { Some(id) } else { None })
                .collect();
        }

        for tab_id in dead_tab_ids {
            log::trace!("tab {} is dead", tab_id);
            self.remove_tab_internal(tab_id);
        }

        if root_window_empty {
            log::trace!("root window is dead");
            self.clear_root_window_internal();
        }

        if self.is_empty() {
            log::trace!("prune_dead_windows: is_empty, send MuxNotification::Empty");
            self.notify(MuxNotification::Empty);
        } else {
            log::trace!("prune_dead_windows: not empty");
        }
    }

    pub fn root_window(&self) -> MappedRwLockReadGuard<'_, Window> {
        RwLockReadGuard::map(self.window.read(), |window| window)
    }

    pub fn root_window_mut(&self) -> MappedRwLockWriteGuard<'_, Window> {
        RwLockWriteGuard::map(self.window.write(), |window| window)
    }

    pub fn root_active_tab(&self) -> Option<Arc<Tab>> {
        let window = self.root_window();
        window.get_active().map(Arc::clone)
    }

    pub fn attach_tab(&self, tab: &Arc<Tab>) -> anyhow::Result<()> {
        let tab_id = tab.tab_id();
        {
            let mut window = self.root_window_mut();
            window.push(tab);
        }
        self.recompute_pane_count();
        self.notify(MuxNotification::TabAddedToWindow { tab_id });
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.panes.read().is_empty()
    }

    pub fn is_workspace_empty(&self, workspace: &str) -> bool {
        *self
            .num_panes_by_workspace
            .read()
            .get(workspace)
            .unwrap_or(&0)
            == 0
    }

    pub fn iter_panes(&self) -> Vec<Arc<dyn Pane>> {
        self.panes
            .read()
            .iter()
            .map(|(_, v)| Arc::clone(v))
            .collect()
    }

    pub(crate) fn resolve_pane_id(&self, pane_id: PaneId) -> Option<TabId> {
        let mut tab_id = None;
        for tab in self.tabs.read().values() {
            for p in tab.iter_panes_ignoring_zoom() {
                if p.pane.pane_id() == pane_id {
                    tab_id = Some(tab.tab_id());
                    break;
                }
            }
        }
        tab_id
    }

    pub fn resolve_spawn_target(
        &self,
        // TODO: disambiguate with TabId
        _pane_id: Option<PaneId>,
    ) -> anyhow::Result<Arc<dyn SpawnTarget>> {
        Ok(self.primary_spawn_target())
    }

    fn resolve_cwd(
        &self,
        command_dir: Option<String>,
        pane: Option<Arc<dyn Pane>>,
        policy: CachePolicy,
    ) -> Option<String> {
        command_dir.or_else(|| {
            match pane {
                Some(pane) => pane
                    .get_current_working_dir(policy)
                    .and_then(|url| {
                        percent_decode_str(url.path())
                            .decode_utf8()
                            .ok()
                            .map(|path| path.into_owned())
                    })
                    .map(|path| {
                        // On Windows the file URI can produce a path like:
                        // `/C:\Users` which is valid in a file URI, but the leading slash
                        // is not liked by the windows file APIs, so we strip it off here.
                        let bytes = path.as_bytes();
                        if bytes.len() > 2 && bytes[0] == b'/' && bytes[2] == b':' {
                            path[1..].to_owned()
                        } else {
                            path
                        }
                    }),
                _ => None,
            }
        })
    }

    pub async fn split_pane(
        &self,
        // TODO: disambiguate with TabId
        pane_id: PaneId,
        request: SplitRequest,
        source: SplitSource,
    ) -> anyhow::Result<(Arc<dyn Pane>, TerminalSize)> {
        let tab_id = self
            .resolve_pane_id(pane_id)
            .ok_or_else(|| anyhow!("pane_id {} invalid", pane_id))?;

        let spawn_target = self
            .resolve_spawn_target(Some(pane_id))
            .context("resolve_spawn_target")?;

        let current_pane = self
            .get_pane(pane_id)
            .ok_or_else(|| anyhow!("pane_id {} is invalid", pane_id))?;
        let term_config = current_pane.get_config();

        let source = match source {
            SplitSource::Spawn {
                command,
                command_dir,
            } => SplitSource::Spawn {
                command,
                command_dir: self.resolve_cwd(
                    command_dir,
                    Some(Arc::clone(&current_pane)),
                    CachePolicy::FetchImmediate,
                ),
            },
            other => other,
        };

        #[allow(deprecated)]
        let pane = spawn_target
            .split_pane(
                source,
                RuntimeId::new(tab_id as u64),
                SessionTerminalHandle::new(pane_id as u64),
                request,
            )
            .await?;
        if let Some(config) = term_config {
            pane.set_config(config);
        }

        // FIXME: clipboard

        let dims = pane.get_dimensions();

        let size = TerminalSize {
            cols: dims.cols,
            rows: dims.viewport_rows,
            pixel_height: 0, // FIXME: split pane pixel dimensions
            pixel_width: 0,
            dpi: dims.dpi,
        };

        Ok((pane, size))
    }

    pub async fn spawn_tab(
        &self,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        size: TerminalSize,
        current_pane_id: Option<PaneId>,
    ) -> anyhow::Result<(Arc<Tab>, Arc<dyn Pane>)> {
        let spawn_target = self
            .resolve_spawn_target(current_pane_id)
            .context("resolve_spawn_target")?;

        let (term_config, size) = match self.root_active_tab() {
            Some(tab) => {
                let pane = tab
                    .get_active_pane()
                    .ok_or_else(|| anyhow!("active tab in root window has no panes"))?;
                (pane.get_config(), tab.get_size())
            }
            None => (None, size),
        };

        let cwd = self.resolve_cwd(
            command_dir,
            match current_pane_id {
                Some(id) => self.get_pane(id),
                None => None,
            },
            CachePolicy::FetchImmediate,
        );

        let tab = spawn_target
            .spawn(size, command.clone(), cwd.clone())
            .await
            .with_context(|| {
                format!(
                    "Spawning on target `{}`: {size:?} command={command:?} cwd={cwd:?}",
                    spawn_target.spawn_target_name()
                )
            })?;

        let pane = tab
            .get_active_pane()
            .ok_or_else(|| anyhow!("missing active pane on tab!?"))?;

        if let Some(config) = term_config {
            pane.set_config(config);
        }

        // FIXME: clipboard?

        let mut window = self.root_window_mut();
        if let Some(idx) = window.idx_by_id(tab.tab_id()) {
            window.save_and_then_set_active(idx);
        }

        Ok((tab, pane))
    }
}


pub(crate) fn terminal_size_to_pty_size(size: TerminalSize) -> anyhow::Result<PtySize> {
    Ok(PtySize {
        rows: size.rows.try_into()?,
        cols: size.cols.try_into()?,
        pixel_height: size.pixel_height.try_into()?,
        pixel_width: size.pixel_width.try_into()?,
    })
}

struct MuxClipboard {
    pane_id: PaneId,
}

impl Clipboard for MuxClipboard {
    fn set_contents(
        &self,
        selection: ClipboardSelection,
        clipboard: Option<String>,
    ) -> anyhow::Result<()> {
        let mux = try_global_mux()
            .ok_or_else(|| anyhow::anyhow!("MuxClipboard::set_contents: no Mux?"))?;
        mux.notify(MuxNotification::AssignClipboard {
            pane_id: self.pane_id,
            selection,
            clipboard,
        });
        Ok(())
    }
}

struct MuxDownloader {}

impl engine_term::DownloadHandler for MuxDownloader {
    fn save_to_downloads(&self, name: Option<String>, data: Vec<u8>) {
        if let Some(mux) = try_global_mux() {
            mux.notify(MuxNotification::SaveToDownloads {
                name,
                data: Arc::new(data),
            });
        }
    }
}
