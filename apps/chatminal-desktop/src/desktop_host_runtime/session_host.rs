// Desktop session host: manages session runtime + host leaf lifecycle for one desktop window.
//
// This is the session-native render path (Phase 03+). The host creates `ChatminalSessionPane`
// objects directly from the session engine's core state and builds `ChatminalRenderState`
// from the session_pane map.

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use super::session_engine::{
    RuntimeId, SessionEngineShared, SessionRuntimeState, StatefulSessionEngine, TerminalInstanceId,
};
use super::spawn_target::DesktopSpawnTarget;
use config::ConfigHandle;
use engine_term::{Clipboard, ClipboardSelection, DownloadHandler, TerminalSize};
use host_runtime::client::ClientId;
use host_runtime::tab::Tab as HostRenderScope;
use portable_pty::CommandBuilder;

use super::session_pane::{pane_id_for_terminal_instance, ChatminalSessionPane};
use super::{
    configured_default_workspace_name, host_window_exists,
    publish_runtime_notification_from_any_thread, subscribe_desktop_runtime_notifications,
    FrontendClientHandle, FrontendFocusedPane, FrontendResolvedPane, HostFocusedPaneBinding,
    HostRenderableDimensions as RenderableDimensions, HostSpawnTargetHandle, HostTerminal,
    LauncherSessionEntry, RuntimeNotification, RuntimeWindow, PRIMARY_HOST_WINDOW_ID,
    ROOT_HOST_WINDOW_ID,
};
use crate::chatminal_render::{ChatminalRenderPane, ChatminalRenderState};
use crate::chatminal_runtime::{
    SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle,
};
use chatminal_runtime::{
    RuntimeHost, RuntimeHostSessionState, RuntimeHostTerminalBinding, RuntimeSessionLaunchSpec,
    RuntimeTerminalSize,
};

// ---------------------------------------------------------------------------
// Singleton host registry
// ---------------------------------------------------------------------------

static HOST_REGISTRY: OnceLock<Arc<DesktopSessionHost>> = OnceLock::new();
static LEGACY_HOST_NOTIFICATIONS_BRIDGED: AtomicBool = AtomicBool::new(false);

pub(crate) fn get_or_init_session_host(
    shared: Arc<SessionEngineShared>,
    config: ConfigHandle,
) -> Arc<DesktopSessionHost> {
    HOST_REGISTRY
        .get_or_init(|| Arc::new(DesktopSessionHost::new(shared, config)))
        .clone()
}

fn host_tab_id(runtime_id: RuntimeId) -> Option<usize> {
    usize::try_from(runtime_id.as_u64()).ok()
}

fn host_root_active_tab_id() -> Option<RuntimeId> {
    host_runtime::root_active_runtime_id()
}

fn host_terminal_handle(pane: &dyn HostTerminal) -> SessionTerminalHandle {
    host_runtime::terminal_handle_for_pane(pane)
}

fn host_remove_pane(terminal_handle: SessionTerminalHandle) {
    let _ = host_runtime::remove_terminal_handle(terminal_handle);
}

/// Register a session pane with the host runtime, using a callback that
/// publishes PTY output events through the desktop notification hub
/// instead of the default host-runtime output notifier.
fn host_add_session_pane_with_output_callback(pane: &Arc<dyn HostTerminal>) -> anyhow::Result<()> {
    let on_pane_output: Arc<dyn Fn(SessionTerminalHandle) + Send + Sync> =
        Arc::new(|terminal_handle| {
            publish_runtime_notification_from_any_thread(RuntimeNotification::PaneOutput(
                terminal_handle,
            ));
        });
    host_runtime::register_pane_with_output_callback(pane, Some(on_pane_output))
        .map_err(anyhow::Error::from)
}

fn bridge_runtime_notifications(runtime: &host_runtime::HostRuntimeHandle) {
    runtime.subscribe(|notification| {
        publish_runtime_notification_from_any_thread(notification.into());
        true
    });
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
        publish_runtime_notification_from_any_thread(RuntimeNotification::AssignClipboard {
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
        publish_runtime_notification_from_any_thread(RuntimeNotification::SaveToDownloads {
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

fn host_get_pane(terminal_handle: SessionTerminalHandle) -> Option<Arc<dyn HostTerminal>> {
    host_runtime::terminal_by_handle(terminal_handle)
}

fn host_remove_tab(runtime_id: RuntimeId) {
    let _ = host_runtime::remove_runtime_entry_by_runtime_id(runtime_id);
}

fn host_add_tab_and_active_pane(tab: &Arc<HostRenderScope>) -> anyhow::Result<()> {
    host_runtime::register_tab(tab).map_err(anyhow::Error::from)
}

fn host_attach_tab(tab: &Arc<HostRenderScope>) -> anyhow::Result<()> {
    host_runtime::attach_tab_to_window(tab)
}

fn with_host_window_ref<R, F>(func: F) -> R
where
    F: FnOnce(&RuntimeWindow) -> R,
{
    host_runtime::with_root_window(func).expect("host runtime root window to exist")
}

fn with_host_window_mut_ref<R, F>(func: F) -> R
where
    F: FnOnce(&mut RuntimeWindow) -> R,
{
    host_runtime::with_root_window_mut(func).expect("host runtime root window to exist")
}

fn host_has_tab(runtime_id: RuntimeId) -> bool {
    host_runtime::runtime_entry_exists(runtime_id)
}

fn host_workspace_name_value() -> String {
    host_runtime::root_window_workspace_name()
        .or_else(|| host_runtime::active_workspace_name())
        .unwrap_or_default()
}

fn host_set_workspace_name(name: &str) {
    let _ = host_runtime::set_active_workspace_name(name);
    let _ = host_runtime::set_root_window_workspace_name(name);
}

fn host_record_focus(terminal_handle: SessionTerminalHandle) {
    let _ = host_runtime::record_focus_for_terminal_handle(terminal_handle);
}

fn host_subscribe<F>(subscriber: F)
where
    F: Fn(RuntimeNotification) -> bool + 'static + Send + Sync,
{
    subscribe_desktop_runtime_notifications(subscriber);
}

fn host_active_identity() -> Option<FrontendClientHandle> {
    host_runtime::active_identity()
}

fn host_workspace_for_client(client_id: &FrontendClientHandle) -> String {
    host_runtime::active_workspace_for_client(client_id).unwrap_or_default()
}

fn host_set_workspace_for_client(client_id: &FrontendClientHandle, workspace: &str) {
    let _ = host_runtime::set_active_workspace_for_client(client_id, workspace);
}

fn host_workspace_is_empty_value(workspace: &str) -> bool {
    host_runtime::is_workspace_empty(workspace).unwrap_or(true)
}

fn host_workspace_names_value() -> Vec<String> {
    host_runtime::iter_workspaces()
}

fn host_root_has_runtime_entries_with_panes() -> bool {
    host_runtime::root_has_runtime_entries_with_panes()
}

fn host_root_window_workspace_name_value() -> Option<String> {
    host_runtime::root_window_workspace_name()
}

fn host_resolve_pane_id_value(terminal_handle: SessionTerminalHandle) -> Option<RuntimeId> {
    host_runtime::resolve_runtime_id_for_terminal_handle(terminal_handle)
}

fn host_resolve_focused_pane_value(
    client_id: &FrontendClientHandle,
) -> Option<HostFocusedPaneBinding> {
    host_runtime::resolve_focused_pane(client_id)
}

fn host_iter_panes() -> Vec<Arc<dyn HostTerminal>> {
    host_runtime::iter_panes()
}

async fn host_spawn_tab(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    current_terminal_handle: Option<SessionTerminalHandle>,
) -> anyhow::Result<(Arc<HostRenderScope>, Arc<dyn HostTerminal>)> {
    host_runtime::spawn_tab(command, command_dir, size, current_terminal_handle).await
}

fn host_initialize_runtime(
    spawn_target: Option<HostSpawnTargetHandle>,
    config: ConfigHandle,
) -> anyhow::Result<Arc<host_runtime::HostRuntimeHandle>> {
    host_runtime::initialize_host_runtime_with_config(spawn_target, config)
}

fn host_set_primary_spawn_target_value(spawn_target: &HostSpawnTargetHandle) {
    let _ = host_runtime::set_primary_spawn_target(spawn_target);
}

fn host_primary_spawn_target_value() -> HostSpawnTargetHandle {
    host_runtime::primary_spawn_target().expect("host runtime primary spawn target")
}

fn host_focus_root_window_tab(runtime_id: RuntimeId) -> bool {
    host_runtime::focus_root_runtime_entry(runtime_id)
}

fn host_active_terminal_handle(runtime_id: RuntimeId) -> Option<SessionTerminalHandle> {
    host_runtime::runtime_entry_active_terminal_handle(runtime_id)
}

fn host_set_tab_title(runtime_id: RuntimeId, title: &str) {
    let _ = host_runtime::set_runtime_entry_title(runtime_id, title);
}

fn host_is_runtime_available() -> bool {
    host_runtime::is_host_runtime_available()
}

fn host_resize_all_tabs(size: TerminalSize) {
    host_runtime::resize_all_root_runtime_entries(size);
}

fn host_shutdown_runtime() {
    host_runtime::shutdown_host_runtime();
}

fn ensure_runtime_notification_bridge(runtime: &host_runtime::HostRuntimeHandle) {
    if !LEGACY_HOST_NOTIFICATIONS_BRIDGED.swap(true, Ordering::AcqRel) {
        bridge_runtime_notifications(runtime);
    }
}

fn initialize_desktop_host_runtime(
    config: &ConfigHandle,
    default_workspace_name: Option<&str>,
) -> anyhow::Result<FrontendClientHandle> {
    let desktop_spawn_target: HostSpawnTargetHandle = Arc::new(DesktopSpawnTarget::new_local()?);
    let runtime = host_initialize_runtime(Some(Arc::clone(&desktop_spawn_target)), config.clone())?;
    host_set_primary_spawn_target_value(&desktop_spawn_target);
    ensure_runtime_notification_bridge(runtime.as_ref());

    let client_id = host_active_identity().unwrap_or_else(|| {
        let client_id = Arc::new(ClientId::new());
        runtime.register_client(client_id.clone());
        client_id
    });
    runtime.replace_identity(Some(client_id.clone()));

    let workspace = default_workspace_name
        .map(str::to_string)
        .unwrap_or_else(|| configured_default_workspace_name(config));
    runtime.set_active_workspace(&workspace);
    let _ = host_runtime::set_root_window_workspace_name(&workspace);
    host_set_workspace_for_client(&client_id, &workspace);
    let _ = PRIMARY_HOST_WINDOW_ID.set(ROOT_HOST_WINDOW_ID);
    Ok(client_id)
}

fn shutdown_desktop_host_runtime() {
    LEGACY_HOST_NOTIFICATIONS_BRIDGED.store(false, Ordering::Release);
    host_shutdown_runtime();
}

fn pane_dims_need_resize(dims: RenderableDimensions, size: TerminalSize) -> bool {
    dims.cols != size.cols
        || dims.viewport_rows != size.rows
        || dims.pixel_width != size.pixel_width
        || dims.pixel_height != size.pixel_height
        || dims.dpi != size.dpi
}

#[cfg(test)]
pub(crate) fn test_active_frontend_client() -> Option<FrontendClientHandle> {
    host_active_identity()
}

#[cfg(test)]
pub(crate) fn test_active_workspace_for_client(client_id: &FrontendClientHandle) -> String {
    let workspace = host_workspace_for_client(client_id);
    if !workspace.is_empty() {
        return workspace;
    }
    host_runtime::root_window_workspace_name()
        .or_else(host_runtime::active_workspace_name)
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) fn build_host_runtime_for_test(
    config: &ConfigHandle,
    default_workspace_name: Option<&str>,
) -> anyhow::Result<()> {
    let _ = initialize_desktop_host_runtime(config, default_workspace_name)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn shutdown_host_runtime_for_test() {
    shutdown_desktop_host_runtime();
}

// ---------------------------------------------------------------------------
// DesktopSessionHost
// ---------------------------------------------------------------------------

pub(crate) struct DesktopSessionHost {
    shared: Arc<SessionEngineShared>,
    config: Mutex<ConfigHandle>,
    // terminal_instance_id → pane (for output/input routing)
    panes: Mutex<HashMap<TerminalInstanceId, Arc<ChatminalSessionPane>>>,
    // session_id → pane (1 session = 1 pane invariant)
    session_pane: Mutex<HashMap<String, Arc<ChatminalSessionPane>>>,
    // session_id → shim runtime id for the host render-scope wrapper (desktop-local)
    session_tab_shim: Mutex<HashMap<String, RuntimeId>>,
    // runtime_id → first-party render snapshot for termwindow compatibility
    runtime_render_state: Mutex<HashMap<RuntimeId, ChatminalRenderState>>,
    // runtime_id → terminal instances owned by that runtime
    runtime_terminal_instances: Mutex<HashMap<RuntimeId, HashSet<TerminalInstanceId>>>,
    // runtime_id → last terminal size confirmed for the live PTY
    runtime_terminal_size: Mutex<HashMap<RuntimeId, TerminalSize>>,
}

fn host_session_state(state: SessionRuntimeState) -> RuntimeHostSessionState {
    RuntimeHostSessionState {
        session_id: state.snapshot.session_id.clone(),
        runtime_id: state.snapshot.runtime_id,
        active_terminal_instance_id: state.snapshot.active_terminal_instance_id,
    }
}

fn engine_terminal_size(size: RuntimeTerminalSize) -> TerminalSize {
    TerminalSize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: size.pixel_width,
        pixel_height: size.pixel_height,
        dpi: size.dpi,
    }
}

fn host_terminal_binding(pane: &ChatminalSessionPane) -> RuntimeHostTerminalBinding {
    RuntimeHostTerminalBinding {
        session_id: pane.session_id_value().to_string(),
        runtime_id: pane.runtime_id_value(),
        terminal_instance_id: pane.terminal_instance_id_value(),
        terminal_handle: pane.pane_id_value(),
    }
}

impl DesktopSessionHost {
    fn new(shared: Arc<SessionEngineShared>, config: ConfigHandle) -> Self {
        Self {
            shared,
            config: Mutex::new(config),
            panes: Mutex::new(HashMap::new()),
            session_pane: Mutex::new(HashMap::new()),
            session_tab_shim: Mutex::new(HashMap::new()),
            runtime_render_state: Mutex::new(HashMap::new()),
            runtime_terminal_instances: Mutex::new(HashMap::new()),
            runtime_terminal_size: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn set_config(&self, config: &ConfigHandle) {
        self.config.lock().unwrap().clone_from(config);
    }

    fn engine(&self) -> StatefulSessionEngine {
        StatefulSessionEngine::with_shared(Arc::clone(&self.shared))
    }

    // -----------------------------------------------------------------
    // Runtime lifecycle
    // -----------------------------------------------------------------

    /// Ensure a session runtime attachment exists: focus it if it already exists, or
    /// spawn a new one. Creates/updates the host render-scope wrapper for termwindow.
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
        let mut session_tab_shim = self.session_tab_shim.lock().unwrap();
        let mut runtime_render_state = self.runtime_render_state.lock().unwrap();

        for session_id in stale_session_ids {
            let stale_terminal_instance_id = session_pane
                .get(&session_id)
                .map(|pane| pane.terminal_instance_id_value());
            if let Some(tab_id) = session_tab_shim.remove(&session_id) {
                host_remove_tab(tab_id);
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

    /// Create or update panes for each leaf in the state, then ensure a render tab
    /// shim exists with the current active pane.
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

        // Create panes for any leaf that doesn't have one yet
        for leaf_snapshot in &layout.leaves {
            let terminal_instance_id = leaf_snapshot.terminal_instance_id;
            if !panes_guard.contains_key(&terminal_instance_id) {
                if let Some(pane) =
                    self.adopt_existing_pane(session_id, runtime_id, terminal_instance_id)
                {
                    panes_guard.insert(terminal_instance_id, pane);
                    continue;
                }

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
                        // Register with the host runtime for render compat,
                        // routing PTY output through the desktop notification
                        // hub instead of the default host-runtime notifier.
                        if let Err(err) = host_add_session_pane_with_output_callback(
                            &(pane.clone() as Arc<dyn HostTerminal>),
                        ) {
                            log::warn!(
                                "session host: could not register pane {terminal_instance_id}: {err}"
                            );
                        }
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

        // Defer stale-pane cleanup until after we have attached a live tab shim
        // to the host window. Otherwise the bootstrap pane can be removed first,
        // making the mux window empty and causing it to be pruned before the
        // session-native tab is attached.
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
            self.ensure_mux_tab_shim(session_id, &active_pane);

            // Build ChatminalRenderState directly from session_pane — no HostRenderScope needed.
            // 1 session = 1 pane invariant: panes has exactly one element; splits = [] (splits are
            // at workspace layout level, not session level).
            let terminal_handle = host_terminal_handle(active_pane.as_ref());
            let terminal_size = self
                .runtime_terminal_size
                .lock()
                .unwrap()
                .get(&runtime_id)
                .copied()
                .unwrap_or_else(|| terminal_size_from_dims(active_pane.get_dimensions()));
            let render_state = ChatminalRenderState {
                render_target: SessionRenderTargetSnapshot {
                    render_target_id: SessionRenderTargetId::new(runtime_id.as_u64()),
                    runtime_id,
                    active_terminal_instance_id: Some(layout.active_terminal_instance_id),
                },
                terminal_size,
                active_terminal_instance_id: Some(layout.active_terminal_instance_id),
                panes: vec![ChatminalRenderPane {
                    terminal_handle,
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
            };
            self.runtime_render_state
                .lock()
                .unwrap()
                .insert(runtime_id, render_state);
        }

        if !stale.is_empty() {
            let mut panes_guard = self.panes.lock().unwrap();
            for stale_terminal_instance_id in stale {
                if let Some(stale_pane) = panes_guard.remove(&stale_terminal_instance_id) {
                    host_remove_pane(host_terminal_handle(stale_pane.as_ref()));
                }
            }
        }
    }

    fn adopt_existing_pane(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<Arc<ChatminalSessionPane>> {
        let pane_id = pane_id_for_terminal_instance(terminal_instance_id);
        let pane = host_get_pane(pane_id)?;
        let pane = pane.downcast_arc::<ChatminalSessionPane>().ok()?;
        if pane.runtime_id_value() != runtime_id
            || pane.terminal_instance_id_value() != terminal_instance_id
            || pane.session_id_value() != session_id
        {
            return None;
        }
        install_desktop_pane_side_effects(&(pane.clone() as Arc<dyn HostTerminal>));
        log::debug!(
            "session host: adopted existing pane {} for session {} terminal_instance {}",
            host_terminal_handle(pane.as_ref()).as_u64(),
            session_id,
            terminal_instance_id.as_u64()
        );
        Some(pane)
    }

    fn ensure_mux_tab_shim(&self, session_id: &str, active_pane: &Arc<ChatminalSessionPane>) {
        if !host_window_exists() {
            log::debug!(
                "session host: deferring tab shim attach for session {session_id}; root window not ready yet",
            );
            return;
        }

        let active_terminal_handle = host_terminal_handle(active_pane.as_ref());
        let title = active_pane.get_title();

        // Desktop-local lookup: use session_tab_shim instead of Mux global index.
        let existing_tab_id = self
            .session_tab_shim
            .lock()
            .unwrap()
            .get(session_id)
            .copied();
        let runtime_id = match existing_tab_id {
            Some(runtime_id)
                if host_active_terminal_handle(runtime_id) == Some(active_terminal_handle) =>
            {
                runtime_id
            }
            Some(runtime_id) => {
                let replacement = Arc::new(HostRenderScope::new(&engine_terminal_size_default()));
                replacement.assign_pane(&(active_pane.clone() as Arc<dyn HostTerminal>));
                replacement.set_title(&title);
                if let Err(err) = host_add_tab_and_active_pane(&replacement) {
                    log::warn!(
                        "session host: failed to register replacement tab shim for session {session_id}: {err}"
                    );
                    return;
                }
                if let Err(err) = host_attach_tab(&replacement) {
                    log::warn!(
                        "session host: failed to attach replacement tab shim for session {session_id} to root window: {err}"
                    );
                    host_remove_tab(replacement.runtime_id());
                    return;
                }
                let replacement_tab_id = replacement.runtime_id();
                host_remove_tab(runtime_id);
                self.session_tab_shim
                    .lock()
                    .unwrap()
                    .insert(session_id.to_string(), replacement_tab_id);
                replacement_tab_id
            }
            None => {
                let tab = Arc::new(HostRenderScope::new(&engine_terminal_size_default()));
                tab.assign_pane(&(active_pane.clone() as Arc<dyn HostTerminal>));
                tab.set_title(&title);
                if let Err(err) = host_add_tab_and_active_pane(&tab) {
                    log::warn!(
                        "session host: failed to register tab shim for session {session_id}: {err}"
                    );
                    return;
                }
                if let Err(err) = host_attach_tab(&tab) {
                    log::warn!(
                        "session host: failed to attach tab shim for session {session_id} to root window: {err}"
                    );
                    host_remove_tab(tab.runtime_id());
                    return;
                }
                let new_tab_id = tab.runtime_id();
                self.session_tab_shim
                    .lock()
                    .unwrap()
                    .insert(session_id.to_string(), new_tab_id);
                new_tab_id
            }
        };

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
                // Also remove from session_pane and session_tab_shim indexes
                let session_id = pane.session_id_value().to_string();
                self.session_pane.lock().unwrap().remove(&session_id);
                self.session_tab_shim.lock().unwrap().remove(&session_id);
                host_remove_pane(host_terminal_handle(pane.as_ref()));
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
    ) -> Option<RuntimeHostTerminalBinding> {
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
    ) -> Option<RuntimeHostTerminalBinding> {
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
            if let Some(tab_id) = self.session_tab_shim.lock().unwrap().remove(&session_id) {
                host_remove_tab(tab_id);
                return true;
            }
        }

        host_remove_pane(terminal_handle);
        true
    }

    pub(crate) fn host_workspace_name(&self) -> String {
        host_workspace_name_value()
    }

    pub(crate) fn active_frontend_client(&self) -> Option<FrontendClientHandle> {
        host_active_identity()
    }

    pub(crate) fn subscribe_notifications<F>(&self, subscriber: F)
    where
        F: Fn(RuntimeNotification) -> bool + 'static + Send + Sync,
    {
        host_subscribe(subscriber);
    }

    pub(crate) fn active_workspace_for_client(&self, client_id: &FrontendClientHandle) -> String {
        let workspace = host_workspace_for_client(client_id);
        if !workspace.is_empty() {
            return workspace;
        }
        host_runtime::root_window_workspace_name()
            .or_else(host_runtime::active_workspace_name)
            .unwrap_or_default()
    }

    pub(crate) fn set_active_workspace_for_client(
        &self,
        client_id: &FrontendClientHandle,
        workspace: &str,
    ) {
        host_set_workspace_for_client(client_id, workspace);
    }

    pub(crate) fn workspace_is_empty(&self, workspace: &str) -> bool {
        host_workspace_is_empty_value(workspace)
    }

    pub(crate) fn workspace_names(&self) -> Vec<String> {
        host_workspace_names_value()
    }

    pub(crate) fn with_window<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&RuntimeWindow) -> R,
    {
        Some(with_host_window_ref(func))
    }

    pub(crate) fn with_window_mut<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&mut RuntimeWindow) -> R,
    {
        Some(with_host_window_mut_ref(func))
    }

    pub(crate) fn window_exists(&self) -> bool {
        host_is_runtime_available()
    }

    pub(crate) fn workspace_has_windows(&self, name: &str) -> bool {
        host_root_window_workspace_name_value()
            .as_deref()
            .map(|workspace| workspace == name)
            .unwrap_or(false)
    }

    pub(crate) fn resize_window_tabs(&self, size: TerminalSize) {
        host_resize_all_tabs(size);
    }

    pub(crate) fn has_panes_in_workspace(&self, workspace: Option<&str>) -> bool {
        if host_iter_panes().is_empty() {
            return false;
        }
        let Some(workspace) = workspace else {
            return true;
        };
        self.workspace_has_windows(workspace) && host_root_has_runtime_entries_with_panes()
    }

    pub(crate) fn active_render_scope_id(&self) -> Option<u64> {
        host_root_active_tab_id().map(|runtime_id| runtime_id.as_u64())
    }

    pub(crate) fn host_window_contains_render_scope(&self, render_scope_id: u64) -> bool {
        host_has_tab(RuntimeId::new(render_scope_id))
    }

    pub(crate) fn remove_terminal_handle(&self, terminal_handle: SessionTerminalHandle) {
        if self.remove_registered_pane(terminal_handle) {
            return;
        }
        host_remove_pane(terminal_handle);
    }

    pub(crate) fn remove_runtime_entry_scope(&self, render_scope_id: u64) {
        host_remove_tab(RuntimeId::new(render_scope_id));
    }

    pub(crate) fn record_focus_for_current_identity(&self, terminal_handle: SessionTerminalHandle) {
        host_record_focus(terminal_handle);
    }

    pub(crate) fn record_input_for_current_identity(&self) {
        let _ = host_runtime::record_input_for_current_identity();
    }

    pub(crate) fn host_window_initial_position(&self) -> Option<config::GuiPosition> {
        if !self.window_exists() {
            return None;
        }
        self.with_window(|window| window.get_initial_position().clone())
            .flatten()
    }

    pub(crate) fn active_runtime_entry_size(&self) -> Option<TerminalSize> {
        if !self.window_exists() {
            return None;
        }
        self.with_window(|window| {
            window
                .get_by_idx(window.get_active_idx())
                .cloned()
                .map(|tab| tab.get_size())
        })
        .flatten()
    }

    pub(crate) fn resolved_window_title(&self) -> Option<String> {
        if !self.window_exists() {
            return None;
        }
        self.with_window(|window| window.get_title().to_string())
    }

    pub(crate) fn launcher_sessions(&self) -> Vec<LauncherSessionEntry> {
        if !self.window_exists() {
            return Vec::new();
        }
        self.with_window(|window| {
            window
                .iter()
                .enumerate()
                .map(|(tab_idx, tab)| {
                    let tab_title = tab.get_title();
                    let title = if tab_title.is_empty() {
                        tab.get_active_pane()
                            .expect("tab to have a pane")
                            .get_title()
                    } else {
                        tab_title
                    };
                    LauncherSessionEntry {
                        title,
                        tab_idx,
                        pane_count: tab.count_panes(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
    }

    pub(crate) fn activate_runtime_entry(&self, render_scope_id: u64) -> anyhow::Result<()> {
        let runtime_id = RuntimeId::new(render_scope_id);
        host_tab_id(runtime_id)
            .ok_or_else(|| anyhow::anyhow!("invalid runtime entry id {render_scope_id}"))?;
        host_focus_root_window_tab(runtime_id)
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
            .or_else(|| host_runtime::terminal_by_public_id(host_terminal_handle))
            .or_else(|| host_runtime::terminal_by_public_id(terminal_instance_id))
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
        let resolved =
            host_resolve_focused_pane_value(client_id).map(|binding| FrontendFocusedPane {
                runtime_id: binding.runtime_id(),
                terminal_handle: binding.terminal_handle(),
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
        let (_runtime_entry, pane) =
            host_spawn_tab(None, None, TerminalSize::default(), None).await?;
        Ok(pane)
    }

    pub(crate) async fn spawn_host_runtime_entry(
        &self,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        size: TerminalSize,
        current_pane_id: Option<u64>,
        workspace: String,
    ) -> anyhow::Result<Arc<dyn HostTerminal>> {
        host_set_workspace_name(&workspace);
        let (_runtime_entry, pane) = host_spawn_tab(
            command,
            command_dir,
            size,
            current_pane_id.map(SessionTerminalHandle::new),
        )
        .await?;
        Ok(pane)
    }

    pub(crate) fn set_primary_spawn_target(&self, spawn_target: &HostSpawnTargetHandle) {
        host_set_primary_spawn_target_value(spawn_target);
    }

    pub(crate) fn primary_spawn_target(&self) -> HostSpawnTargetHandle {
        host_primary_spawn_target_value()
    }

    pub(crate) fn build_initial_host_runtime(
        &self,
        config: &ConfigHandle,
        default_workspace_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let _ = initialize_desktop_host_runtime(config, default_workspace_name)?;
        Ok(())
    }

    pub(crate) fn shutdown_host_runtime(&self) {
        shutdown_desktop_host_runtime();
    }

    pub(crate) fn create_serial_spawn_target(
        &self,
        serial_target: config::SerialTarget,
    ) -> anyhow::Result<HostSpawnTargetHandle> {
        Ok(Arc::new(DesktopSpawnTarget::new_serial(serial_target)?))
    }
}

impl std::fmt::Debug for DesktopSessionHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopSessionHost").finish_non_exhaustive()
    }
}

impl RuntimeHost for DesktopSessionHost {
    fn runtime_id_for_session(&self, session_id: &str) -> Option<RuntimeId> {
        self.shared
            .core_state()
            .lock()
            .ok()?
            .runtime_id_for_session(session_id)
    }

    fn ensure_session_runtime(
        &self,
        launch: &RuntimeSessionLaunchSpec,
        generation: u64,
        size: RuntimeTerminalSize,
    ) -> Option<RuntimeHostSessionState> {
        let mut command = CommandBuilder::new(&launch.shell);
        command.cwd(&launch.cwd);
        DesktopSessionHost::ensure_runtime(
            self,
            &launch.session_id,
            generation,
            command,
            engine_terminal_size(size),
        )
        .map(host_session_state)
    }

    fn focus_session_runtime(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Option<RuntimeHostSessionState> {
        DesktopSessionHost::focus_runtime(self, session_id, runtime_id).map(host_session_state)
    }

    fn hydrate_session_runtime(&self, runtime_id: RuntimeId) -> Option<RuntimeHostSessionState> {
        DesktopSessionHost::hydrate_runtime(self, runtime_id).map(host_session_state)
    }

    fn remember_runtime_terminal_size(&self, runtime_id: RuntimeId, size: RuntimeTerminalSize) {
        DesktopSessionHost::remember_runtime_terminal_size(
            self,
            runtime_id,
            engine_terminal_size(size),
        );
    }

    fn terminal_binding_for_handle(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<RuntimeHostTerminalBinding> {
        self.terminal_binding_for_handle_inner(terminal_handle)
    }

    fn focus_terminal_instance(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<RuntimeHostSessionState> {
        DesktopSessionHost::focus_terminal_instance(
            self,
            session_id,
            runtime_id,
            terminal_instance_id,
        )
        .map(host_session_state)
    }

    fn resize_session_runtime(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        size: RuntimeTerminalSize,
    ) -> Option<RuntimeHostSessionState> {
        DesktopSessionHost::resize_runtime(self, session_id, runtime_id, engine_terminal_size(size))
            .map(host_session_state)
    }

    fn close_session_runtime(&self, session_id: &str, runtime_id: RuntimeId) {
        DesktopSessionHost::close_runtime(self, session_id, runtime_id);
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
        build_host_runtime_for_test, test_active_frontend_client,
        test_active_workspace_for_client, shutdown_host_runtime_for_test,
        ChatminalSessionPane, DesktopSessionHost,
    };
    use crate::chatminal_render::ChatminalRenderState;
    use crate::chatminal_runtime::{
        SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle,
    };
    use crate::desktop_host_runtime::session_engine::{
        RuntimeId, SessionCoreState, SessionEngineShared, TerminalInstanceId,
    };
    use config::ConfigHandle;
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
        assert!(host
            .pane_for_terminal_handle(pane.pane_id_value())
            .is_some());

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
        assert!(host
            .pane_for_public_id(pane.pane_id_value().as_u64())
            .is_some());

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
        assert!(host
            .pane_for_terminal_handle(pane.pane_id_value())
            .is_none());
        assert!(host
            .pane_for_public_id(terminal_instance_id.as_u64())
            .is_none());
        assert!(host
            .runtime_terminal_instances
            .lock()
            .unwrap()
            .get(&runtime_id)
            .is_none());
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
        let live_dims = host_runtime::renderable::RenderableDimensions {
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
            host_runtime::renderable::RenderableDimensions {
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
