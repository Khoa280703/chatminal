// Desktop session host: manages session runtime + host leaf lifecycle for one desktop window.
//
// This is the session-native render path (Phase 03+). The host creates `ChatminalSessionPane`
// objects directly from the session engine's core state and builds `ChatminalRenderState`
// from the session_pane map.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use super::session_engine::{
    RuntimeId, SessionEngineShared, SessionRuntimeState, StatefulSessionEngine, TerminalInstanceId,
};
use super::spawn_target::DesktopSpawnTarget;
use chatminal_runtime::{ClientId, RuntimeEntryInfo, RuntimeEntryTerminalInfo};
use config::ConfigHandle;
use config::keyassignment::SessionDirection;
use engine_term::{Clipboard, ClipboardSelection, DownloadHandler, TerminalSize};
use portable_pty::CommandBuilder;

use super::session_pane::ChatminalSessionPane;
use super::{
    FrontendClientHandle, FrontendFocusedPane, FrontendResolvedPane, HostSpawnTargetHandle,
    HostTerminal, LauncherSessionEntry, PRIMARY_HOST_WINDOW_ID, ROOT_HOST_WINDOW_ID, RuntimeWindow,
    configured_default_workspace_name, host_window_exists, overlay_shell,
    publish_runtime_notification_from_any_thread, subscribe_desktop_runtime_notifications,
};
use crate::chatminal_render::{ChatminalRenderPane, ChatminalRenderState};
use crate::chatminal_runtime::{
    SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle,
};
use chatminal_runtime::{
    FocusedPaneBinding, HostRuntimeNotification, RenderableDimensions, RuntimeSessionLaunchSpec,
};

// ---------------------------------------------------------------------------
// Singleton host registry
// ---------------------------------------------------------------------------

static HOST_REGISTRY: OnceLock<Arc<DesktopSessionHost>> = OnceLock::new();

pub(crate) fn get_or_init_session_host(
    shared: Arc<SessionEngineShared>,
    config: ConfigHandle,
) -> Arc<DesktopSessionHost> {
    HOST_REGISTRY
        .get_or_init(|| Arc::new(DesktopSessionHost::new(shared, config)))
        .clone()
}

pub(crate) fn terminal_handle_for_host_terminal(pane: &dyn HostTerminal) -> SessionTerminalHandle {
    pane.terminal_handle()
}

fn host_terminal_handle(pane: &dyn HostTerminal) -> SessionTerminalHandle {
    pane.terminal_handle()
}

struct DesktopClipboardBridge {
    terminal_handle: SessionTerminalHandle,
}

impl Clipboard for DesktopClipboardBridge {
    fn set_contents(
        &self,
        selection: ClipboardSelection,
        clipboard: Option<String>,
    ) -> anyhow::Result<()> {
        publish_runtime_notification_from_any_thread(HostRuntimeNotification::AssignClipboard {
            pane_id: self.terminal_handle,
            selection,
            clipboard,
        });
        Ok(())
    }
}

struct DesktopDownloadBridge;

impl DownloadHandler for DesktopDownloadBridge {
    fn save_to_downloads(&self, name: Option<String>, data: Vec<u8>) {
        publish_runtime_notification_from_any_thread(HostRuntimeNotification::SaveToDownloads {
            name,
            data: Arc::new(data),
        });
    }
}

fn install_desktop_pane_side_effects(pane: &Arc<dyn HostTerminal>) {
    let clipboard: Arc<dyn Clipboard> = Arc::new(DesktopClipboardBridge {
        terminal_handle: host_terminal_handle(pane.as_ref()),
    });
    pane.set_clipboard(&clipboard);

    let downloader: Arc<dyn DownloadHandler> = Arc::new(DesktopDownloadBridge);
    pane.set_download_handler(&downloader);
}

fn host_remove_tab(runtime_id: RuntimeId) {
    if let Some(host) = HOST_REGISTRY.get() {
        if host.window_contains_runtime(runtime_id) {
            host.remove_tab_from_window(runtime_id);
        }
    }
}

fn host_set_workspace_name(name: &str) {
    if let Some(host) = HOST_REGISTRY.get() {
        host.set_workspace_name_value(name);
    }
}

fn host_subscribe<F>(subscriber: F)
where
    F: Fn(HostRuntimeNotification) -> bool + 'static + Send + Sync,
{
    subscribe_desktop_runtime_notifications(subscriber);
}

fn host_active_identity() -> Option<FrontendClientHandle> {
    HOST_REGISTRY
        .get()
        .and_then(|host| host.active_client_value())
}

fn host_resolve_pane_id_value(terminal_handle: SessionTerminalHandle) -> Option<RuntimeId> {
    HOST_REGISTRY
        .get()
        .and_then(|host| host.runtime_id_for_terminal_handle_value(terminal_handle))
}

fn host_resolve_focused_pane_value(client_id: &FrontendClientHandle) -> Option<FocusedPaneBinding> {
    HOST_REGISTRY
        .get()
        .and_then(|host| host.focused_pane_for_client_value(client_id))
}

fn host_focus_root_window_tab(runtime_id: RuntimeId) -> bool {
    HOST_REGISTRY
        .get()
        .is_some_and(|host| host.focus_runtime_in_window(runtime_id))
}

fn host_set_tab_title(runtime_id: RuntimeId, title: &str) {
    if let Some(host) = HOST_REGISTRY.get() {
        host.set_runtime_title_value(runtime_id, title);
    }
}

fn initialize_desktop_session_host(
    config: &ConfigHandle,
    default_workspace_name: Option<&str>,
) -> anyhow::Result<FrontendClientHandle> {
    let desktop_spawn_target =
        HostSpawnTargetHandle::new(Arc::new(DesktopSpawnTarget::new_local()?));
    if let Some(host) = HOST_REGISTRY.get() {
        host.primary_spawn_target
            .lock()
            .unwrap()
            .replace(desktop_spawn_target.clone());
    }

    let client_id = host_active_identity().unwrap_or_else(|| Arc::new(ClientId::new()));
    if let Some(host) = HOST_REGISTRY.get() {
        host.active_client
            .lock()
            .unwrap()
            .replace(client_id.clone());
    }

    let workspace = default_workspace_name
        .map(str::to_string)
        .unwrap_or_else(|| configured_default_workspace_name(config));
    host_set_workspace_name(&workspace);
    let _ = PRIMARY_HOST_WINDOW_ID.set(ROOT_HOST_WINDOW_ID);
    Ok(client_id)
}

fn shutdown_desktop_session_host() {}

fn pane_dims_need_resize(dims: RenderableDimensions, size: TerminalSize) -> bool {
    dims.cols != size.cols
        || dims.viewport_rows != size.rows
        || dims.pixel_width != size.pixel_width
        || dims.pixel_height != size.pixel_height
        || dims.dpi != size.dpi
}

fn runtime_entry_terminal_handle_in_direction(
    infos: &[RuntimeEntryTerminalInfo],
    direction: SessionDirection,
) -> Option<SessionTerminalHandle> {
    let active = infos
        .iter()
        .find(|info| info.is_active)
        .or_else(|| infos.first())?;

    if matches!(direction, SessionDirection::Next | SessionDirection::Prev) {
        let max_index = infos
            .iter()
            .map(|info| info.index)
            .max()
            .unwrap_or(active.index);
        let target_index = match direction {
            SessionDirection::Next => {
                if active.index == max_index {
                    0
                } else {
                    active.index + 1
                }
            }
            SessionDirection::Prev => {
                if active.index == 0 {
                    max_index
                } else {
                    active.index - 1
                }
            }
            SessionDirection::Up
            | SessionDirection::Down
            | SessionDirection::Left
            | SessionDirection::Right => unreachable!(),
        };
        return infos
            .iter()
            .find(|info| info.index == target_index)
            .map(|info| info.terminal_handle);
    }

    let edge_intersects =
        |active_start: usize, active_size: usize, current_start: usize, current_size: usize| {
            let active_end = active_start + active_size;
            let current_end = current_start + current_size;
            active_start < current_end && current_start < active_end
        };

    infos
        .iter()
        .filter(|info| info.terminal_handle != active.terminal_handle)
        .filter(|info| match direction {
            SessionDirection::Right => {
                info.left == active.left + active.width + 1
                    && edge_intersects(active.top, active.height, info.top, info.height)
            }
            SessionDirection::Left => {
                info.left + info.width + 1 == active.left
                    && edge_intersects(active.top, active.height, info.top, info.height)
            }
            SessionDirection::Up => {
                info.top + info.height + 1 == active.top
                    && edge_intersects(active.left, active.width, info.left, info.width)
            }
            SessionDirection::Down => {
                active.top + active.height + 1 == info.top
                    && edge_intersects(active.left, active.width, info.left, info.width)
            }
            SessionDirection::Next | SessionDirection::Prev => unreachable!(),
        })
        .map(|info| info.terminal_handle)
        .next()
}

#[cfg(test)]
pub(crate) fn test_active_frontend_client() -> Option<FrontendClientHandle> {
    host_active_identity()
}

#[cfg(test)]
pub(crate) fn test_active_workspace_for_client(client_id: &FrontendClientHandle) -> String {
    HOST_REGISTRY
        .get()
        .map(|host| host.workspace_for_client_value(client_id))
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) fn build_host_runtime_for_test(
    config: &ConfigHandle,
    default_workspace_name: Option<&str>,
) -> anyhow::Result<()> {
    let runtime = super::EmbeddedRuntime::global().map_err(anyhow::Error::msg)?;
    let _ = get_or_init_session_host(runtime.session_engine_shared(), config.clone());
    let _ = initialize_desktop_session_host(config, default_workspace_name)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn shutdown_host_runtime_for_test() {
    shutdown_desktop_session_host();
}

// ---------------------------------------------------------------------------
// DesktopSessionHost
// ---------------------------------------------------------------------------

pub(crate) struct DesktopSessionHost {
    shared: Arc<SessionEngineShared>,
    config: Mutex<ConfigHandle>,
    window: Mutex<RuntimeWindow>,
    primary_spawn_target: Mutex<Option<HostSpawnTargetHandle>>,
    active_client: Mutex<Option<FrontendClientHandle>>,
    workspace_by_client: Mutex<HashMap<FrontendClientHandle, String>>,
    focused_pane_by_client: Mutex<HashMap<FrontendClientHandle, FocusedPaneBinding>>,
    // terminal_instance_id → pane (for output/input routing)
    panes: Mutex<HashMap<TerminalInstanceId, Arc<ChatminalSessionPane>>>,
    // session_id → pane (1 session = 1 pane invariant)
    session_pane: Mutex<HashMap<String, Arc<ChatminalSessionPane>>>,
    // runtime_id → first-party render snapshot for termwindow compatibility
    runtime_render_state: Mutex<HashMap<RuntimeId, ChatminalRenderState>>,
    // runtime_id → terminal instances owned by that runtime
    runtime_terminal_instances: Mutex<HashMap<RuntimeId, HashSet<TerminalInstanceId>>>,
    // runtime_id → last terminal size confirmed for the live PTY
    runtime_terminal_size: Mutex<HashMap<RuntimeId, TerminalSize>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionHostTerminalBinding {
    pub(crate) session_id: String,
    pub(crate) runtime_id: RuntimeId,
    pub(crate) terminal_instance_id: TerminalInstanceId,
    pub(crate) terminal_handle: SessionTerminalHandle,
}

fn host_terminal_binding(pane: &ChatminalSessionPane) -> SessionHostTerminalBinding {
    SessionHostTerminalBinding {
        session_id: pane.session_id_value().to_string(),
        runtime_id: pane.runtime_id_value(),
        terminal_instance_id: pane.terminal_instance_id_value(),
        terminal_handle: pane.pane_id_value(),
    }
}

impl DesktopSessionHost {
    fn new(shared: Arc<SessionEngineShared>, config: ConfigHandle) -> Self {
        let workspace = configured_default_workspace_name(&config);
        Self {
            shared,
            config: Mutex::new(config),
            window: Mutex::new(RuntimeWindow::new(workspace, None)),
            primary_spawn_target: Mutex::new(None),
            active_client: Mutex::new(None),
            workspace_by_client: Mutex::new(HashMap::new()),
            focused_pane_by_client: Mutex::new(HashMap::new()),
            panes: Mutex::new(HashMap::new()),
            session_pane: Mutex::new(HashMap::new()),
            runtime_render_state: Mutex::new(HashMap::new()),
            runtime_terminal_instances: Mutex::new(HashMap::new()),
            runtime_terminal_size: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn set_config(&self, config: &ConfigHandle) {
        self.config.lock().unwrap().clone_from(config);
    }

    fn active_client_value(&self) -> Option<FrontendClientHandle> {
        self.active_client.lock().unwrap().clone()
    }

    fn workspace_name_value(&self) -> String {
        self.window.lock().unwrap().get_workspace().to_string()
    }

    fn set_workspace_name_value(&self, workspace: &str) {
        self.window.lock().unwrap().set_workspace(workspace);
        if let Some(client_id) = self.active_client_value() {
            self.set_workspace_for_client_value(&client_id, workspace);
        }
    }

    fn workspace_for_client_value(&self, client_id: &FrontendClientHandle) -> String {
        self.workspace_by_client
            .lock()
            .unwrap()
            .get(client_id)
            .cloned()
            .unwrap_or_else(|| self.workspace_name_value())
    }

    fn set_workspace_for_client_value(&self, client_id: &FrontendClientHandle, workspace: &str) {
        self.workspace_by_client
            .lock()
            .unwrap()
            .insert(client_id.clone(), workspace.to_string());
        publish_runtime_notification_from_any_thread(
            HostRuntimeNotification::ActiveWorkspaceChanged(client_id.clone()),
        );
    }

    fn workspace_is_empty_value(&self, workspace: &str) -> bool {
        self.workspace_name_value() != workspace || self.window.lock().unwrap().is_empty()
    }

    fn workspace_names_value(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .workspace_by_client
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        let current = self.workspace_name_value();
        if !names.iter().any(|name| name == &current) {
            names.push(current);
        }
        names.sort();
        names.dedup();
        names
    }

    fn focused_pane_for_client_value(
        &self,
        client_id: &FrontendClientHandle,
    ) -> Option<FocusedPaneBinding> {
        self.focused_pane_by_client
            .lock()
            .unwrap()
            .get(client_id)
            .copied()
    }

    fn runtime_id_for_terminal_handle_value(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<RuntimeId> {
        self.terminal_binding_for_handle_inner(terminal_handle)
            .map(|binding| binding.runtime_id)
    }

    fn attach_runtime_to_window(&self, runtime_id: RuntimeId) {
        let mut window = self.window.lock().unwrap();
        if window
            .iter()
            .any(|existing| existing.runtime_id() == runtime_id)
        {
            return;
        }
        window.push_runtime(runtime_id);
        let last_index = window.len().saturating_sub(1);
        window.save_and_then_set_active(last_index);
        publish_runtime_notification_from_any_thread(HostRuntimeNotification::TabAddedToWindow {
            runtime_id,
        });
    }

    fn remove_tab_from_window(&self, runtime_id: RuntimeId) {
        self.window.lock().unwrap().remove_by_id(runtime_id);
    }

    fn focus_runtime_in_window(&self, runtime_id: RuntimeId) -> bool {
        let mut window = self.window.lock().unwrap();
        let Some(index) = window.iter().position(|tab| tab.runtime_id() == runtime_id) else {
            return false;
        };
        window.save_and_then_set_active(index);
        true
    }

    fn active_terminal_handle_value(&self, runtime_id: RuntimeId) -> Option<SessionTerminalHandle> {
        self.runtime_render_state
            .lock()
            .unwrap()
            .get(&runtime_id)
            .and_then(|state| {
                state
                    .panes
                    .iter()
                    .find(|pane| pane.is_active)
                    .map(|pane| pane.terminal_handle)
            })
    }

    fn set_runtime_title_value(&self, runtime_id: RuntimeId, title: &str) {
        let mut window = self.window.lock().unwrap();
        if let Some(entry) = window.entry_by_runtime_id_mut(runtime_id) {
            entry.set_title(title);
        }
        if window
            .get_active()
            .is_some_and(|entry| entry.runtime_id() == runtime_id)
        {
            window.set_title(title);
        }
    }

    fn engine(&self) -> StatefulSessionEngine {
        StatefulSessionEngine::with_shared(Arc::clone(&self.shared))
    }

    pub(crate) fn root_runtime_entry_infos(&self) -> Vec<RuntimeEntryInfo> {
        let window = self.window.lock().unwrap();
        window
            .iter()
            .filter_map(|entry| self.runtime_entry_info_by_runtime_id(entry.runtime_id()))
            .collect()
    }

    pub(crate) fn runtime_entry_info_by_session_id(
        &self,
        session_id: &str,
    ) -> Option<RuntimeEntryInfo> {
        let runtime_id = self.runtime_id_for_session(session_id)?;
        self.runtime_entry_info_by_runtime_id(runtime_id)
    }

    pub(crate) fn runtime_entry_info_by_runtime_id(
        &self,
        runtime_id: RuntimeId,
    ) -> Option<RuntimeEntryInfo> {
        let render_state = self
            .runtime_render_state
            .lock()
            .unwrap()
            .get(&runtime_id)
            .cloned()?;
        Some(RuntimeEntryInfo {
            runtime_id,
            title: render_state
                .panes
                .first()
                .and_then(|pane| self.pane_for_terminal_handle(pane.terminal_handle))
                .map(|pane| pane.get_title())
                .unwrap_or_default(),
            session_id: self
                .shared
                .core_state()
                .lock()
                .ok()?
                .runtime(runtime_id)
                .map(|record| record.session_id.clone()),
            size: render_state.terminal_size,
            active_terminal_handle: render_state
                .panes
                .iter()
                .find(|pane| pane.is_active)
                .map(|pane| pane.terminal_handle),
            active_terminal_instance_id: render_state
                .active_terminal_instance_id
                .map(|id| id.as_u64()),
        })
    }

    // -----------------------------------------------------------------
    // Runtime lifecycle
    // -----------------------------------------------------------------

    /// Ensure a session runtime attachment exists: focus it if it already exists, or
    /// spawn a new one. Refreshes desktop-local render state for termwindow.
    ///
    /// Returns the session runtime state (contains layout snapshot), or `None`
    /// on failure.
    pub(crate) fn ensure_runtime(
        &self,
        session_id: &str,
        generation: u64,
        command: CommandBuilder,
        size: TerminalSize,
    ) -> Option<SessionRuntimeState> {
        self.ensure_runtime_inner(session_id, generation, command, size, true)
    }

    fn ensure_runtime_inner(
        &self,
        session_id: &str,
        generation: u64,
        command: CommandBuilder,
        size: TerminalSize,
        activate: bool,
    ) -> Option<SessionRuntimeState> {
        let initial_scrollback =
            crate::chatminal_runtime::read_session_restore_snapshot(session_id)
                .ok()
                .map(|snapshot| snapshot.content)
                .filter(|content| !content.is_empty());
        let state = self
            .engine()
            .ensure_session_runtime_native(
                session_id,
                generation,
                command,
                size,
                initial_scrollback,
            )
            .map_err(|err| {
                log::error!("session host: ensure runtime failed for {session_id}: {err}");
            })
            .ok()?;

        self.runtime_terminal_size
            .lock()
            .unwrap()
            .insert(state.snapshot.runtime_id, size);
        self.sync_render_state_for_runtime(&state);
        let _ = activate;
        Some(state)
    }

    /// Focus an already-existing session runtime attachment. Returns the runtime state or
    /// `None` if the runtime is not found in core state.
    pub(crate) fn focus_runtime(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Option<SessionRuntimeState> {
        let state = self.engine().focus_runtime_native(session_id, runtime_id)?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    /// Hydrate render objects for an existing runtime without changing focus.
    pub(crate) fn hydrate_runtime(&self, runtime_id: RuntimeId) -> Option<SessionRuntimeState> {
        let state = self.engine().snapshot_runtime_from_core(runtime_id)?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    pub(crate) fn remember_runtime_terminal_size(&self, runtime_id: RuntimeId, size: TerminalSize) {
        self.runtime_terminal_size
            .lock()
            .unwrap()
            .insert(runtime_id, size);
    }

    /// Focus a specific leaf. Returns the updated runtime snapshot.
    pub(crate) fn focus_terminal_instance(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<SessionRuntimeState> {
        let state = self.engine().focus_terminal_instance_native(
            session_id,
            runtime_id,
            terminal_instance_id,
        )?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    pub(crate) fn resize_runtime(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        size: TerminalSize,
    ) -> Option<SessionRuntimeState> {
        let pane = self.pane_for_session(session_id)?;
        // `runtime_terminal_size` is a desired-size cache used by render/layout plumbing.
        // It may be updated before the live pane has actually been resized, so it cannot be
        // the source of truth for deciding whether a PTY/terminal resize is needed.
        let dims = pane.get_dimensions();
        let needs_resize = pane_dims_need_resize(dims, size);
        if needs_resize {
            log::warn!(
                "resize_runtime: session={} runtime={} from cols={} rows={} px={}x{} dpi={} to cols={} rows={} px={}x{} dpi={}",
                session_id,
                runtime_id.as_u64(),
                dims.cols,
                dims.viewport_rows,
                dims.pixel_width,
                dims.pixel_height,
                dims.dpi,
                size.cols,
                size.rows,
                size.pixel_width,
                size.pixel_height,
                size.dpi
            );
            if let Err(err) = pane.resize(size) {
                log::error!("session host: resize runtime failed for {session_id}: {err}");
                return None;
            }
        }
        self.runtime_terminal_size
            .lock()
            .unwrap()
            .insert(runtime_id, size);
        let state = self.engine().snapshot_runtime_from_core(runtime_id)?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    /// Close a session runtime attachment and unregister all associated panes/tabs.
    pub(crate) fn close_runtime(&self, session_id: &str, runtime_id: RuntimeId) {
        let _ = session_id;
        self.engine().close_runtime_native(session_id, runtime_id);
        self.remove_runtime_resources(runtime_id);
    }

    pub(crate) fn render_state_for_runtime(
        &self,
        runtime_id: RuntimeId,
    ) -> Option<ChatminalRenderState> {
        self.runtime_render_state
            .lock()
            .unwrap()
            .get(&runtime_id)
            .cloned()
    }

    pub(crate) fn reconcile_visible_sessions(&self, visible_session_ids: &HashSet<String>) {
        let stale_session_ids: Vec<String> = self
            .session_pane
            .lock()
            .unwrap()
            .keys()
            .filter(|session_id| !visible_session_ids.contains(*session_id))
            .cloned()
            .collect();

        if stale_session_ids.is_empty() {
            return;
        }

        let mut panes = self.panes.lock().unwrap();
        let mut session_pane = self.session_pane.lock().unwrap();
        let mut runtime_render_state = self.runtime_render_state.lock().unwrap();

        for session_id in stale_session_ids {
            let stale_terminal_instance_id = session_pane
                .get(&session_id)
                .map(|pane| pane.terminal_instance_id_value());
            if let Some(runtime_id) = self.runtime_id_for_session(&session_id) {
                host_remove_tab(runtime_id);
            }
            if let Some(pane) = session_pane.remove(&session_id) {
                runtime_render_state.remove(&pane.runtime_id_value());
            }
            if let Some(terminal_instance_id) = stale_terminal_instance_id {
                panes.remove(&terminal_instance_id);
            }
        }
    }

    // -----------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------

    /// Create or update panes for each leaf in the state, then sync local window/runtime
    /// bookkeeping for the current active pane.
    fn sync_render_state_for_runtime(&self, state: &SessionRuntimeState) {
        let Some(layout) = &state.layout else {
            return;
        };
        let runtime_id = state.snapshot.runtime_id;
        let session_id = &state.snapshot.session_id;
        let pane_size = self
            .runtime_terminal_size
            .lock()
            .unwrap()
            .get(&runtime_id)
            .copied()
            .unwrap_or_else(engine_terminal_size_default);

        let mut panes_guard = self.panes.lock().unwrap();

        // Create panes for any leaf that doesn't have one yet.
        // Keep the whole creation path under a single panes mutex scope; do not
        // re-enter self.panes via helper calls from inside this loop.
        for leaf_snapshot in &layout.leaves {
            let terminal_instance_id = leaf_snapshot.terminal_instance_id;
            if !panes_guard.contains_key(&terminal_instance_id) {
                match ChatminalSessionPane::new(
                    Arc::clone(&self.shared),
                    session_id.clone(),
                    runtime_id,
                    terminal_instance_id,
                    pane_size,
                    self.config.lock().unwrap().clone(),
                ) {
                    Ok(pane) => {
                        install_desktop_pane_side_effects(&(pane.clone() as Arc<dyn HostTerminal>));
                        panes_guard.insert(terminal_instance_id, pane);
                    }
                    Err(err) => {
                        log::error!(
                            "session host: create pane for leaf {terminal_instance_id}: {err}"
                        );
                    }
                }
            }
        }

        // Capture active pane before dropping panes_guard (avoids double-lock)
        let active_pane_for_session = panes_guard
            .get(&layout.active_terminal_instance_id)
            .cloned();

        // Defer stale-pane cleanup until after we have refreshed the local
        // window/runtime binding. Otherwise bootstrap cleanup can race the
        // window registry and leave the runtime detached for one frame.
        let live_terminal_instance_ids: HashSet<TerminalInstanceId> = layout
            .leaves
            .iter()
            .map(|l| l.terminal_instance_id)
            .collect();
        let stale: Vec<TerminalInstanceId> = self
            .runtime_terminal_instances
            .lock()
            .unwrap()
            .insert(runtime_id, live_terminal_instance_ids.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|id| !live_terminal_instance_ids.contains(id))
            .collect();

        drop(panes_guard);

        // Sync session_pane index: 1 session = 1 pane invariant
        if let Some(active_pane) = active_pane_for_session {
            let mut session_pane_guard = self.session_pane.lock().unwrap();
            if session_pane_guard
                .get(session_id.as_str())
                .is_some_and(|existing| existing.pane_id_value() != active_pane.pane_id_value())
            {
                log::debug!("session host: replacing stale pane mapping for session {session_id}");
            }
            session_pane_guard.insert(session_id.to_string(), active_pane.clone());
            self.sync_runtime_window_entry(session_id, runtime_id, &active_pane);

            let terminal_size = self
                .runtime_terminal_size
                .lock()
                .unwrap()
                .get(&runtime_id)
                .copied()
                .unwrap_or_else(|| terminal_size_from_dims(active_pane.get_dimensions()));
            let render_state =
                self.build_render_state(runtime_id, layout, active_pane.as_ref(), terminal_size);
            self.runtime_render_state
                .lock()
                .unwrap()
                .insert(runtime_id, render_state);
        }

        if !stale.is_empty() {
            let mut panes_guard = self.panes.lock().unwrap();
            for stale_terminal_instance_id in stale {
                panes_guard.remove(&stale_terminal_instance_id);
            }
        }
    }

    fn build_render_state(
        &self,
        runtime_id: RuntimeId,
        layout: &chatminal_runtime::execution::SessionLayoutSnapshot,
        active_pane: &ChatminalSessionPane,
        terminal_size: TerminalSize,
    ) -> ChatminalRenderState {
        let render_target = SessionRenderTargetSnapshot {
            render_target_id: SessionRenderTargetId::new(runtime_id.as_u64()),
            runtime_id,
            active_terminal_instance_id: Some(layout.active_terminal_instance_id),
        };

        ChatminalRenderState {
            render_target,
            terminal_size,
            active_terminal_instance_id: Some(layout.active_terminal_instance_id),
            panes: vec![ChatminalRenderPane {
                terminal_handle: host_terminal_handle(active_pane),
                terminal_instance_id: layout.active_terminal_instance_id,
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 0,
                top: 0,
                width: terminal_size.cols as usize,
                pixel_width: 0,
                height: terminal_size.rows as usize,
                pixel_height: 0,
            }],
            splits: vec![],
        }
    }

    fn sync_runtime_window_entry(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        active_pane: &Arc<ChatminalSessionPane>,
    ) {
        if !host_window_exists() {
            log::debug!(
                "session host: deferring runtime window sync for session {session_id}; root window not ready yet",
            );
            return;
        }

        let title = active_pane.get_title();
        self.attach_runtime_to_window(runtime_id);
        host_set_tab_title(runtime_id, &title);
        let _ = host_focus_root_window_tab(runtime_id);
    }

    /// Remove panes and render snapshot for a runtime from all registries.
    fn remove_runtime_resources(&self, runtime_id: RuntimeId) {
        self.runtime_render_state
            .lock()
            .unwrap()
            .remove(&runtime_id);
        self.runtime_terminal_size
            .lock()
            .unwrap()
            .remove(&runtime_id);
        let stale_terminal_instance_ids: Vec<TerminalInstanceId> = self
            .runtime_terminal_instances
            .lock()
            .unwrap()
            .remove(&runtime_id)
            .unwrap_or_default()
            .into_iter()
            .collect();
        let mut panes = self.panes.lock().unwrap();
        for terminal_instance_id in stale_terminal_instance_ids {
            if let Some(pane) = panes.remove(&terminal_instance_id) {
                // Also remove from session_pane index
                let session_id = pane.session_id_value().to_string();
                self.session_pane.lock().unwrap().remove(&session_id);
            }
        }
    }

    /// Look up the pane for a session (1 session = 1 pane invariant).
    pub(crate) fn pane_for_session(&self, session_id: &str) -> Option<Arc<ChatminalSessionPane>> {
        self.session_pane.lock().unwrap().get(session_id).cloned()
    }

    fn terminal_binding_for_handle_inner(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<SessionHostTerminalBinding> {
        let public_id = terminal_handle.as_u64();
        let pane = self.find_registered_pane(|pane| {
            pane.pane_id_value().as_u64() == public_id
                || pane.terminal_instance_id_value().as_u64() == public_id
        })?;
        Some(host_terminal_binding(&pane))
    }

    fn find_registered_pane(
        &self,
        predicate: impl Fn(&ChatminalSessionPane) -> bool,
    ) -> Option<Arc<ChatminalSessionPane>> {
        self.panes
            .lock()
            .unwrap()
            .values()
            .find(|pane| predicate(pane))
            .cloned()
    }

    pub(crate) fn pane_for_terminal_handle(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<Arc<dyn HostTerminal>> {
        self.find_registered_pane(|pane| host_terminal_handle(pane) == terminal_handle)
            .map(|pane| pane as Arc<dyn HostTerminal>)
    }

    pub(crate) fn pane_for_public_id(&self, public_id: u64) -> Option<Arc<dyn HostTerminal>> {
        self.find_registered_pane(|pane| {
            pane.pane_id_value().as_u64() == public_id
                || pane.terminal_instance_id_value().as_u64() == public_id
        })
        .map(|pane| pane as Arc<dyn HostTerminal>)
    }

    pub(crate) fn terminal_binding_for_public_id(
        &self,
        public_id: u64,
    ) -> Option<SessionHostTerminalBinding> {
        self.find_registered_pane(|pane| {
            pane.pane_id_value().as_u64() == public_id
                || pane.terminal_instance_id_value().as_u64() == public_id
        })
        .map(|pane| host_terminal_binding(&pane))
    }

    fn remove_registered_pane(&self, terminal_handle: SessionTerminalHandle) -> bool {
        let Some(pane) =
            self.find_registered_pane(|pane| host_terminal_handle(pane) == terminal_handle)
        else {
            return false;
        };

        let session_id = pane.session_id_value().to_string();
        let runtime_id = pane.runtime_id_value();
        let terminal_instance_id = pane.terminal_instance_id_value();

        self.panes.lock().unwrap().remove(&terminal_instance_id);

        let removed_session_mapping = {
            let mut session_pane = self.session_pane.lock().unwrap();
            let matches = session_pane
                .get(&session_id)
                .is_some_and(|mapped| host_terminal_handle(mapped.as_ref()) == terminal_handle);
            if matches {
                session_pane.remove(&session_id);
            }
            matches
        };

        let runtime_became_empty = {
            let mut runtime_terminal_instances = self.runtime_terminal_instances.lock().unwrap();
            let remove_runtime_entry = match runtime_terminal_instances.get_mut(&runtime_id) {
                Some(terminal_instances) => {
                    terminal_instances.remove(&terminal_instance_id);
                    terminal_instances.is_empty()
                }
                None => false,
            };
            if remove_runtime_entry {
                runtime_terminal_instances.remove(&runtime_id);
            }
            remove_runtime_entry
        };

        if runtime_became_empty {
            self.runtime_render_state
                .lock()
                .unwrap()
                .remove(&runtime_id);
            self.runtime_terminal_size
                .lock()
                .unwrap()
                .remove(&runtime_id);
        }

        if removed_session_mapping {
            host_remove_tab(runtime_id);
            return true;
        }

        true
    }

    pub(crate) fn host_workspace_name(&self) -> String {
        self.workspace_name_value()
    }

    pub(crate) fn active_frontend_client(&self) -> Option<FrontendClientHandle> {
        self.active_client_value()
    }

    pub(crate) fn root_active_runtime_id(&self) -> Option<RuntimeId> {
        self.with_window(|window| window.get_active().map(|entry| entry.runtime_id()))
            .flatten()
    }

    pub(crate) fn runtime_available(&self) -> bool {
        true // session host only exists when runtime is initialized
    }

    pub(crate) fn iter_all_panes(&self) -> Vec<Arc<dyn HostTerminal>> {
        self.panes
            .lock()
            .unwrap()
            .values()
            .cloned()
            .map(|pane| pane as Arc<dyn HostTerminal>)
            .collect()
    }

    pub(crate) fn subscribe_notifications<F>(&self, subscriber: F)
    where
        F: Fn(HostRuntimeNotification) -> bool + 'static + Send + Sync,
    {
        host_subscribe(subscriber);
    }

    pub(crate) fn active_workspace_for_client(&self, client_id: &FrontendClientHandle) -> String {
        self.workspace_for_client_value(client_id)
    }

    pub(crate) fn set_active_workspace_for_client(
        &self,
        client_id: &FrontendClientHandle,
        workspace: &str,
    ) {
        if self.active_client_value().as_ref() == Some(client_id) {
            self.set_workspace_name_value(workspace);
            return;
        }
        self.set_workspace_for_client_value(client_id, workspace);
    }

    pub(crate) fn workspace_is_empty(&self, workspace: &str) -> bool {
        self.workspace_is_empty_value(workspace)
    }

    pub(crate) fn workspace_names(&self) -> Vec<String> {
        self.workspace_names_value()
    }

    pub(crate) fn root_window_workspace_name(&self) -> Option<String> {
        Some(self.workspace_name_value())
    }

    pub(crate) fn set_root_window_workspace_name(&self, workspace: &str) -> bool {
        self.set_workspace_name_value(workspace);
        true
    }

    pub(crate) fn set_active_workspace_name(&self, workspace: &str) -> bool {
        self.set_workspace_name_value(workspace);
        true
    }

    pub(crate) fn rename_workspace(&self, old_workspace: &str, new_workspace: &str) -> bool {
        let mut changed = false;
        if self.workspace_name_value() == old_workspace {
            self.set_workspace_name_value(new_workspace);
            changed = true;
        }
        {
            let mut workspace_by_client = self.workspace_by_client.lock().unwrap();
            for workspace in workspace_by_client.values_mut() {
                if workspace == old_workspace {
                    *workspace = new_workspace.to_string();
                    changed = true;
                }
            }
        }
        if changed {
            publish_runtime_notification_from_any_thread(
                HostRuntimeNotification::WorkspaceRenamed {
                    old_workspace: old_workspace.to_string(),
                    new_workspace: new_workspace.to_string(),
                },
            );
        }
        changed
    }

    pub(crate) fn root_window_title(&self) -> Option<String> {
        Some(self.window.lock().unwrap().get_title().to_string())
    }

    pub(crate) fn set_root_window_title(&self, title: &str) -> bool {
        self.window.lock().unwrap().set_title(title);
        true
    }

    pub(crate) fn root_window_spawn_context(
        &self,
    ) -> (TerminalSize, Option<SessionTerminalHandle>) {
        let runtime_id = self.root_active_runtime_id();
        let size = runtime_id
            .and_then(|runtime_id| {
                self.runtime_entry_info_by_runtime_id(runtime_id)
                    .map(|info| info.size)
            })
            .unwrap_or_else(engine_terminal_size_default);
        let sibling =
            runtime_id.and_then(|runtime_id| self.active_terminal_handle_value(runtime_id));
        (size, sibling)
    }

    pub(crate) fn set_runtime_entry_title_by_session_id(
        &self,
        session_id: &str,
        title: &str,
    ) -> bool {
        let Some(runtime_id) = self.runtime_id_for_session(session_id) else {
            return false;
        };
        self.set_runtime_title_value(runtime_id, title);
        true
    }

    pub(crate) fn runtime_entry_exists_for_session(&self, session_id: &str) -> bool {
        self.runtime_id_for_session(session_id).is_some()
    }

    pub(crate) fn runtime_entry_terminal_handles_by_session_id(
        &self,
        session_id: &str,
    ) -> Vec<SessionTerminalHandle> {
        let Some(runtime_id) = self.runtime_id_for_session(session_id) else {
            return Vec::new();
        };
        self.runtime_entry_terminal_infos(runtime_id)
            .into_iter()
            .map(|info| info.terminal_handle)
            .collect()
    }

    pub(crate) fn runtime_entry_terminal_handle_in_direction_by_session_id(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> Option<SessionTerminalHandle> {
        let infos = self.runtime_entry_terminal_infos_by_session_id(session_id);
        runtime_entry_terminal_handle_in_direction(&infos, direction)
    }

    pub(crate) fn runtime_entry_terminal_infos_by_session_id(
        &self,
        session_id: &str,
    ) -> Vec<RuntimeEntryTerminalInfo> {
        let Some(runtime_id) = self.runtime_id_for_session(session_id) else {
            return Vec::new();
        };
        self.runtime_entry_terminal_infos(runtime_id)
    }

    pub(crate) fn resolve_runtime_id_for_terminal_handle(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<RuntimeId> {
        self.runtime_id_for_terminal_handle_value(terminal_handle)
    }

    pub(crate) fn focus_root_runtime_entry(&self, runtime_id: RuntimeId) -> bool {
        host_focus_root_window_tab(runtime_id)
    }

    pub(crate) fn set_runtime_entry_active_terminal(
        &self,
        runtime_id: RuntimeId,
        terminal_handle: SessionTerminalHandle,
    ) -> bool {
        if let Some(binding) = self.terminal_binding_for_handle_inner(terminal_handle) {
            if binding.runtime_id == runtime_id {
                let _ = self.focus_root_runtime_entry(runtime_id);
                if self
                    .focus_terminal_instance(
                        &binding.session_id,
                        runtime_id,
                        binding.terminal_instance_id,
                    )
                    .is_some()
                {
                    self.record_focus_for_current_identity(terminal_handle);
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn runtime_entry_terminal_infos(
        &self,
        runtime_id: RuntimeId,
    ) -> Vec<RuntimeEntryTerminalInfo> {
        let session_id = self.shared.core_state().lock().ok().and_then(|state| {
            state
                .runtime(runtime_id)
                .map(|record| record.session_id.clone())
        });
        self.runtime_render_state
            .lock()
            .unwrap()
            .get(&runtime_id)
            .map(|render_state| {
                render_state
                    .panes
                    .iter()
                    .map(|pane| RuntimeEntryTerminalInfo {
                        index: pane.index,
                        is_active: pane.is_active,
                        is_zoomed: pane.is_zoomed,
                        left: pane.left,
                        top: pane.top,
                        width: pane.width,
                        pixel_width: pane.pixel_width,
                        height: pane.height,
                        pixel_height: pane.pixel_height,
                        terminal_handle: pane.terminal_handle,
                        session_id: session_id.clone(),
                        terminal_instance_id: Some(pane.terminal_instance_id.as_u64()),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn resize_runtime_entry(&self, _runtime_id: RuntimeId, _size: TerminalSize) -> bool {
        false // session-native splits removed; multi-pane resize is a no-op
    }

    pub(crate) fn resize_runtime_entry_split(
        &self,
        _runtime_id: RuntimeId,
        _split_index: usize,
        _delta: isize,
    ) -> Option<chatminal_runtime::RuntimeEntrySplitInfo> {
        None // session-native splits removed
    }

    pub(crate) fn with_window<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&RuntimeWindow) -> R,
    {
        let window = self.window.lock().unwrap();
        Some(func(&window))
    }

    pub(crate) fn window_exists(&self) -> bool {
        true
    }

    pub(crate) fn workspace_has_windows(&self, name: &str) -> bool {
        self.workspace_name_value() == name && !self.window.lock().unwrap().is_empty()
    }

    pub(crate) fn has_panes_in_workspace(&self, workspace: Option<&str>) -> bool {
        if self.panes.lock().unwrap().is_empty() {
            return false;
        }
        let Some(workspace) = workspace else {
            return true;
        };
        self.workspace_has_windows(workspace)
    }

    pub(crate) fn window_contains_runtime(&self, runtime_id: RuntimeId) -> bool {
        self.window
            .lock()
            .unwrap()
            .iter()
            .any(|entry| entry.runtime_id() == runtime_id)
    }

    pub(crate) fn host_window_contains_render_scope(&self, render_scope_id: u64) -> bool {
        self.window_contains_runtime(RuntimeId::new(render_scope_id))
    }

    pub(crate) fn remove_terminal_handle(&self, terminal_handle: SessionTerminalHandle) {
        self.remove_registered_pane(terminal_handle);
    }

    pub(crate) fn remove_runtime_entry_scope(&self, render_scope_id: u64) {
        let runtime_id = RuntimeId::new(render_scope_id);
        self.remove_tab_from_window(runtime_id);
    }

    pub(crate) fn record_focus_for_current_identity(&self, terminal_handle: SessionTerminalHandle) {
        if let Some(client_id) = self.active_client_value() {
            if let Some(runtime_id) = self.resolve_runtime_id_for_terminal_handle(terminal_handle) {
                self.focused_pane_by_client.lock().unwrap().insert(
                    client_id,
                    FocusedPaneBinding::new(runtime_id, terminal_handle),
                );
            }
        }
    }

    pub(crate) fn record_input_for_current_identity(&self) {
        let _ = self.active_client_value();
    }

    pub(crate) fn host_window_initial_position(&self) -> Option<config::GuiPosition> {
        self.with_window(|window| window.get_initial_position().clone())
            .flatten()
    }

    pub(crate) fn resolved_window_title(&self) -> Option<String> {
        self.root_window_title()
    }

    pub(crate) fn launcher_sessions(&self) -> Vec<LauncherSessionEntry> {
        self.with_window(|window| {
            window
                .iter()
                .enumerate()
                .map(|(tab_idx, entry)| {
                    let title = if entry.title().is_empty() {
                        self.runtime_entry_info_by_runtime_id(entry.runtime_id())
                            .map(|info| info.title)
                            .unwrap_or_default()
                    } else {
                        entry.title().to_string()
                    };
                    LauncherSessionEntry {
                        title,
                        tab_idx,
                        pane_count: self
                            .render_state_for_runtime(entry.runtime_id())
                            .map(|state| state.panes.len()),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
    }

    pub(crate) fn overlay_pane_layouts_by_id(
        &self,
        render_scope_id: u64,
    ) -> Vec<overlay_shell::OverlayPaneLayout> {
        self.runtime_entry_terminal_infos(RuntimeId::new(render_scope_id))
            .into_iter()
            .filter_map(|layout| {
                Some(overlay_shell::OverlayPaneLayout {
                    index: layout.index,
                    is_active: layout.is_active,
                    is_zoomed: layout.is_zoomed,
                    left: layout.left,
                    top: layout.top,
                    width: layout.width,
                    pixel_width: layout.pixel_width,
                    height: layout.height,
                    pixel_height: layout.pixel_height,
                    pane: self.pane_for_terminal_handle(layout.terminal_handle)?,
                })
            })
            .collect()
    }

    pub(crate) fn resize_render_scope(&self, render_scope_id: u64, size: TerminalSize) -> bool {
        self.resize_runtime_entry(RuntimeId::new(render_scope_id), size)
    }

    pub(crate) fn resize_render_scope_split(
        &self,
        render_scope_id: u64,
        split_index: usize,
        delta: isize,
    ) -> Option<overlay_shell::OverlaySplitLayout> {
        self.resize_runtime_entry_split(RuntimeId::new(render_scope_id), split_index, delta)
            .map(|split| overlay_shell::OverlaySplitLayout {
                index: split.index,
                direction: split.direction,
                left: split.left,
                top: split.top,
                size: split.size,
            })
    }

    pub(crate) fn activate_runtime_entry(&self, render_scope_id: u64) -> anyhow::Result<()> {
        let runtime_id = RuntimeId::new(render_scope_id);
        self.focus_root_runtime_entry(runtime_id)
            .then_some(())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "runtime entry {} not attached to root window",
                    runtime_id.as_u64()
                )
            })
    }

    pub(crate) fn resolve_public_pane_fallback(
        &self,
        host_terminal_handle: u64,
        terminal_instance_id: u64,
    ) -> Option<Arc<dyn HostTerminal>> {
        self.pane_for_public_id(host_terminal_handle)
            .or_else(|| self.pane_for_public_id(terminal_instance_id))
    }

    pub(crate) fn frontend_resolve_pane_fallback(
        &self,
        pane_id: SessionTerminalHandle,
    ) -> Option<FrontendResolvedPane> {
        if let Some(binding) = self.terminal_binding_for_public_id(pane_id.as_u64()) {
            return Some(FrontendResolvedPane {
                runtime_id: binding.runtime_id,
            });
        }
        Some(FrontendResolvedPane {
            runtime_id: host_resolve_pane_id_value(pane_id)?,
        })
    }

    pub(crate) fn frontend_resolve_focused_pane_fallback(
        &self,
        client_id: &FrontendClientHandle,
    ) -> Option<FrontendFocusedPane> {
        let resolved = self
            .focused_pane_for_client_value(client_id)
            .map(|binding| FrontendFocusedPane {
                runtime_id: binding.runtime_id(),
                terminal_handle: binding.terminal_handle(),
            })
            .or_else(|| {
                host_resolve_focused_pane_value(client_id).map(|binding| FrontendFocusedPane {
                    runtime_id: binding.runtime_id(),
                    terminal_handle: binding.terminal_handle(),
                })
            })?;
        if let Some(binding) =
            self.terminal_binding_for_public_id(resolved.terminal_handle.as_u64())
        {
            return Some(FrontendFocusedPane {
                runtime_id: binding.runtime_id,
                terminal_handle: binding.terminal_handle,
            });
        }
        Some(resolved)
    }

    pub(crate) async fn spawn_local_shell_runner(&self) -> anyhow::Result<Arc<dyn HostTerminal>> {
        self.primary_spawn_target()
            .spawn(TerminalSize::default(), None, None)
            .await
            .map(|spawned| spawned.pane)
    }

    pub(crate) async fn spawn_desktop_terminal(
        &self,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        size: TerminalSize,
        _current_terminal_handle: Option<SessionTerminalHandle>,
        workspace: String,
    ) -> anyhow::Result<Arc<dyn HostTerminal>> {
        host_set_workspace_name(&workspace);
        self.primary_spawn_target()
            .spawn(size, command, command_dir)
            .await
            .map(|spawned| spawned.pane)
    }

    pub(crate) fn set_primary_spawn_target(&self, spawn_target: &HostSpawnTargetHandle) {
        self.primary_spawn_target
            .lock()
            .unwrap()
            .replace(spawn_target.clone());
    }

    pub(crate) fn primary_spawn_target(&self) -> HostSpawnTargetHandle {
        self.primary_spawn_target
            .lock()
            .unwrap()
            .clone()
            .expect("desktop primary spawn target")
    }

    pub(crate) fn build_initial_host_runtime(
        &self,
        config: &ConfigHandle,
        default_workspace_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let _ = initialize_desktop_session_host(config, default_workspace_name)?;
        Ok(())
    }

    pub(crate) fn shutdown_host_runtime(&self) {
        shutdown_desktop_session_host();
    }

    pub(crate) fn create_serial_spawn_target(
        &self,
        serial_target: config::SerialTarget,
    ) -> anyhow::Result<HostSpawnTargetHandle> {
        Ok(HostSpawnTargetHandle::new(Arc::new(
            DesktopSpawnTarget::new_serial(serial_target)?,
        )))
    }
}

impl std::fmt::Debug for DesktopSessionHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopSessionHost").finish_non_exhaustive()
    }
}

impl DesktopSessionHost {
    pub(crate) fn runtime_id_for_session(&self, session_id: &str) -> Option<RuntimeId> {
        self.shared
            .core_state()
            .lock()
            .ok()?
            .runtime_id_for_session(session_id)
    }

    pub(crate) fn ensure_session_runtime(
        &self,
        launch: &RuntimeSessionLaunchSpec,
        generation: u64,
        size: TerminalSize,
    ) -> Option<SessionRuntimeState> {
        let mut command = CommandBuilder::new(&launch.shell);
        command.cwd(&launch.cwd);
        self.ensure_runtime(&launch.session_id, generation, command, size)
    }

    pub(crate) fn terminal_binding_for_handle(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<SessionHostTerminalBinding> {
        self.terminal_binding_for_handle_inner(terminal_handle)
    }

    pub(crate) fn focus_terminal_handle(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<SessionRuntimeState> {
        let binding = self.terminal_binding_for_handle_inner(terminal_handle)?;
        self.focus_terminal_instance(
            &binding.session_id,
            binding.runtime_id,
            binding.terminal_instance_id,
        )
    }
}

fn engine_terminal_size_default() -> TerminalSize {
    TerminalSize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
        dpi: 96,
    }
}

fn terminal_size_from_dims(dims: RenderableDimensions) -> TerminalSize {
    TerminalSize {
        rows: dims.viewport_rows.max(1),
        cols: dims.cols.max(1),
        pixel_width: dims.pixel_width,
        pixel_height: dims.pixel_height,
        dpi: dims.dpi.max(1),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    use super::super::acquire_host_runtime_test_lock;
    use super::{
        ChatminalSessionPane, DesktopSessionHost, RenderableDimensions,
        build_host_runtime_for_test, shutdown_host_runtime_for_test, test_active_frontend_client,
        test_active_workspace_for_client,
    };
    use crate::chatminal_render::ChatminalRenderState;
    use crate::chatminal_runtime::{
        SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle,
    };
    use crate::desktop_session_host::session_engine::{
        RuntimeId, SessionCoreState, SessionEngineShared, TerminalInstanceId,
    };
    use chatminal_runtime::RuntimeEntryTerminalInfo;
    use config::ConfigHandle;
    use config::keyassignment::SessionDirection;
    use engine_term::TerminalSize;

    fn host_with_registered_session_pane(
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> (DesktopSessionHost, Arc<ChatminalSessionPane>) {
        host_with_registered_session_pane_size(
            session_id,
            runtime_id,
            terminal_instance_id,
            TerminalSize::default(),
        )
    }

    fn host_with_registered_session_pane_size(
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        size: TerminalSize,
    ) -> (DesktopSessionHost, Arc<ChatminalSessionPane>) {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let host = DesktopSessionHost::new(shared, config::current_config_handle());
        let pane = ChatminalSessionPane::new(
            Arc::clone(&host.shared),
            session_id.to_string(),
            runtime_id,
            terminal_instance_id,
            size,
            config::current_config_handle(),
        )
        .expect("create session pane");
        host.panes
            .lock()
            .unwrap()
            .insert(terminal_instance_id, Arc::clone(&pane));
        host.session_pane
            .lock()
            .unwrap()
            .insert(session_id.to_string(), Arc::clone(&pane));
        (host, pane)
    }

    #[test]
    fn terminal_binding_for_public_id_resolves_handle_and_terminal_instance() {
        let runtime_id = RuntimeId::new(41);
        let terminal_instance_id = TerminalInstanceId::new(42);
        let (host, pane) =
            host_with_registered_session_pane("session-a", runtime_id, terminal_instance_id);
        assert_eq!(pane.pane_id_value().as_u64(), terminal_instance_id.as_u64());
        assert!(
            host.pane_for_terminal_handle(pane.pane_id_value())
                .is_some()
        );

        let from_handle = host
            .terminal_binding_for_public_id(pane.pane_id_value().as_u64())
            .expect("binding from pane handle");
        let from_terminal_instance = host
            .terminal_binding_for_public_id(terminal_instance_id.as_u64())
            .expect("binding from terminal instance");

        assert_eq!(from_handle.session_id, "session-a");
        assert_eq!(from_handle.runtime_id, runtime_id);
        assert_eq!(from_handle.terminal_instance_id, terminal_instance_id);
        assert_eq!(
            from_handle.terminal_handle,
            from_terminal_instance.terminal_handle
        );
        assert_eq!(from_handle.runtime_id, from_terminal_instance.runtime_id);
    }

    #[test]
    fn frontend_resolve_pane_fallback_uses_local_registry_before_mux() {
        let runtime_id = RuntimeId::new(51);
        let terminal_instance_id = TerminalInstanceId::new(52);
        let (host, pane) =
            host_with_registered_session_pane("session-b", runtime_id, terminal_instance_id);
        assert!(
            host.pane_for_public_id(pane.pane_id_value().as_u64())
                .is_some()
        );

        let resolved_from_handle = host
            .frontend_resolve_pane_fallback(pane.pane_id_value())
            .expect("resolved pane from handle");
        let resolved_from_terminal_instance = host
            .frontend_resolve_pane_fallback(SessionTerminalHandle::new(
                terminal_instance_id.as_u64(),
            ))
            .expect("resolved pane from terminal instance");

        assert_eq!(resolved_from_handle.runtime_id, runtime_id);
        assert_eq!(resolved_from_terminal_instance.runtime_id, runtime_id);
    }

    #[test]
    fn remove_terminal_handle_prunes_local_registry_before_host_fallback() {
        let runtime_id = RuntimeId::new(61);
        let terminal_instance_id = TerminalInstanceId::new(62);
        let (host, pane) =
            host_with_registered_session_pane("session-c", runtime_id, terminal_instance_id);
        host.runtime_terminal_instances
            .lock()
            .unwrap()
            .insert(runtime_id, HashSet::from([terminal_instance_id]));

        host.remove_terminal_handle(pane.pane_id_value());

        assert!(host.pane_for_session("session-c").is_none());
        assert!(
            host.pane_for_terminal_handle(pane.pane_id_value())
                .is_none()
        );
        assert!(
            host.pane_for_public_id(terminal_instance_id.as_u64())
                .is_none()
        );
        assert!(
            host.runtime_terminal_instances
                .lock()
                .unwrap()
                .get(&runtime_id)
                .is_none()
        );
    }

    #[test]
    fn runtime_entry_terminal_handle_in_direction_prefers_adjacent_local_layout() {
        let left = SessionTerminalHandle::new(71);
        let right = SessionTerminalHandle::new(72);
        let infos = vec![
            RuntimeEntryTerminalInfo {
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 0,
                top: 0,
                width: 40,
                pixel_width: 400,
                height: 24,
                pixel_height: 240,
                terminal_handle: left,
                session_id: Some("session-d".to_string()),
                terminal_instance_id: Some(71),
            },
            RuntimeEntryTerminalInfo {
                index: 1,
                is_active: false,
                is_zoomed: false,
                left: 41,
                top: 0,
                width: 40,
                pixel_width: 400,
                height: 24,
                pixel_height: 240,
                terminal_handle: right,
                session_id: Some("session-d".to_string()),
                terminal_instance_id: Some(72),
            },
        ];

        assert_eq!(
            super::runtime_entry_terminal_handle_in_direction(&infos, SessionDirection::Right),
            Some(right)
        );
        assert_eq!(
            super::runtime_entry_terminal_handle_in_direction(&infos, SessionDirection::Left),
            None
        );
    }

    #[test]
    fn runtime_entry_terminal_handle_in_direction_cycles_next_prev_locally() {
        let first = SessionTerminalHandle::new(81);
        let second = SessionTerminalHandle::new(82);
        let infos = vec![
            RuntimeEntryTerminalInfo {
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 0,
                top: 0,
                width: 40,
                pixel_width: 400,
                height: 24,
                pixel_height: 240,
                terminal_handle: first,
                session_id: Some("session-e".to_string()),
                terminal_instance_id: Some(81),
            },
            RuntimeEntryTerminalInfo {
                index: 1,
                is_active: false,
                is_zoomed: false,
                left: 41,
                top: 0,
                width: 40,
                pixel_width: 400,
                height: 24,
                pixel_height: 240,
                terminal_handle: second,
                session_id: Some("session-e".to_string()),
                terminal_instance_id: Some(82),
            },
        ];

        assert_eq!(
            super::runtime_entry_terminal_handle_in_direction(&infos, SessionDirection::Next),
            Some(second)
        );
        assert_eq!(
            super::runtime_entry_terminal_handle_in_direction(&infos, SessionDirection::Prev),
            Some(second)
        );
    }

    #[test]
    fn reconcile_visible_sessions_prunes_stale_session_indexes() {
        let runtime_id = RuntimeId::new(71);
        let terminal_instance_id = TerminalInstanceId::new(72);
        let (host, _pane) =
            host_with_registered_session_pane("session-d", runtime_id, terminal_instance_id);
        host.runtime_render_state.lock().unwrap().insert(
            runtime_id,
            ChatminalRenderState {
                render_target: SessionRenderTargetSnapshot {
                    render_target_id: SessionRenderTargetId::new(runtime_id.as_u64()),
                    runtime_id,
                    active_terminal_instance_id: Some(terminal_instance_id),
                },
                terminal_size: TerminalSize::default(),
                active_terminal_instance_id: Some(terminal_instance_id),
                panes: Vec::new(),
                splits: Vec::new(),
            },
        );

        host.reconcile_visible_sessions(&HashSet::new());

        assert!(host.panes.lock().unwrap().is_empty());
        assert!(host.session_pane.lock().unwrap().is_empty());
        assert!(host.runtime_render_state.lock().unwrap().is_empty());
    }

    #[test]
    fn pane_dims_need_resize_uses_live_dimensions_not_desired_size_cache() {
        let live_dims = RenderableDimensions {
            cols: 80,
            viewport_rows: 24,
            scrollback_rows: 24,
            physical_top: 0,
            scrollback_top: 0,
            pixel_width: 800,
            pixel_height: 600,
            dpi: 96,
            reverse_video: false,
        };
        let desired_size = TerminalSize {
            rows: 12,
            cols: 40,
            pixel_width: 400,
            pixel_height: 300,
            dpi: 96,
        };

        assert!(super::pane_dims_need_resize(live_dims, desired_size));
        assert!(!super::pane_dims_need_resize(
            RenderableDimensions {
                cols: desired_size.cols,
                viewport_rows: desired_size.rows,
                scrollback_rows: desired_size.rows,
                physical_top: 0,
                scrollback_top: 0,
                pixel_width: desired_size.pixel_width,
                pixel_height: desired_size.pixel_height,
                dpi: desired_size.dpi,
                reverse_video: false,
            },
            desired_size,
        ));
    }

    #[test]
    fn build_host_runtime_for_test_reuses_existing_client_identity() {
        let _guard = acquire_host_runtime_test_lock();
        shutdown_host_runtime_for_test();

        let config = ConfigHandle::default_config();
        build_host_runtime_for_test(&config, Some("workspace-a"))
            .expect("build initial host runtime first time");
        let first_client = test_active_frontend_client().expect("first client");
        assert_eq!(
            test_active_workspace_for_client(&first_client),
            "workspace-a".to_string()
        );

        build_host_runtime_for_test(&config, Some("workspace-b"))
            .expect("build initial host runtime second time");
        let second_client = test_active_frontend_client().expect("second client");
        assert_eq!(first_client, second_client);
        assert_eq!(
            test_active_workspace_for_client(&second_client),
            "workspace-b".to_string()
        );

        shutdown_host_runtime_for_test();
    }
}
