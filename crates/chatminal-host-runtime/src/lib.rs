use crate::client::{ClientId, ClientInfo};
use crate::pane::{
    pane_id_for_pane, pane_id_from_terminal_handle, CachePolicy, CloseReason, Pane, PaneId,
};
use crate::tab::{SplitDirection, SplitRequest, Tab, TabId};
use crate::window::Window;
use anyhow::{anyhow, Context, Error};
use chatminal_runtime::{RuntimeId, SessionTerminalHandle};
use config::{
    current_config_handle, keyassignment::SessionDirection, ConfigHandle, ExitBehavior,
    GuiPosition, OutputParserConfig,
};
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

pub(crate) fn try_host_runtime_root() -> Option<Arc<HostRuntimeRoot>> {
    HOST_RUNTIME_ROOT.lock().as_ref().map(Arc::clone)
}

pub(crate) fn with_host_runtime_root<R>(func: impl FnOnce(&HostRuntimeRoot) -> R) -> Option<R> {
    let root = try_host_runtime_root()?;
    Some(func(&root))
}

pub(crate) fn with_host_runtime_root_strict<R>(func: impl FnOnce(&HostRuntimeRoot) -> R) -> R {
    let root = try_host_runtime_root().expect("host runtime root must exist");
    func(&root)
}

fn with_control_plane<R>(func: impl FnOnce(&HostRuntimeControlPlane) -> R) -> Option<R> {
    with_host_runtime_root(|root| func(&root.control))
}

pub(crate) fn notify_runtime(notification: HostRuntimeEvent) {
    let _ = with_host_runtime_root(|root| root.notify(notification));
}

pub(crate) fn notify_runtime_any_thread(notification: HostRuntimeEvent) {
    if let Some(root) = try_host_runtime_root() {
        if root.is_main_thread() {
            root.notify(notification);
            return;
        }
    }
    promise::spawn::spawn_into_main_thread(async move {
        if let Some(root) = try_host_runtime_root() {
            root.notify(notification);
        }
    })
    .detach();
}

pub(crate) fn prune_dead_windows_on_main_thread() {
    promise::spawn::spawn_into_main_thread(async move {
        let _ = with_host_runtime_root(|root| root.prune_dead_windows());
    })
    .detach();
}

pub(crate) fn remove_pane_on_main_thread(pane_id: PaneId) {
    promise::spawn::spawn_into_main_thread(async move {
        let _ = with_host_runtime_root(|root| root.remove_pane(pane_id));
    })
    .detach();
}

pub(crate) fn terminal_by_id(pane_id: usize) -> Option<Arc<dyn Pane>> {
    with_host_runtime_root(|root| root.get_pane(pane_id)).flatten()
}

pub fn terminal_by_handle(terminal_handle: SessionTerminalHandle) -> Option<Arc<dyn Pane>> {
    let pane_id = pane_id_from_terminal_handle(terminal_handle)?;
    terminal_by_id(pane_id)
}

pub fn terminal_by_terminal_instance_id(terminal_instance_id: u64) -> Option<Arc<dyn Pane>> {
    iter_panes()
        .into_iter()
        .find(|pane| pane_terminal_instance_id(pane.as_ref()) == Some(terminal_instance_id))
}

pub fn terminal_by_public_id(public_id: u64) -> Option<Arc<dyn Pane>> {
    terminal_by_handle(SessionTerminalHandle::new(public_id))
        .or_else(|| terminal_by_terminal_instance_id(public_id))
}

pub fn alloc_terminal_handle_value() -> usize {
    crate::pane::alloc_pane_id()
}

fn new_host_runtime_root(
    primary_spawn_target: Option<Arc<dyn SpawnTarget>>,
    config: ConfigHandle,
) -> Arc<HostRuntimeRoot> {
    let workspace = configured_default_workspace_name(&config);
    Arc::new(HostRuntimeRoot {
        tabs: RwLock::new(HashMap::new()),
        panes: RwLock::new(HashMap::new()),
        window: RwLock::new(Window::new(workspace, None)),
        control: HostRuntimeControlPlane::new(primary_spawn_target),
        config: RwLock::new(config),
        num_panes_by_workspace: RwLock::new(HashMap::new()),
        main_thread_id: std::thread::current().id(),
    })
}

fn install_host_runtime_root(root: &Arc<HostRuntimeRoot>) {
    HOST_RUNTIME_ROOT.lock().replace(Arc::clone(root));
}

fn clear_host_runtime_root() {
    HOST_RUNTIME_ROOT.lock().take();
}

/// Initialize the host runtime by creating and installing a global host runtime root.
pub fn initialize_host_runtime(
    primary_spawn_target: Option<Arc<dyn SpawnTarget>>,
) -> anyhow::Result<Arc<HostRuntimeHandle>> {
    initialize_host_runtime_with_config(primary_spawn_target, current_config_handle())
}

pub fn initialize_host_runtime_with_config(
    primary_spawn_target: Option<Arc<dyn SpawnTarget>>,
    config: ConfigHandle,
) -> anyhow::Result<Arc<HostRuntimeHandle>> {
    if let Some(root) = try_host_runtime_root() {
        if let Some(spawn_target) = primary_spawn_target.as_ref() {
            root.control.set_primary_spawn_target(spawn_target);
        }
        root.set_config(config);
        return Ok(Arc::new(HostRuntimeHandle(root)));
    }
    let root = new_host_runtime_root(primary_spawn_target, config);
    install_host_runtime_root(&root);
    Ok(Arc::new(HostRuntimeHandle(root)))
}

/// Shut down the host runtime by clearing the installed root.
pub fn shutdown_host_runtime() {
    clear_host_runtime_root();
}

pub fn set_host_runtime_config(config: ConfigHandle) -> bool {
    with_host_runtime_root(|root| root.set_config(config)).is_some()
}

/// Returns `true` if the host runtime has been initialized.
pub fn is_host_runtime_available() -> bool {
    with_control_plane(|_| ()).is_some()
}

/// Opaque handle to a freshly-initialized host runtime root.
pub struct HostRuntimeHandle(Arc<HostRuntimeRoot>);

impl HostRuntimeHandle {
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

pub fn register_runtime_client(client_id: Arc<ClientId>) -> bool {
    with_host_runtime_root(|root| root.register_client(client_id)).is_some()
}

pub fn replace_active_identity(id: Option<Arc<ClientId>>) -> Option<Arc<ClientId>> {
    with_host_runtime_root(|root| root.replace_identity(id)).flatten()
}

pub fn subscribe_runtime_notifications<F>(subscriber: F) -> bool
where
    F: Fn(HostRuntimeNotification) -> bool + 'static + Send + Sync,
{
    with_host_runtime_root(|root| {
        root.subscribe(move |notification| subscriber(notification.into()));
    })
    .is_some()
}

pub fn root_active_runtime_id() -> Option<RuntimeId> {
    root_window_info().and_then(|info| info.active_runtime_id)
}

pub fn remove_terminal_handle(terminal_handle: SessionTerminalHandle) -> bool {
    let Some(pane_id) = pane_id_from_terminal_handle(terminal_handle) else {
        return false;
    };
    let _ = with_host_runtime_root(|root| root.remove_pane(pane_id));
    true
}

pub fn register_pane(pane: &Arc<dyn Pane>) -> Result<(), Error> {
    with_host_runtime_root_strict(|root| root.add_pane_without_default_side_effects(pane))
}

pub(crate) fn register_pane_with_default_side_effects(pane: &Arc<dyn Pane>) -> Result<(), Error> {
    with_host_runtime_root_strict(|root| {
        root.add_pane_with_default_side_effects_and_io_hooks(pane, PtyIoHooks::host_default())
    })
}

pub(crate) fn register_pane_with_default_side_effects_and_io_hooks(
    pane: &Arc<dyn Pane>,
    hooks: PtyIoHooks,
) -> Result<(), Error> {
    with_host_runtime_root_strict(|root| {
        root.add_pane_with_default_side_effects_and_io_hooks(pane, hooks)
    })
}

/// Register a pane with the host runtime, skipping default side effects, and
/// optionally providing a callback that replaces the default PTY output
/// notification owner for this pane.
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
    with_host_runtime_root_strict(|root| root.add_pane_with_io_hooks(pane, hooks))
}

pub(crate) fn tab_by_id(tab_id: TabId) -> Option<Arc<Tab>> {
    with_host_runtime_root(|root| root.get_tab(tab_id)).flatten()
}

pub(crate) fn runtime_entry_by_runtime_id(runtime_id: RuntimeId) -> Option<Arc<Tab>> {
    let tab_id = usize::try_from(runtime_id.as_u64()).ok()?;
    tab_by_id(tab_id)
}

pub fn runtime_entry_info_by_runtime_id(runtime_id: RuntimeId) -> Option<RuntimeEntryInfo> {
    runtime_entry_by_runtime_id(runtime_id).map(|tab| runtime_entry_info(&tab))
}

pub fn runtime_entry_exists(runtime_id: RuntimeId) -> bool {
    runtime_entry_by_runtime_id(runtime_id).is_some()
}

pub(crate) fn remove_tab_by_id(tab_id: TabId) {
    let _ = with_host_runtime_root(|root| root.remove_tab(tab_id));
}

pub fn remove_runtime_entry_by_runtime_id(runtime_id: RuntimeId) -> bool {
    let Some(tab_id) = usize::try_from(runtime_id.as_u64()).ok() else {
        return false;
    };
    remove_tab_by_id(tab_id);
    true
}

pub fn register_tab(tab: &Arc<Tab>) -> Result<(), Error> {
    with_host_runtime_root_strict(|root| {
        root.add_tab_and_active_pane_with_io_hooks(tab, PtyIoHooks::host_default())
    })
}

pub fn attach_tab_to_window(tab: &Arc<Tab>) -> anyhow::Result<()> {
    with_host_runtime_root_strict(|root| root.attach_tab(tab))
}

pub(crate) fn build_runtime_entry_tab(
    pane: &Arc<dyn Pane>,
    size: TerminalSize,
    title: Option<&str>,
) -> Arc<Tab> {
    let tab = Arc::new(Tab::new(&size));
    tab.assign_pane(pane);
    if let Some(title) = title {
        tab.set_title(title);
    }
    tab
}

pub(crate) fn register_attached_runtime_entry_tab(tab: &Arc<Tab>) -> anyhow::Result<()> {
    register_tab(tab)?;
    attach_tab_to_window(tab)
}

pub fn create_attached_runtime_entry_for_terminal(
    pane: &Arc<dyn Pane>,
    size: TerminalSize,
    title: Option<&str>,
) -> anyhow::Result<RuntimeEntryInfo> {
    let tab = build_runtime_entry_tab(pane, size, title);
    register_attached_runtime_entry_tab(&tab)?;
    Ok(runtime_entry_info(&tab))
}

pub fn with_root_window<R>(func: impl FnOnce(&Window) -> R) -> Option<R> {
    let root = try_host_runtime_root()?;
    let window = root.root_window();
    Some(func(&window))
}

pub fn with_root_window_mut<R>(func: impl FnOnce(&mut Window) -> R) -> Option<R> {
    let root = try_host_runtime_root()?;
    let mut window = root.root_window_mut();
    Some(func(&mut window))
}

#[derive(Clone, Debug, PartialEq)]
pub struct RootWindowInfo {
    pub workspace: String,
    pub title: String,
    pub initial_position: Option<GuiPosition>,
    pub active_runtime_id: Option<RuntimeId>,
    pub last_active_runtime_id: Option<RuntimeId>,
    pub runtime_ids: Vec<RuntimeId>,
}

pub fn root_window_info() -> Option<RootWindowInfo> {
    with_root_window(|window| RootWindowInfo {
        workspace: window.get_workspace().to_string(),
        title: window.get_title().to_string(),
        initial_position: window.get_initial_position().clone(),
        active_runtime_id: window.get_active().map(|tab| tab.runtime_id()),
        last_active_runtime_id: window
            .get_last_active_idx()
            .and_then(|idx| window.get_by_idx(idx))
            .map(|tab| tab.runtime_id()),
        runtime_ids: window.iter().map(|tab| tab.runtime_id()).collect(),
    })
}

pub fn root_window_workspace_name() -> Option<String> {
    root_window_info().map(|info| info.workspace)
}

pub fn root_window_title() -> Option<String> {
    root_window_info().map(|info| info.title)
}

pub fn root_last_active_runtime_id() -> Option<RuntimeId> {
    root_window_info().and_then(|info| info.last_active_runtime_id)
}

pub fn root_window_initial_position() -> Option<GuiPosition> {
    root_window_info().and_then(|info| info.initial_position)
}

pub fn focus_root_runtime_entry_by_session_id(session_id: &str) -> bool {
    let Some(runtime_id) = runtime_id_for_session_id(session_id) else {
        return false;
    };
    focus_root_runtime_entry(runtime_id)
}

pub fn set_root_window_workspace_name(workspace: &str) -> bool {
    let Some(root) = try_host_runtime_root() else {
        return false;
    };
    root.window.write().set_workspace(workspace);
    true
}

pub fn set_root_window_title(title: &str) -> bool {
    let Some(root) = try_host_runtime_root() else {
        return false;
    };
    root.window.write().set_title(title);
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
    root_window_info()
        .map(|info| info.runtime_ids)
        .unwrap_or_default()
}

pub fn root_runtime_entry_count() -> usize {
    root_window_info()
        .map(|info| info.runtime_ids.len())
        .unwrap_or_default()
}

pub fn root_active_runtime_entry_index() -> Option<usize> {
    with_root_window(|window| {
        if window.get_active().is_some() {
            Some(window.get_active_idx())
        } else {
            None
        }
    })
    .flatten()
}

pub fn root_last_active_runtime_entry_index() -> Option<usize> {
    with_root_window(|window| window.get_last_active_idx()).flatten()
}

pub fn root_runtime_id_at_index(index: usize) -> Option<RuntimeId> {
    with_root_window(|window| window.get_by_idx(index).map(|tab| tab.runtime_id())).flatten()
}

pub fn root_runtime_entry_info_at_index(index: usize) -> Option<RuntimeEntryInfo> {
    let runtime_id = root_runtime_id_at_index(index)?;
    runtime_entry_info_by_runtime_id(runtime_id)
}

pub fn root_runtime_entry_infos() -> Vec<RuntimeEntryInfo> {
    root_runtime_entries()
        .into_iter()
        .map(|tab| runtime_entry_info(&tab))
        .collect()
}

pub fn root_active_runtime_entry_info() -> Option<RuntimeEntryInfo> {
    let runtime_id = root_active_runtime_id()?;
    runtime_entry_info_by_runtime_id(runtime_id)
}

pub fn focus_root_runtime_entry_index(index: usize) -> bool {
    with_root_window_mut(|window| {
        if window.get_by_idx(index).is_none() {
            return false;
        }
        window.save_and_then_set_active(index);
        true
    })
    .unwrap_or(false)
}

pub fn focus_root_last_runtime_entry() -> bool {
    let Some(index) = root_last_active_runtime_entry_index() else {
        return false;
    };
    focus_root_runtime_entry_index(index)
}

pub fn focus_root_runtime_entry_relative(delta: isize, wrap: bool) -> bool {
    let Some((count, active_index)) =
        with_root_window(|window| (window.len(), window.get_active_idx() as isize))
    else {
        return false;
    };
    if count == 0 {
        return false;
    }

    let target_index = if wrap {
        let count = count as isize;
        (active_index + delta).rem_euclid(count) as usize
    } else {
        (active_index + delta).clamp(0, count as isize - 1) as usize
    };
    focus_root_runtime_entry_index(target_index)
}

pub fn move_root_active_runtime_entry_to_index(index: usize) -> bool {
    with_root_window_mut(|window| {
        let count = window.len();
        if count == 0 || index >= count {
            return false;
        }

        let active = window.get_active_idx();
        let tab = window.remove_by_idx(active);
        window.insert(index, &tab);
        window.set_active_without_saving(index);
        true
    })
    .unwrap_or(false)
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
        .map(|pane| terminal_handle_for_pane(pane.as_ref()));
    (size, pane)
}

pub fn active_workspace_name() -> Option<String> {
    with_host_runtime_root(|root| {
        root.control
            .workspace_for_identity()
            .unwrap_or_else(|| root.get_default_workspace())
    })
}

pub fn set_active_workspace_name(workspace: &str) -> bool {
    let Some(root) = try_host_runtime_root() else {
        return false;
    };
    if let Some(ident) = root.control.active_identity() {
        if root.control.set_workspace_for_client(&ident, workspace) {
            root.notify(HostRuntimeEvent::ActiveWorkspaceChanged(ident));
        }
    }
    root.window.write().set_workspace(workspace);
    true
}

pub fn rename_workspace(old_workspace: &str, new_workspace: &str) -> bool {
    let Some(root) = try_host_runtime_root() else {
        return false;
    };
    if old_workspace == new_workspace {
        return true;
    }

    root.notify(HostRuntimeEvent::WorkspaceRenamed {
        old_workspace: old_workspace.to_string(),
        new_workspace: new_workspace.to_string(),
    });

    {
        let mut window = root.window.write();
        if window.get_workspace() == old_workspace {
            window.set_workspace(new_workspace);
        }
    }
    root.recompute_pane_count();
    for client_id in root.control.rename_workspace(old_workspace, new_workspace) {
        root.notify(HostRuntimeEvent::ActiveWorkspaceChanged(client_id));
    }
    true
}

pub(crate) fn focus_pane_and_tab(pane_id: PaneId) -> anyhow::Result<()> {
    with_host_runtime_root_strict(|root| root.focus_pane_and_containing_tab(pane_id))
}

pub fn focus_terminal_handle(terminal_handle: SessionTerminalHandle) -> anyhow::Result<()> {
    let pane_id = pane_id_from_terminal_handle(terminal_handle)
        .ok_or_else(|| anyhow!("invalid terminal handle {}", terminal_handle.as_u64()))?;
    focus_pane_and_tab(pane_id)
}

pub fn record_focus_for_terminal_handle(terminal_handle: SessionTerminalHandle) -> bool {
    let Some(pane_id) = pane_id_from_terminal_handle(terminal_handle) else {
        return false;
    };
    let Some(root) = try_host_runtime_root() else {
        return false;
    };
    if let Some(ident) = root.control.active_identity() {
        root.record_focus_for_client(&ident, pane_id);
    }
    true
}

pub fn record_input_for_current_identity() -> bool {
    let Some(root) = try_host_runtime_root() else {
        return false;
    };
    if let Some(ident) = root.control.active_identity() {
        root.control.client_had_input(&ident);
    }
    true
}

pub fn active_identity() -> Option<Arc<ClientId>> {
    with_control_plane(|control| control.active_identity()).flatten()
}

pub fn active_workspace_for_client(ident: &Arc<ClientId>) -> Option<String> {
    with_host_runtime_root(|root| {
        root.control
            .workspace_for_client(ident)
            .unwrap_or_else(|| root.get_default_workspace())
    })
}

pub fn set_active_workspace_for_client(ident: &Arc<ClientId>, workspace: &str) -> bool {
    let Some(root) = try_host_runtime_root() else {
        return false;
    };
    if root.control.set_workspace_for_client(ident, workspace) {
        root.notify(HostRuntimeEvent::ActiveWorkspaceChanged(ident.clone()));
    }
    true
}

pub fn is_workspace_empty(workspace: &str) -> Option<bool> {
    with_host_runtime_root(|root| root.is_workspace_empty(workspace))
}

pub fn iter_workspaces() -> Vec<String> {
    with_host_runtime_root(|root| root.iter_workspaces()).unwrap_or_default()
}

pub(crate) fn resolve_pane_id(pane_id: PaneId) -> Option<TabId> {
    with_host_runtime_root(|root| root.resolve_pane_id(pane_id)).flatten()
}

pub fn resolve_runtime_id_for_terminal_handle(
    terminal_handle: SessionTerminalHandle,
) -> Option<RuntimeId> {
    let pane_id = usize::try_from(terminal_handle.as_u64()).ok()?;
    resolve_pane_id(pane_id).map(|tab_id| RuntimeId::new(tab_id as u64))
}

pub fn resolve_runtime_id_for_terminal_instance_id(terminal_instance_id: u64) -> Option<RuntimeId> {
    let pane = terminal_by_terminal_instance_id(terminal_instance_id)?;
    resolve_runtime_id_for_terminal_handle(terminal_handle_for_pane(pane.as_ref()))
}

pub fn runtime_id_for_session_id(session_id: &str) -> Option<RuntimeId> {
    runtime_entry_info_by_session_id(session_id).map(|info| info.runtime_id)
}

pub(crate) fn runtime_entry_by_session_id(session_id: &str) -> Option<Arc<Tab>> {
    with_host_runtime_root(|root| root.get_tab_by_chatminal_session_id(session_id)).flatten()
}

pub fn runtime_entry_info_by_session_id(session_id: &str) -> Option<RuntimeEntryInfo> {
    runtime_entry_by_session_id(session_id).map(|tab| runtime_entry_info(&tab))
}

pub fn runtime_entry_exists_for_session(session_id: &str) -> bool {
    runtime_entry_by_session_id(session_id).is_some()
}

pub fn runtime_entry_contains_terminal(
    runtime_id: RuntimeId,
    terminal_handle: SessionTerminalHandle,
) -> bool {
    runtime_entry_by_runtime_id(runtime_id)
        .map(|tab| tab.contains_pane(terminal_handle))
        .unwrap_or(false)
}

pub fn set_runtime_entry_title(runtime_id: RuntimeId, title: &str) -> bool {
    let Some(tab) = runtime_entry_by_runtime_id(runtime_id) else {
        return false;
    };
    tab.set_title(title);
    true
}

pub fn set_runtime_entry_title_by_session_id(session_id: &str, title: &str) -> bool {
    let Some(tab) = runtime_entry_by_session_id(session_id) else {
        return false;
    };
    tab.set_title(title);
    true
}

pub fn runtime_entry_terminal_handles(runtime_id: RuntimeId) -> Vec<SessionTerminalHandle> {
    runtime_entry_by_runtime_id(runtime_id)
        .map(|tab| {
            tab.iter_panes_ignoring_zoom()
                .into_iter()
                .map(|pos| terminal_handle_for_pane(pos.pane.as_ref()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn runtime_entry_terminal_handles_by_session_id(
    session_id: &str,
) -> Vec<SessionTerminalHandle> {
    runtime_entry_by_session_id(session_id)
        .map(|tab| {
            tab.iter_panes_ignoring_zoom()
                .into_iter()
                .map(|pos| terminal_handle_for_pane(pos.pane.as_ref()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn runtime_entry_terminal_handle_in_direction(
    runtime_id: RuntimeId,
    direction: SessionDirection,
) -> Option<SessionTerminalHandle> {
    let tab = runtime_entry_by_runtime_id(runtime_id)?;
    tab.terminal_handle_in_direction(direction)
}

pub fn runtime_entry_terminal_handle_in_direction_by_session_id(
    session_id: &str,
    direction: SessionDirection,
) -> Option<SessionTerminalHandle> {
    let tab = runtime_entry_by_session_id(session_id)?;
    tab.terminal_handle_in_direction(direction)
}

pub fn runtime_entry_active_terminal_handle(
    runtime_id: RuntimeId,
) -> Option<SessionTerminalHandle> {
    runtime_entry_info_by_runtime_id(runtime_id).and_then(|info| info.active_terminal_handle)
}

pub fn runtime_entry_active_terminal_handle_by_session_id(
    session_id: &str,
) -> Option<SessionTerminalHandle> {
    runtime_entry_info_by_session_id(session_id).and_then(|info| info.active_terminal_handle)
}

pub fn set_runtime_entry_zoomed(runtime_id: RuntimeId, zoomed: bool) -> Option<bool> {
    let tab = runtime_entry_by_runtime_id(runtime_id)?;
    Some(tab.set_zoomed(zoomed))
}

pub fn set_runtime_entry_zoomed_by_session_id(session_id: &str, zoomed: bool) -> Option<bool> {
    let tab = runtime_entry_by_session_id(session_id)?;
    Some(tab.set_zoomed(zoomed))
}

pub fn rotate_runtime_entry_counter_clockwise(runtime_id: RuntimeId) -> bool {
    let Some(tab) = runtime_entry_by_runtime_id(runtime_id) else {
        return false;
    };
    tab.rotate_counter_clockwise();
    true
}

pub fn rotate_runtime_entry_counter_clockwise_by_session_id(session_id: &str) -> bool {
    let Some(tab) = runtime_entry_by_session_id(session_id) else {
        return false;
    };
    tab.rotate_counter_clockwise();
    true
}

pub fn rotate_runtime_entry_clockwise(runtime_id: RuntimeId) -> bool {
    let Some(tab) = runtime_entry_by_runtime_id(runtime_id) else {
        return false;
    };
    tab.rotate_clockwise();
    true
}

pub fn rotate_runtime_entry_clockwise_by_session_id(session_id: &str) -> bool {
    let Some(tab) = runtime_entry_by_session_id(session_id) else {
        return false;
    };
    tab.rotate_clockwise();
    true
}

pub fn set_runtime_entry_active_terminal(
    runtime_id: RuntimeId,
    terminal_handle: SessionTerminalHandle,
) -> bool {
    let Some(tab) = runtime_entry_by_runtime_id(runtime_id) else {
        return false;
    };
    tab.set_active_terminal_handle(terminal_handle)
}

pub fn set_runtime_entry_active_terminal_by_session_id(
    session_id: &str,
    terminal_handle: SessionTerminalHandle,
) -> bool {
    let Some(tab) = runtime_entry_by_session_id(session_id) else {
        return false;
    };
    tab.set_active_terminal_handle(terminal_handle)
}

pub fn activate_runtime_entry_terminal_index(runtime_id: RuntimeId, index: usize) -> bool {
    let terminal_handle = runtime_entry_terminal_infos(runtime_id)
        .into_iter()
        .find(|info| info.index == index)
        .map(|info| info.terminal_handle);
    let Some(terminal_handle) = terminal_handle else {
        return false;
    };
    set_runtime_entry_active_terminal(runtime_id, terminal_handle)
}

pub fn activate_runtime_entry_terminal_index_by_session_id(session_id: &str, index: usize) -> bool {
    let terminal_handle = runtime_entry_terminal_infos_by_session_id(session_id)
        .into_iter()
        .find(|info| info.index == index)
        .map(|info| info.terminal_handle);
    let Some(terminal_handle) = terminal_handle else {
        return false;
    };
    set_runtime_entry_active_terminal_by_session_id(session_id, terminal_handle)
}

pub fn activate_runtime_entry_terminal_direction(
    runtime_id: RuntimeId,
    direction: SessionDirection,
) -> bool {
    let Some(terminal_handle) = runtime_entry_terminal_handle_in_direction(runtime_id, direction)
    else {
        return false;
    };
    set_runtime_entry_active_terminal(runtime_id, terminal_handle)
}

pub fn activate_runtime_entry_terminal_direction_by_session_id(
    session_id: &str,
    direction: SessionDirection,
) -> bool {
    let Some(terminal_handle) =
        runtime_entry_terminal_handle_in_direction_by_session_id(session_id, direction)
    else {
        return false;
    };
    set_runtime_entry_active_terminal_by_session_id(session_id, terminal_handle)
}

pub fn toggle_runtime_entry_zoom(runtime_id: RuntimeId) -> bool {
    let Some(tab) = runtime_entry_by_runtime_id(runtime_id) else {
        return false;
    };
    tab.toggle_zoom();
    true
}

pub fn toggle_runtime_entry_zoom_by_session_id(session_id: &str) -> bool {
    let Some(tab) = runtime_entry_by_session_id(session_id) else {
        return false;
    };
    tab.toggle_zoom();
    true
}

pub fn resize_runtime_entry(runtime_id: RuntimeId, size: TerminalSize) -> bool {
    let Some(tab) = runtime_entry_by_runtime_id(runtime_id) else {
        return false;
    };
    tab.resize(size);
    true
}

pub fn resize_runtime_entry_by_session_id(session_id: &str, size: TerminalSize) -> bool {
    let Some(tab) = runtime_entry_by_session_id(session_id) else {
        return false;
    };
    tab.resize(size);
    true
}

pub fn adjust_runtime_entry_active_terminal_size(
    runtime_id: RuntimeId,
    direction: SessionDirection,
    amount: usize,
) -> bool {
    let Some(tab) = runtime_entry_by_runtime_id(runtime_id) else {
        return false;
    };
    tab.adjust_pane_size(direction, amount);
    true
}

pub fn adjust_runtime_entry_active_terminal_size_by_session_id(
    session_id: &str,
    direction: SessionDirection,
    amount: usize,
) -> bool {
    let Some(tab) = runtime_entry_by_session_id(session_id) else {
        return false;
    };
    tab.adjust_pane_size(direction, amount);
    true
}

pub fn swap_runtime_entry_active_with_terminal(
    runtime_id: RuntimeId,
    terminal_handle: SessionTerminalHandle,
    keep_focus: bool,
) -> bool {
    let Some(tab) = runtime_entry_by_runtime_id(runtime_id) else {
        return false;
    };
    let Some(pane_index) = tab
        .iter_panes_ignoring_zoom()
        .iter()
        .position(|pos| pos.pane.terminal_handle() == terminal_handle)
    else {
        return false;
    };
    tab.swap_active_with_index(pane_index, keep_focus).is_some()
}

pub fn swap_runtime_entry_active_with_terminal_by_session_id(
    session_id: &str,
    terminal_handle: SessionTerminalHandle,
    keep_focus: bool,
) -> bool {
    let Some(tab) = runtime_entry_by_session_id(session_id) else {
        return false;
    };
    let Some(pane_index) = tab
        .iter_panes_ignoring_zoom()
        .iter()
        .position(|pos| pos.pane.terminal_handle() == terminal_handle)
    else {
        return false;
    };
    tab.swap_active_with_index(pane_index, keep_focus).is_some()
}

pub fn runtime_entry_can_close_without_prompting(
    runtime_id: RuntimeId,
    reason: CloseReason,
) -> bool {
    runtime_entry_by_runtime_id(runtime_id)
        .map(|tab| tab.can_close_without_prompting(reason))
        .unwrap_or(false)
}

pub fn runtime_entry_can_close_without_prompting_by_session_id(
    session_id: &str,
    reason: CloseReason,
) -> bool {
    runtime_entry_by_session_id(session_id)
        .map(|tab| tab.can_close_without_prompting(reason))
        .unwrap_or(false)
}

pub fn resolve_focused_pane(client_id: &ClientId) -> Option<FocusedPaneBinding> {
    with_host_runtime_root(|root| {
        let terminal_handle = root.control.focused_terminal_handle_for_client(client_id)?;
        let pane_id = pane_id_from_terminal_handle(terminal_handle)?;
        let runtime_id = RuntimeId::new(root.resolve_pane_id(pane_id)? as u64);
        Some(FocusedPaneBinding::new(runtime_id, terminal_handle))
    })
    .flatten()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusedPaneBinding {
    runtime_id: RuntimeId,
    terminal_handle: SessionTerminalHandle,
}

impl FocusedPaneBinding {
    fn new(runtime_id: RuntimeId, terminal_handle: SessionTerminalHandle) -> Self {
        Self {
            runtime_id,
            terminal_handle,
        }
    }

    pub fn runtime_id(self) -> RuntimeId {
        self.runtime_id
    }

    pub fn terminal_handle(self) -> SessionTerminalHandle {
        self.terminal_handle
    }
}

pub fn iter_panes() -> Vec<Arc<dyn Pane>> {
    with_host_runtime_root(|root| root.iter_panes()).unwrap_or_default()
}

pub async fn spawn_tab(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    current_terminal_handle: Option<SessionTerminalHandle>,
) -> anyhow::Result<(Arc<Tab>, Arc<dyn Pane>)> {
    let root = try_host_runtime_root()
        .ok_or_else(|| anyhow!("host runtime root unavailable for spawn_tab"))?;
    let current_pane_id = current_terminal_handle
        .map(|terminal_handle| {
            pane_id_from_terminal_handle(terminal_handle)
                .ok_or_else(|| anyhow!("invalid terminal handle for spawn_tab"))
        })
        .transpose()?;
    root.spawn_tab(command, command_dir, size, current_pane_id)
        .await
}

pub async fn spawn_runtime_entry(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    current_terminal_handle: Option<SessionTerminalHandle>,
) -> anyhow::Result<(RuntimeEntryInfo, Arc<dyn Pane>)> {
    let (tab, pane) = spawn_tab(command, command_dir, size, current_terminal_handle).await?;
    Ok((runtime_entry_info(&tab), pane))
}

pub async fn split_pane(
    terminal_handle: SessionTerminalHandle,
    request: SplitRequest,
    source: SplitSource,
) -> anyhow::Result<(Arc<dyn Pane>, TerminalSize)> {
    let root = try_host_runtime_root()
        .ok_or_else(|| anyhow!("host runtime root unavailable for split_pane"))?;
    let pane_id = pane_id_from_terminal_handle(terminal_handle)
        .ok_or_else(|| anyhow!("invalid terminal handle for split_pane"))?;
    root.split_pane(pane_id, request, source).await
}

pub fn set_primary_spawn_target(spawn_target: &Arc<dyn SpawnTarget>) -> bool {
    let Some(root) = try_host_runtime_root() else {
        return false;
    };
    root.control.set_primary_spawn_target(spawn_target);
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
    pane.terminal_handle()
}

pub fn terminal_instance_id_for_pane(pane: &dyn Pane) -> Option<u64> {
    pane_terminal_instance_id(pane)
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

#[derive(Clone, Debug)]
pub struct RuntimeEntryTerminalInfo {
    pub index: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub pixel_width: usize,
    pub height: usize,
    pub pixel_height: usize,
    pub terminal_handle: SessionTerminalHandle,
    pub session_id: Option<String>,
    pub terminal_instance_id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeEntrySplitInfo {
    pub index: usize,
    pub direction: SplitDirection,
    pub left: usize,
    pub top: usize,
    pub size: usize,
}

#[derive(Clone, Debug)]
pub struct RootRuntimeEntrySummary {
    pub runtime_id: RuntimeId,
    pub index: usize,
    pub title: String,
    pub pane_count: Option<usize>,
    pub is_active: bool,
}

fn runtime_entry_terminal_info(pos: crate::tab::PositionedPane) -> RuntimeEntryTerminalInfo {
    let terminal_handle = terminal_handle_for_pane(pos.pane.as_ref());
    let session_id = pane_chatminal_session_id(pos.pane.as_ref());
    let terminal_instance_id = pane_terminal_instance_id(pos.pane.as_ref());

    RuntimeEntryTerminalInfo {
        index: pos.index,
        is_active: pos.is_active,
        is_zoomed: pos.is_zoomed,
        left: pos.left,
        top: pos.top,
        width: pos.width,
        pixel_width: pos.pixel_width,
        height: pos.height,
        pixel_height: pos.pixel_height,
        terminal_handle,
        session_id,
        terminal_instance_id,
    }
}

fn runtime_entry_split_info(pos: crate::tab::PositionedSplit) -> RuntimeEntrySplitInfo {
    RuntimeEntrySplitInfo {
        index: pos.index,
        direction: pos.direction,
        left: pos.left,
        top: pos.top,
        size: pos.size,
    }
}

fn runtime_entry_display_title(tab: &Tab) -> String {
    let title = tab.get_title();
    if !title.is_empty() {
        return title;
    }
    tab.get_active_pane()
        .map(|pane| pane.get_title())
        .unwrap_or_default()
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

pub fn runtime_entry_terminal_infos(runtime_id: RuntimeId) -> Vec<RuntimeEntryTerminalInfo> {
    runtime_entry_by_runtime_id(runtime_id)
        .map(|tab| {
            tab.iter_panes_ignoring_zoom()
                .into_iter()
                .map(runtime_entry_terminal_info)
                .collect()
        })
        .unwrap_or_default()
}

pub fn runtime_entry_terminal_infos_by_session_id(
    session_id: &str,
) -> Vec<RuntimeEntryTerminalInfo> {
    runtime_entry_by_session_id(session_id)
        .map(|tab| {
            tab.iter_panes_ignoring_zoom()
                .into_iter()
                .map(runtime_entry_terminal_info)
                .collect()
        })
        .unwrap_or_default()
}

pub fn runtime_entry_split_infos(runtime_id: RuntimeId) -> Vec<RuntimeEntrySplitInfo> {
    runtime_entry_by_runtime_id(runtime_id)
        .map(|tab| {
            tab.iter_splits()
                .into_iter()
                .map(runtime_entry_split_info)
                .collect()
        })
        .unwrap_or_default()
}

pub fn runtime_entry_split_infos_by_session_id(session_id: &str) -> Vec<RuntimeEntrySplitInfo> {
    runtime_entry_by_session_id(session_id)
        .map(|tab| {
            tab.iter_splits()
                .into_iter()
                .map(runtime_entry_split_info)
                .collect()
        })
        .unwrap_or_default()
}

pub fn resize_runtime_entry_split(
    runtime_id: RuntimeId,
    split_index: usize,
    delta: isize,
) -> Option<RuntimeEntrySplitInfo> {
    let tab = runtime_entry_by_runtime_id(runtime_id)?;
    tab.resize_split_by(split_index, delta);
    tab.iter_splits()
        .into_iter()
        .find(|split| split.index == split_index)
        .map(runtime_entry_split_info)
}

pub fn root_runtime_entry_summaries() -> Vec<RootRuntimeEntrySummary> {
    let active_runtime_id = root_active_runtime_id();
    root_runtime_entries()
        .into_iter()
        .enumerate()
        .map(|(index, tab)| RootRuntimeEntrySummary {
            runtime_id: tab.runtime_id(),
            index,
            title: runtime_entry_display_title(&tab),
            pane_count: tab.count_panes(),
            is_active: active_runtime_id == Some(tab.runtime_id()),
        })
        .collect()
}

pub fn resize_runtime_entry_split_by_session_id(
    session_id: &str,
    split_index: usize,
    delta: isize,
) -> Option<RuntimeEntrySplitInfo> {
    let tab = runtime_entry_by_session_id(session_id)?;
    tab.resize_split_by(split_index, delta);
    tab.iter_splits()
        .into_iter()
        .find(|split| split.index == split_index)
        .map(runtime_entry_split_info)
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

impl From<HostRuntimeEvent> for HostRuntimeNotification {
    fn from(notification: HostRuntimeEvent) -> Self {
        match notification {
            HostRuntimeEvent::PaneOutput(pane_id) => {
                Self::PaneOutput(SessionTerminalHandle::new(pane_id as u64))
            }
            HostRuntimeEvent::PaneAdded(pane_id) => {
                Self::PaneAdded(SessionTerminalHandle::new(pane_id as u64))
            }
            HostRuntimeEvent::PaneRemoved(pane_id) => {
                Self::PaneRemoved(SessionTerminalHandle::new(pane_id as u64))
            }
            HostRuntimeEvent::WindowInvalidated => Self::WindowInvalidated,
            HostRuntimeEvent::WindowWorkspaceChanged => Self::WindowWorkspaceChanged,
            HostRuntimeEvent::ActiveWorkspaceChanged(client_id) => {
                Self::ActiveWorkspaceChanged(client_id)
            }
            HostRuntimeEvent::Alert { pane_id, alert } => Self::Alert {
                pane_id: SessionTerminalHandle::new(pane_id as u64),
                alert,
            },
            HostRuntimeEvent::Empty => Self::Empty,
            HostRuntimeEvent::AssignClipboard {
                pane_id,
                selection,
                clipboard,
            } => Self::AssignClipboard {
                pane_id: SessionTerminalHandle::new(pane_id as u64),
                selection,
                clipboard,
            },
            HostRuntimeEvent::SaveToDownloads { name, data } => {
                Self::SaveToDownloads { name, data }
            }
            HostRuntimeEvent::TabAddedToWindow { tab_id } => Self::TabAddedToWindow {
                runtime_id: RuntimeId::new(tab_id as u64),
            },
            HostRuntimeEvent::PaneFocused(pane_id) => {
                Self::PaneFocused(SessionTerminalHandle::new(pane_id as u64))
            }
            HostRuntimeEvent::TabResized(tab_id) => Self::TabResized(RuntimeId::new(tab_id as u64)),
            HostRuntimeEvent::TabTitleChanged { tab_id, title } => Self::TabTitleChanged {
                runtime_id: RuntimeId::new(tab_id as u64),
                title,
            },
            HostRuntimeEvent::WindowTitleChanged { title } => Self::WindowTitleChanged { title },
            HostRuntimeEvent::WorkspaceRenamed {
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
pub(crate) enum HostRuntimeEvent {
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

type HostRuntimeSubscriber = Box<dyn Fn(HostRuntimeEvent) -> bool + Send + Sync>;

struct HostRuntimeControlPlane {
    primary_spawn_target: RwLock<Option<Arc<dyn SpawnTarget>>>,
    subscribers: RwLock<HashMap<usize, HostRuntimeSubscriber>>,
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

    fn focused_terminal_handle_for_client(
        &self,
        client_id: &ClientId,
    ) -> Option<SessionTerminalHandle> {
        self.clients
            .read()
            .get(client_id)?
            .focused_terminal_handle()
    }

    fn update_focus_for_client(
        &self,
        client_id: &ClientId,
        pane_id: PaneId,
    ) -> Option<SessionTerminalHandle> {
        let mut clients = self.clients.write();
        let info = clients.get_mut(client_id)?;
        let prior = info.focused_terminal_handle();
        info.update_focused_terminal_handle(SessionTerminalHandle::new(pane_id as u64));
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
        F: Fn(HostRuntimeEvent) -> bool + 'static + Send + Sync,
    {
        let sub_id = SUB_ID.fetch_add(1, Ordering::Relaxed);
        self.subscribers
            .write()
            .insert(sub_id, Box::new(subscriber));
    }

    fn notify(&self, notification: HostRuntimeEvent) {
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

struct HostRuntimeRoot {
    tabs: RwLock<HashMap<TabId, Arc<Tab>>>,
    panes: RwLock<HashMap<PaneId, Arc<dyn Pane>>>,
    window: RwLock<Window>,
    control: HostRuntimeControlPlane,
    config: RwLock<ConfigHandle>,
    num_panes_by_workspace: RwLock<HashMap<String, usize>>,
    main_thread_id: std::thread::ThreadId,
}

fn configured_default_workspace_name(config: &ConfigHandle) -> String {
    config
        .default_workspace
        .as_deref()
        .unwrap_or(DEFAULT_WORKSPACE)
        .to_string()
}

pub(crate) fn switch_to_last_active_tab_when_closing_tab() -> bool {
    current_host_runtime_config().switch_to_last_active_tab_when_closing_tab
}

pub(crate) fn unzoom_on_switch_pane() -> bool {
    current_host_runtime_config().unzoom_on_switch_pane
}

pub fn default_initial_terminal_size() -> TerminalSize {
    current_host_runtime_config().initial_size(0, None)
}

pub(crate) fn current_host_runtime_config() -> ConfigHandle {
    with_host_runtime_root(|root| root.config()).unwrap_or_else(current_config_handle)
}

pub(crate) fn current_host_output_parser_config() -> OutputParserConfig {
    let config = current_host_runtime_config();
    OutputParserConfig {
        buffer_size: config.output_parser_buffer_size,
        coalesce_delay_ms: config.output_parser_coalesce_delay_ms,
    }
}

pub(crate) fn current_host_exit_behavior() -> ExitBehavior {
    current_host_runtime_config().exit_behavior
}

lazy_static::lazy_static! {
    static ref HOST_RUNTIME_ROOT: Mutex<Option<Arc<HostRuntimeRoot>>> = Mutex::new(None);
}

#[cfg(test)]
pub(crate) static HOST_RUNTIME_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl HostRuntimeRoot {
    fn config(&self) -> ConfigHandle {
        self.config.read().clone()
    }

    fn set_config(&self, config: ConfigHandle) {
        *self.config.write() = config;
    }

    fn get_default_workspace(&self) -> String {
        configured_default_workspace_name(&self.config())
    }

    fn is_main_thread(&self) -> bool {
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
        let terminal_handle = SessionTerminalHandle::new(pane_id as u64);

        if prior == Some(terminal_handle) {
            return;
        }
        if let Some(prior_id) = prior.and_then(|handle| usize::try_from(handle.as_u64()).ok()) {
            if let Some(pane) = self.get_pane(prior_id) {
                pane.focus_changed(false);
            }
        }
        if let Some(pane) = self.get_pane(pane_id) {
            pane.focus_changed(true);
        }
    }

    pub(crate) fn register_client(&self, client_id: Arc<ClientId>) {
        self.control.register_client(client_id);
    }

    pub(crate) fn iter_workspaces(&self) -> Vec<String> {
        vec![self.window.read().get_workspace().to_string()]
    }

    pub(crate) fn set_active_workspace_for_client(&self, ident: &Arc<ClientId>, workspace: &str) {
        if self.control.set_workspace_for_client(ident, workspace) {
            self.notify(HostRuntimeEvent::ActiveWorkspaceChanged(ident.clone()));
        }
    }

    pub(crate) fn set_active_workspace(&self, workspace: &str) {
        if let Some(ident) = self.control.active_identity() {
            self.set_active_workspace_for_client(&ident, workspace);
        }
        self.window.write().set_workspace(workspace);
    }

    pub(crate) fn replace_identity(&self, id: Option<Arc<ClientId>>) -> Option<Arc<ClientId>> {
        self.control.replace_identity(id)
    }

    pub(crate) fn subscribe<F>(&self, subscriber: F)
    where
        F: Fn(HostRuntimeEvent) -> bool + 'static + Send + Sync,
    {
        self.control.subscribe(subscriber);
    }

    pub(crate) fn notify(&self, notification: HostRuntimeEvent) {
        self.control.notify(notification);
    }

    pub(crate) fn primary_spawn_target(&self) -> Arc<dyn SpawnTarget> {
        self.control.primary_spawn_target().unwrap()
    }

    pub(crate) fn get_pane(&self, pane_id: PaneId) -> Option<Arc<dyn Pane>> {
        self.panes.read().get(&pane_id).map(Arc::clone)
    }

    pub(crate) fn get_tab(&self, tab_id: TabId) -> Option<Arc<Tab>> {
        self.tabs.read().get(&tab_id).map(Arc::clone)
    }

    pub(crate) fn get_tab_by_chatminal_session_id(&self, session_id: &str) -> Option<Arc<Tab>> {
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

    pub(crate) fn root_window(&self) -> MappedRwLockReadGuard<'_, Window> {
        RwLockReadGuard::map(self.window.read(), |window| window)
    }

    pub(crate) fn root_window_mut(&self) -> MappedRwLockWriteGuard<'_, Window> {
        RwLockWriteGuard::map(self.window.write(), |window| window)
    }

    pub(crate) fn root_active_tab(&self) -> Option<Arc<Tab>> {
        let window = self.root_window();
        window.get_active().map(Arc::clone)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.panes.read().is_empty()
    }

    pub(crate) fn is_workspace_empty(&self, workspace: &str) -> bool {
        *self
            .num_panes_by_workspace
            .read()
            .get(workspace)
            .unwrap_or(&0)
            == 0
    }

    pub(crate) fn iter_panes(&self) -> Vec<Arc<dyn Pane>> {
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
                if pane_id_for_pane(p.pane.as_ref()) == pane_id {
                    tab_id = Some(tab.tab_id());
                    break;
                }
            }
        }
        tab_id
    }

    pub(crate) fn focus_pane_and_containing_tab(&self, pane_id: PaneId) -> anyhow::Result<()> {
        let pane = self
            .get_pane(pane_id)
            .ok_or_else(|| anyhow::anyhow!("pane {pane_id} not found"))?;

        let tab_id = self
            .resolve_pane_id(pane_id)
            .ok_or_else(|| anyhow::anyhow!("can't find {pane_id} in the mux"))?;

        {
            let mut win = self.window.write();
            let tab_idx = win
                .idx_by_id(tab_id)
                .ok_or_else(|| anyhow::anyhow!("tab {tab_id} not in root window"))?;
            win.save_and_then_set_active(tab_idx);
        }

        let tab = self
            .get_tab(tab_id)
            .ok_or_else(|| anyhow::anyhow!("tab {tab_id} not found"))?;
        tab.set_active_pane(&pane);

        Ok(())
    }

    fn register_pane_internal(
        &self,
        pane: &Arc<dyn Pane>,
        install_default_side_effects: bool,
    ) -> Result<bool, Error> {
        let pane_id = pane_id_for_pane(pane.as_ref());
        if self.panes.read().contains_key(&pane_id) {
            return Ok(false);
        }

        if install_default_side_effects {
            let clipboard: Arc<dyn Clipboard> = Arc::new(HostRuntimeClipboard { pane_id });
            pane.set_clipboard(&clipboard);

            let downloader: Arc<dyn DownloadHandler> = Arc::new(HostRuntimeDownloader {});
            pane.set_download_handler(&downloader);
        }

        self.panes.write().insert(pane_id, Arc::clone(pane));
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

        let pane_id = pane_id_for_pane(pane.as_ref());
        start_pane_pty_reader(pane, hooks)?;
        self.recompute_pane_count();
        self.notify(HostRuntimeEvent::PaneAdded(pane_id));
        Ok(())
    }

    pub(crate) fn add_pane_without_default_side_effects(
        &self,
        pane: &Arc<dyn Pane>,
    ) -> Result<(), Error> {
        self.add_pane_internal(pane, false, PtyIoHooks::noop())
    }

    pub(crate) fn add_pane_with_io_hooks(
        &self,
        pane: &Arc<dyn Pane>,
        hooks: PtyIoHooks,
    ) -> Result<(), Error> {
        self.add_pane_internal(pane, false, hooks)
    }

    pub(crate) fn add_pane_with_default_side_effects_and_io_hooks(
        &self,
        pane: &Arc<dyn Pane>,
        hooks: PtyIoHooks,
    ) -> Result<(), Error> {
        self.add_pane_internal(pane, true, hooks)
    }

    pub(crate) fn add_tab_and_active_pane_with_io_hooks(
        &self,
        tab: &Arc<Tab>,
        hooks: PtyIoHooks,
    ) -> Result<(), Error> {
        self.tabs.write().insert(tab.tab_id(), Arc::clone(tab));
        let pane = tab
            .get_active_pane()
            .ok_or_else(|| anyhow!("tab MUST have an active pane"))?;
        self.add_pane_with_default_side_effects_and_io_hooks(&pane, hooks)
    }

    fn remove_pane_internal(&self, pane_id: PaneId) {
        log::debug!("removing pane {}", pane_id);
        let mut changed = false;
        if let Some(pane) = self.panes.write().remove(&pane_id).clone() {
            log::debug!("killing pane {}", pane_id);
            pane.kill();
            self.notify(HostRuntimeEvent::PaneRemoved(pane_id));
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
            pane_ids.push(pane_id_for_pane(pos.pane.as_ref()));
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

    pub(crate) fn prune_dead_windows(&self) {
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
            log::trace!("prune_dead_windows: is_empty, send HostRuntimeEvent::Empty");
            self.notify(HostRuntimeEvent::Empty);
        } else {
            log::trace!("prune_dead_windows: not empty");
        }
    }

    pub(crate) fn attach_tab(&self, tab: &Arc<Tab>) -> anyhow::Result<()> {
        let tab_id = tab.tab_id();
        {
            let mut window = self.root_window_mut();
            window.push(tab);
        }
        self.recompute_pane_count();
        self.notify(HostRuntimeEvent::TabAddedToWindow { tab_id });
        Ok(())
    }

    pub(crate) fn resolve_spawn_target(
        &self,
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
        command_dir.or_else(|| match pane {
            Some(pane) => pane
                .get_current_working_dir(policy)
                .and_then(|url| {
                    percent_decode_str(url.path())
                        .decode_utf8()
                        .ok()
                        .map(|path| path.into_owned())
                })
                .map(|path| {
                    let bytes = path.as_bytes();
                    if bytes.len() > 2 && bytes[0] == b'/' && bytes[2] == b':' {
                        path[1..].to_owned()
                    } else {
                        path
                    }
                }),
            _ => None,
        })
    }

    pub(crate) async fn split_pane(
        &self,
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

        let dims = pane.get_dimensions();

        let size = TerminalSize {
            cols: dims.cols,
            rows: dims.viewport_rows,
            pixel_height: 0,
            pixel_width: 0,
            dpi: dims.dpi,
        };

        Ok((pane, size))
    }

    pub(crate) async fn spawn_tab(
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

struct HostRuntimeClipboard {
    pane_id: PaneId,
}

impl Clipboard for HostRuntimeClipboard {
    fn set_contents(
        &self,
        selection: ClipboardSelection,
        clipboard: Option<String>,
    ) -> anyhow::Result<()> {
        let root = try_host_runtime_root().ok_or_else(|| {
            anyhow::anyhow!("HostRuntimeClipboard::set_contents: no host runtime root?")
        })?;
        root.notify(HostRuntimeEvent::AssignClipboard {
            pane_id: self.pane_id,
            selection,
            clipboard,
        });
        Ok(())
    }
}

struct HostRuntimeDownloader {}

impl engine_term::DownloadHandler for HostRuntimeDownloader {
    fn save_to_downloads(&self, name: Option<String>, data: Vec<u8>) {
        if let Some(root) = try_host_runtime_root() {
            root.notify(HostRuntimeEvent::SaveToDownloads {
                name,
                data: Arc::new(data),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane::{
        CachePolicy, ForEachPaneLogicalLine, LogicalLine, Pane, PaneId, WithPaneLines,
    };
    use crate::renderable::{RenderableDimensions, StableCursorPosition};
    use crate::spawn_target::SpawnTarget;
    use async_trait::async_trait;
    use engine_dynamic::Value;
    use engine_term::color::ColorPalette;
    use engine_term::{KeyCode, KeyModifiers, Line, MouseEvent, StableRowIndex, TerminalSize};
    use parking_lot::{MappedMutexGuard, Mutex};
    use rangeset::RangeSet;
    use std::collections::BTreeMap;
    use std::ops::Range;
    use std::sync::Arc;
    use termwiz::surface::SequenceNo;
    use url::Url;

    struct FakePane {
        id: PaneId,
        size: Mutex<TerminalSize>,
        session_id: Option<String>,
        terminal_instance_id: Option<u64>,
    }

    impl FakePane {
        fn new(id: PaneId, size: TerminalSize) -> Arc<dyn Pane> {
            Arc::new(Self {
                id,
                size: Mutex::new(size),
                session_id: None,
                terminal_instance_id: None,
            })
        }

        fn new_with_session_id(id: PaneId, size: TerminalSize, session_id: &str) -> Arc<dyn Pane> {
            Arc::new(Self {
                id,
                size: Mutex::new(size),
                session_id: Some(session_id.to_string()),
                terminal_instance_id: None,
            })
        }

        fn new_with_metadata(
            id: PaneId,
            size: TerminalSize,
            session_id: Option<&str>,
            terminal_instance_id: Option<u64>,
        ) -> Arc<dyn Pane> {
            Arc::new(Self {
                id,
                size: Mutex::new(size),
                session_id: session_id.map(str::to_string),
                terminal_instance_id,
            })
        }
    }

    impl Pane for FakePane {
        fn terminal_handle(&self) -> SessionTerminalHandle {
            SessionTerminalHandle::new(self.id as u64)
        }

        fn get_cursor_position(&self) -> StableCursorPosition {
            unimplemented!()
        }

        fn get_current_seqno(&self) -> SequenceNo {
            unimplemented!()
        }

        fn get_metadata(&self) -> Value {
            let mut metadata = BTreeMap::new();
            if let Some(session_id) = &self.session_id {
                metadata.insert(
                    Value::String("chatminal_session_id".to_string()),
                    Value::String(session_id.clone()),
                );
            }
            if let Some(terminal_instance_id) = self.terminal_instance_id {
                metadata.insert(
                    Value::String("chatminal_terminal_instance_id".to_string()),
                    Value::U64(terminal_instance_id),
                );
            }
            Value::Object(metadata.into())
        }

        fn get_changed_since(
            &self,
            _lines: Range<StableRowIndex>,
            _seqno: SequenceNo,
        ) -> RangeSet<StableRowIndex> {
            unimplemented!()
        }

        fn get_lines(&self, _lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
            unimplemented!()
        }

        fn with_lines_mut(
            &self,
            _lines: Range<StableRowIndex>,
            _with_lines: &mut dyn WithPaneLines,
        ) {
            unimplemented!()
        }

        fn for_each_logical_line_in_stable_range_mut(
            &self,
            _lines: Range<StableRowIndex>,
            _for_line: &mut dyn ForEachPaneLogicalLine,
        ) {
            unimplemented!()
        }

        fn get_logical_lines(&self, _lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
            unimplemented!()
        }

        fn get_dimensions(&self) -> RenderableDimensions {
            unimplemented!()
        }

        fn get_title(&self) -> String {
            "fake-pane".to_string()
        }

        fn send_paste(&self, _text: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn reader(&self) -> anyhow::Result<Option<Box<dyn std::io::Read + Send>>> {
            Ok(None)
        }

        fn writer(&self) -> MappedMutexGuard<'_, dyn std::io::Write> {
            unimplemented!()
        }

        fn resize(&self, size: TerminalSize) -> anyhow::Result<()> {
            *self.size.lock() = size;
            Ok(())
        }

        fn key_down(&self, _key: KeyCode, _mods: KeyModifiers) -> anyhow::Result<()> {
            unimplemented!()
        }

        fn key_up(&self, _key: KeyCode, _mods: KeyModifiers) -> anyhow::Result<()> {
            unimplemented!()
        }

        fn mouse_event(&self, _event: MouseEvent) -> anyhow::Result<()> {
            unimplemented!()
        }

        fn is_dead(&self) -> bool {
            false
        }

        fn palette(&self) -> ColorPalette {
            unimplemented!()
        }

        fn is_mouse_grabbed(&self) -> bool {
            false
        }

        fn is_alt_screen_active(&self) -> bool {
            false
        }

        fn get_current_working_dir(&self, _policy: CachePolicy) -> Option<Url> {
            None
        }
    }

    struct FakeSpawnTarget {
        pane: Arc<dyn Pane>,
    }

    impl std::fmt::Debug for FakeSpawnTarget {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FakeSpawnTarget").finish_non_exhaustive()
        }
    }

    #[async_trait(?Send)]
    impl SpawnTarget for FakeSpawnTarget {
        async fn spawn_pane(
            &self,
            _size: TerminalSize,
            _command: Option<CommandBuilder>,
            _command_dir: Option<String>,
        ) -> anyhow::Result<Arc<dyn Pane>> {
            Ok(Arc::clone(&self.pane))
        }

        fn spawn_target_name(&self) -> &str {
            "fake-spawn-target"
        }
    }

    #[test]
    fn root_window_info_is_none_without_runtime() {
        let _guard = crate::HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        shutdown_host_runtime();
        assert!(root_window_info().is_none());
    }

    #[test]
    fn create_attached_runtime_entry_for_terminal_updates_root_window_snapshot() {
        let _guard = crate::HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        shutdown_host_runtime();
        let _mux_handle = initialize_host_runtime(None).unwrap();

        let size = TerminalSize {
            rows: 24,
            cols: 80,
            pixel_width: 800,
            pixel_height: 600,
            dpi: 96,
        };

        let first = create_attached_runtime_entry_for_terminal(
            &FakePane::new(101, size),
            size,
            Some("first"),
        )
        .unwrap();
        let second = create_attached_runtime_entry_for_terminal(
            &FakePane::new_with_session_id(102, size, "session-second"),
            size,
            Some("second"),
        )
        .unwrap();

        assert_eq!(first.title, "first");
        assert_eq!(
            first.active_terminal_handle,
            Some(SessionTerminalHandle::new(101))
        );
        assert_eq!(root_active_runtime_id(), Some(first.runtime_id));
        assert!(focus_root_runtime_entry(second.runtime_id));

        let root = root_window_info().unwrap();
        assert_eq!(root.initial_position, None);
        assert_eq!(root.runtime_ids, vec![first.runtime_id, second.runtime_id]);
        assert_eq!(root.active_runtime_id, Some(second.runtime_id));
        assert_eq!(root.last_active_runtime_id, Some(first.runtime_id));
        assert_eq!(root_last_active_runtime_id(), Some(first.runtime_id));
        assert_eq!(root_window_initial_position(), None);
        assert_eq!(
            runtime_id_for_session_id("session-second"),
            Some(second.runtime_id)
        );
        assert_eq!(root_runtime_entry_count(), 2);
        assert_eq!(root_active_runtime_entry_index(), Some(1));
        assert_eq!(root_last_active_runtime_entry_index(), Some(0));
        assert_eq!(root_runtime_id_at_index(0), Some(first.runtime_id));
        assert_eq!(
            root_runtime_entry_info_at_index(1).map(|info| info.runtime_id),
            Some(second.runtime_id)
        );
        assert!(focus_root_runtime_entry_by_session_id("session-second"));
        assert!(focus_root_last_runtime_entry());
        assert_eq!(root_active_runtime_id(), Some(first.runtime_id));
        assert!(focus_root_runtime_entry_relative(1, false));
        assert_eq!(root_active_runtime_id(), Some(second.runtime_id));
        assert!(focus_root_runtime_entry_relative(1, true));
        assert_eq!(root_active_runtime_id(), Some(first.runtime_id));
        assert!(move_root_active_runtime_entry_to_index(1));
        assert_eq!(
            root_runtime_ids(),
            vec![second.runtime_id, first.runtime_id]
        );
        assert_eq!(root_active_runtime_entry_index(), Some(1));

        let active_info = root_active_runtime_entry_info().unwrap();
        assert_eq!(active_info.runtime_id, first.runtime_id);

        let summaries = root_runtime_entry_summaries();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].runtime_id, second.runtime_id);
        assert_eq!(summaries[0].title, "second");
        assert_eq!(summaries[0].pane_count, Some(1));
        assert!(!summaries[0].is_active);
        assert_eq!(summaries[1].runtime_id, first.runtime_id);
        assert_eq!(summaries[1].title, "first");
        assert_eq!(summaries[1].pane_count, Some(1));
        assert!(summaries[1].is_active);

        shutdown_host_runtime();
    }

    #[test]
    fn root_runtime_entry_navigation_helpers_reject_missing_targets() {
        let _guard = crate::HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        shutdown_host_runtime();
        let _mux_handle = initialize_host_runtime(None).unwrap();

        assert_eq!(root_runtime_entry_count(), 0);
        assert_eq!(root_active_runtime_entry_index(), None);
        assert_eq!(root_last_active_runtime_entry_index(), None);
        assert_eq!(root_runtime_id_at_index(0), None);
        assert!(root_runtime_entry_info_at_index(0).is_none());
        assert!(!focus_root_runtime_entry_index(0));
        assert!(!focus_root_last_runtime_entry());
        assert!(!focus_root_runtime_entry_relative(1, true));
        assert!(!move_root_active_runtime_entry_to_index(0));

        shutdown_host_runtime();
    }

    #[test]
    fn pane_registration_and_focus_helpers_work_without_direct_mux_calls() {
        let _guard = crate::HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        shutdown_host_runtime();
        let _mux_handle = initialize_host_runtime(None).unwrap();

        let size = TerminalSize {
            rows: 24,
            cols: 80,
            pixel_width: 800,
            pixel_height: 600,
            dpi: 96,
        };

        let detached = FakePane::new(610, size);
        register_pane(&detached).unwrap();
        assert!(terminal_by_handle(SessionTerminalHandle::new(610)).is_some());
        assert!(remove_terminal_handle(SessionTerminalHandle::new(610)));
        assert!(terminal_by_handle(SessionTerminalHandle::new(610)).is_none());

        let first = create_attached_runtime_entry_for_terminal(
            &FakePane::new(611, size),
            size,
            Some("first"),
        )
        .unwrap();
        let second = create_attached_runtime_entry_for_terminal(
            &FakePane::new(612, size),
            size,
            Some("second"),
        )
        .unwrap();

        assert_eq!(root_active_runtime_id(), Some(first.runtime_id));
        focus_terminal_handle(SessionTerminalHandle::new(612)).unwrap();
        assert_eq!(root_active_runtime_id(), Some(second.runtime_id));
        assert_eq!(
            runtime_entry_active_terminal_handle(second.runtime_id),
            Some(SessionTerminalHandle::new(612))
        );

        shutdown_host_runtime();
    }

    #[test]
    fn spawn_runtime_entry_returns_runtime_info_without_exposing_tab() {
        let _guard = crate::HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        shutdown_host_runtime();

        let size = TerminalSize {
            rows: 24,
            cols: 80,
            pixel_width: 800,
            pixel_height: 600,
            dpi: 96,
        };
        let pane = FakePane::new_with_session_id(301, size, "spawned-session");
        let spawn_target: Arc<dyn SpawnTarget> = Arc::new(FakeSpawnTarget { pane });
        let _mux_handle = initialize_host_runtime(Some(spawn_target)).unwrap();

        let (runtime_entry, pane) =
            smol::block_on(spawn_runtime_entry(None, None, size, None)).unwrap();
        assert_eq!(runtime_entry.session_id.as_deref(), Some("spawned-session"));
        assert_eq!(
            runtime_entry.active_terminal_handle,
            Some(SessionTerminalHandle::new(301))
        );
        assert_eq!(terminal_handle_for_pane(pane.as_ref()).as_u64(), 301);

        shutdown_host_runtime();
    }

    #[test]
    fn terminal_metadata_helpers_resolve_public_ids_and_runtime_ids() {
        let _guard = crate::HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        shutdown_host_runtime();
        let _mux_handle = initialize_host_runtime(None).unwrap();

        let size = TerminalSize {
            rows: 24,
            cols: 80,
            pixel_width: 800,
            pixel_height: 600,
            dpi: 96,
        };
        let runtime_entry = create_attached_runtime_entry_for_terminal(
            &FakePane::new_with_metadata(401, size, Some("session-metadata"), Some(9001)),
            size,
            Some("metadata"),
        )
        .unwrap();

        let pane = terminal_by_terminal_instance_id(9001).unwrap();
        assert_eq!(terminal_handle_for_pane(pane.as_ref()).as_u64(), 401);
        assert_eq!(terminal_instance_id_for_pane(pane.as_ref()), Some(9001));
        assert_eq!(
            terminal_by_public_id(9001)
                .map(|pane| terminal_handle_for_pane(pane.as_ref()).as_u64()),
            Some(401)
        );
        assert_eq!(
            resolve_runtime_id_for_terminal_instance_id(9001),
            Some(runtime_entry.runtime_id)
        );

        shutdown_host_runtime();
    }

    #[test]
    fn runtime_entry_split_helpers_report_and_resize_split_layout() {
        let _guard = crate::HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        shutdown_host_runtime();
        let _mux_handle = initialize_host_runtime(None).unwrap();

        let size = TerminalSize {
            rows: 24,
            cols: 80,
            pixel_width: 800,
            pixel_height: 600,
            dpi: 96,
        };

        let runtime_entry =
            create_attached_runtime_entry_for_terminal(&FakePane::new(201, size), size, None)
                .unwrap();
        let tab = runtime_entry_by_runtime_id(runtime_entry.runtime_id).unwrap();
        let split_request = SplitRequest {
            direction: SplitDirection::Horizontal,
            ..Default::default()
        };
        let split_size = tab.compute_split_size(0, split_request).unwrap();
        tab.split_and_insert(0, split_request, FakePane::new(202, split_size.second))
            .unwrap();

        assert!(resize_runtime_entry(
            runtime_entry.runtime_id,
            TerminalSize {
                rows: 30,
                cols: 100,
                pixel_width: 1000,
                pixel_height: 750,
                dpi: 96,
            }
        ));
        assert_eq!(
            runtime_entry_info_by_runtime_id(runtime_entry.runtime_id)
                .unwrap()
                .size
                .cols,
            100
        );
        assert!(!runtime_entry_can_close_without_prompting(
            runtime_entry.runtime_id,
            CloseReason::Tab
        ));

        let split_infos = runtime_entry_split_infos(runtime_entry.runtime_id);
        assert_eq!(split_infos.len(), 1);
        assert_eq!(split_infos[0].direction, SplitDirection::Horizontal);
        assert_eq!(
            runtime_entry_active_terminal_handle(runtime_entry.runtime_id),
            Some(SessionTerminalHandle::new(202))
        );
        assert!(activate_runtime_entry_terminal_index(
            runtime_entry.runtime_id,
            0
        ));
        assert_eq!(
            runtime_entry_active_terminal_handle(runtime_entry.runtime_id),
            Some(SessionTerminalHandle::new(201))
        );
        assert!(activate_runtime_entry_terminal_direction(
            runtime_entry.runtime_id,
            SessionDirection::Right
        ));
        assert_eq!(
            runtime_entry_active_terminal_handle(runtime_entry.runtime_id),
            Some(SessionTerminalHandle::new(202))
        );

        let resized =
            resize_runtime_entry_split(runtime_entry.runtime_id, split_infos[0].index, 1).unwrap();
        assert_eq!(resized.index, split_infos[0].index);
        assert_eq!(
            runtime_entry_terminal_infos(runtime_entry.runtime_id).len(),
            2
        );

        shutdown_host_runtime();
    }
}
