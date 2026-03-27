mod spawn_target;
pub(crate) mod execution_bridge;
pub(crate) mod session_engine;
mod session_host;
mod session_pane;

use std::convert::TryFrom;
use std::ffi::OsStr;
use std::sync::Arc;
use std::sync::OnceLock;

use crate::chatminal_runtime::ChatminalRuntimeClient;
use anyhow::anyhow;
use chatminal_runtime::RuntimeState;
use config::keyassignment::{RotationDirection, SessionDirection};
use config::ConfigHandle;
use engine_dynamic::Value;
use engine_term::TerminalSize;
use host_runtime::activity::Activity;
use host_runtime::client::ClientId;
use host_runtime::spawn_target::SpawnTarget;
use host_runtime::pane::Pane;
use host_runtime::window::{Window as MuxWindow, WindowId as EngineWindowId, ROOT_WINDOW_ID};
use host_runtime::{Mux, MuxNotification};
use portable_pty::CommandBuilder;
use session_engine::TerminalInstanceId;

pub(crate) use execution_bridge::DesktopRuntimeExecutionBridge;
pub(crate) use session_host::{get_or_init_session_host, DesktopSessionHost};
pub(crate) use session_pane::ChatminalSessionPane;

pub(crate) const CHATMINAL_RUNTIME_SPAWN_TARGET_NAME: &str = "chatminal-runtime";
const DESKTOP_PROXY_COMMAND: &str = "proxy-desktop-session";
pub(crate) type HostSpawnTargetHandle = Arc<dyn SpawnTarget>;

pub(crate) struct HostActivityGuard(Activity);

pub(crate) mod overlay_compat {
    pub use host_runtime::pane::{
        CachePolicy as OverlayCachePolicy, CloseReason as OverlayCloseReason,
        ForEachPaneLogicalLine as OverlayForEachLogicalLine, LogicalLine as OverlayLogicalLine,
        Pane as OverlayPane, Pattern as OverlayPattern, PatternType as OverlayPatternType,
        PerformAssignmentResult as OverlayAssignmentResult, SearchResult as OverlaySearchResult,
        WithPaneLines as OverlayWithPaneLines,
    };
    pub use host_runtime::renderable::*;
    pub use host_runtime::termwiztermtab::{
        allocate as allocate_overlay_terminal, TermWizTerminal as OverlayTerminal,
    };
    pub type OverlaySplitDirection = host_runtime::tab::SplitDirection;
    pub type OverlayPaneHandle = usize;
    pub type OverlayRuntimeEntryHandle = usize;

    #[derive(Clone)]
    pub struct OverlayPaneLayout {
        pub index: usize,
        pub is_active: bool,
        pub is_zoomed: bool,
        pub left: usize,
        pub top: usize,
        pub width: usize,
        pub pixel_width: usize,
        pub height: usize,
        pub pixel_height: usize,
        pub pane: std::sync::Arc<dyn OverlayPane>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct OverlaySplitLayout {
        pub index: usize,
        pub direction: OverlaySplitDirection,
        pub left: usize,
        pub top: usize,
        pub size: usize,
    }
}

pub(crate) struct EmbeddedRuntime {
    pub(crate) state: RuntimeState,
    bridge: Arc<DesktopRuntimeExecutionBridge>,
}

pub(crate) type FrontendClientHandle = Arc<ClientId>;
pub(crate) type RuntimeNotification = MuxNotification;
pub(crate) type RuntimeWindow = MuxWindow;
pub(crate) type PrimaryHostWindowId = EngineWindowId;
pub(crate) type HostMux = Mux;
pub(crate) use host_runtime::spawn_target::SpawnTarget as HostSpawnTarget;
pub(crate) use host_runtime::pane::Pane as HostTerminal;
pub(crate) type HostTerminalHandle = usize;
pub(crate) type HostCachePolicy = host_runtime::pane::CachePolicy;
pub(crate) type HostCloseReason = host_runtime::pane::CloseReason;
pub(crate) type HostLogicalLine = host_runtime::pane::LogicalLine;
pub(crate) type HostSearchResult = host_runtime::pane::SearchResult;
pub(crate) type HostPattern = host_runtime::pane::Pattern;
pub(crate) type HostRenderableDimensions = host_runtime::renderable::RenderableDimensions;
pub(crate) type HostStableCursorPosition = host_runtime::renderable::StableCursorPosition;
pub(crate) type HostRenderScope = host_runtime::tab::Tab;

pub(crate) use host_runtime::pane::alloc_pane_id as alloc_host_terminal_handle;
pub(crate) use host_runtime::pane::impl_get_logical_lines_via_get_lines as host_impl_get_logical_lines_via_get_lines;
pub(crate) use host_runtime::renderable::{
    terminal_for_each_logical_line_in_stable_range_mut as host_terminal_for_each_logical_line_in_stable_range_mut,
    terminal_get_cursor_position as host_terminal_get_cursor_position,
    terminal_get_dimensions as host_terminal_get_dimensions,
    terminal_get_dirty_lines as host_terminal_get_dirty_lines,
    terminal_get_lines as host_terminal_get_lines,
    terminal_with_lines_mut as host_terminal_with_lines_mut,
};

#[derive(Clone, Debug)]
pub(crate) struct FrontendResolvedPane {
    pub runtime_entry_id: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct FrontendFocusedPane {
    pub runtime_entry_id: u64,
    pub pane_id: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct LauncherSessionEntry {
    pub title: String,
    pub tab_idx: usize,
    pub pane_count: Option<usize>,
}

static EMBEDDED_RUNTIME: OnceLock<Arc<EmbeddedRuntime>> = OnceLock::new();
static PRIMARY_HOST_WINDOW_ID: OnceLock<EngineWindowId> = OnceLock::new();

impl EmbeddedRuntime {
    pub(crate) fn session_engine_shared(&self) -> Arc<session_engine::SessionEngineShared> {
        self.bridge.shared()
    }

    pub(crate) fn global() -> Result<&'static Arc<Self>, String> {
        if let Some(runtime) = EMBEDDED_RUNTIME.get() {
            return Ok(runtime);
        }

        let bridge = Arc::new(DesktopRuntimeExecutionBridge::new());
        let bridge_dyn: Arc<dyn chatminal_runtime::RuntimeExecutionAdapter> =
            Arc::clone(&bridge) as _;
        let (state, _config) = RuntimeState::initialize_default(bridge_dyn)?;
        let runtime = Arc::new(Self { state, bridge });
        let _ = EMBEDDED_RUNTIME.set(runtime);
        EMBEDDED_RUNTIME
            .get()
            .ok_or_else(|| "failed to initialize embedded chatminal runtime".to_string())
    }
}

pub(crate) fn runtime_client() -> Result<ChatminalRuntimeClient, String> {
    let runtime = EmbeddedRuntime::global().map(Arc::clone)?;
    ChatminalRuntimeClient::new(runtime)
}

pub(crate) fn native_session_command(session_id: &str) -> Result<CommandBuilder, String> {
    let launch = runtime_client()?.session_launch_spec(session_id)?;
    let mut command = CommandBuilder::new(launch.shell);
    command.cwd(launch.cwd);
    Ok(command)
}

pub(crate) fn parse_proxy_session_id(builder: &CommandBuilder) -> Option<String> {
    let argv = builder.get_argv();
    if argv.len() < 2 {
        return None;
    }
    if argv
        .get(1)
        .and_then(|value| os_str_to_str(value.as_os_str()))
        .is_none_or(|value| value != DESKTOP_PROXY_COMMAND)
    {
        return None;
    }
    argv.get(2)
        .and_then(|value| os_str_to_str(value.as_os_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or(Some(String::new()))
}

fn os_str_to_str(value: &OsStr) -> Option<&str> {
    value.to_str()
}

fn terminal_handle_metadata_u64(pane: &dyn Pane, key: &str) -> Option<u64> {
    match pane.get_metadata() {
        Value::Object(map) => {
            map.get(&Value::String(key.to_string()))
                .and_then(|value| match value {
                    Value::U64(value) => Some(*value),
                    _ => None,
                })
        }
        _ => None,
    }
}

fn terminal_handle_terminal_instance_id(pane: &dyn Pane) -> Option<TerminalInstanceId> {
    terminal_handle_metadata_u64(pane, "chatminal_terminal_instance_id")
        .map(TerminalInstanceId::new)
}

fn terminal_handle_matches_public_id(pane: &dyn Pane, public_id: u64) -> bool {
    pane.pane_id() as u64 == public_id
        || terminal_handle_terminal_instance_id(pane)
            .map(|terminal_instance_id| terminal_instance_id.as_u64() == public_id)
            .unwrap_or(false)
}

pub(crate) fn host_active_render_scope_id() -> Option<u64> {
    Mux::get().root_active_tab().map(|tab| tab.tab_id() as u64)
}

fn primary_host_window_engine_id() -> EngineWindowId {
    *PRIMARY_HOST_WINDOW_ID
        .get()
        .expect("primary host window to be initialized")
}

pub(crate) fn host_render_scope_capability(render_scope_id: u64) -> Option<Arc<HostRenderScope>> {
    usize::try_from(render_scope_id)
        .ok()
        .and_then(host_runtime::runtime_entry_by_id)
}

fn with_host_render_scope_by_id<R, F>(render_scope_id: u64, func: F) -> Option<R>
where
    F: FnOnce(&Arc<HostRenderScope>) -> R,
{
    let render_scope = host_render_scope_capability(render_scope_id)?;
    Some(func(&render_scope))
}

pub(crate) fn host_render_scope_size(render_scope_id: u64) -> Option<TerminalSize> {
    with_host_render_scope_by_id(render_scope_id, |render_scope| render_scope.get_size())
}

pub(crate) fn host_render_scope_can_close_without_prompting(
    render_scope_id: u64,
    reason: HostCloseReason,
) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope.can_close_without_prompting(reason)
    })
    .unwrap_or(false)
}

pub(crate) fn host_overlay_pane_layouts_by_id(
    render_scope_id: u64,
) -> Vec<overlay_compat::OverlayPaneLayout> {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        overlay_pane_layouts(render_scope)
    })
    .unwrap_or_default()
}

pub(crate) fn host_overlay_split_layouts_by_id(
    render_scope_id: u64,
) -> Vec<overlay_compat::OverlaySplitLayout> {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        overlay_split_layouts(render_scope)
    })
    .unwrap_or_default()
}

pub(crate) fn host_resize_render_scope(render_scope_id: u64, size: TerminalSize) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope.resize(size);
        true
    })
    .unwrap_or(false)
}

pub(crate) fn host_resize_render_scope_split(
    render_scope_id: u64,
    split_index: usize,
    delta: isize,
) -> Option<overlay_compat::OverlaySplitLayout> {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope.resize_split_by(split_index, delta);
        overlay_split_layouts(render_scope)
            .into_iter()
            .nth(split_index)
    })
    .flatten()
}

pub(crate) fn host_set_render_scope_zoomed(render_scope_id: u64, zoomed: bool) -> Option<bool> {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope.set_zoomed(zoomed)
    })
}

pub(crate) fn host_toggle_render_scope_zoom(render_scope_id: u64) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| render_scope.toggle_zoom())
        .is_some()
}

pub(crate) fn host_adjust_render_scope_terminal_size(
    render_scope_id: u64,
    direction: SessionDirection,
    amount: usize,
) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope.adjust_pane_size(direction, amount);
        true
    })
    .is_some()
}

pub(crate) fn host_rotate_render_scope_terminals(
    render_scope_id: u64,
    direction: RotationDirection,
) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| match direction {
        RotationDirection::Clockwise => render_scope.rotate_clockwise(),
        RotationDirection::CounterClockwise => render_scope.rotate_counter_clockwise(),
    })
    .is_some()
}

pub(crate) fn host_swap_active_with_terminal_handle_in_render_scope(
    render_scope_id: u64,
    terminal_handle: u64,
    keep_focus: bool,
) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope
            .iter_panes()
            .iter()
            .position(|positioned| {
                terminal_handle_matches_public_id(&*positioned.pane, terminal_handle)
            })
            .and_then(|tab_index| render_scope.swap_active_with_index(tab_index, keep_focus))
            .is_some()
    })
    .unwrap_or(false)
}

pub(crate) fn host_active_terminal_in_render_scope(
    render_scope_id: u64,
) -> Option<Arc<dyn HostTerminal>> {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope.get_active_pane()
    })
    .flatten()
}

pub(crate) fn host_render_scope_contains_terminal(
    render_scope_id: u64,
    terminal_handle: u64,
) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope
            .iter_panes()
            .iter()
            .any(|positioned| terminal_handle_matches_public_id(&*positioned.pane, terminal_handle))
    })
    .unwrap_or(false)
}

pub(crate) fn host_activate_terminal_index_in_render_scope(
    render_scope_id: u64,
    index: usize,
) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        let panes = render_scope.iter_panes();
        if panes
            .iter()
            .position(|positioned| positioned.index == index)
            .is_some()
        {
            render_scope.set_active_idx(index);
            return true;
        }
        false
    })
    .unwrap_or(false)
}

pub(crate) fn host_activate_terminal_direction_in_render_scope(
    render_scope_id: u64,
    direction: SessionDirection,
) -> bool {
    with_host_render_scope_by_id(render_scope_id, |render_scope| {
        render_scope.activate_pane_direction(direction);
        true
    })
    .unwrap_or(false)
}

pub(crate) fn overlay_pane_layouts(
    render_scope: &HostRenderScope,
) -> Vec<overlay_compat::OverlayPaneLayout> {
    render_scope
        .iter_panes()
        .into_iter()
        .map(|layout| overlay_compat::OverlayPaneLayout {
            index: layout.index,
            is_active: layout.is_active,
            is_zoomed: layout.is_zoomed,
            left: layout.left,
            top: layout.top,
            width: layout.width,
            pixel_width: layout.pixel_width,
            height: layout.height,
            pixel_height: layout.pixel_height,
            pane: layout.pane,
        })
        .collect()
}

pub(crate) fn overlay_split_layouts(
    render_scope: &HostRenderScope,
) -> Vec<overlay_compat::OverlaySplitLayout> {
    render_scope
        .iter_splits()
        .into_iter()
        .map(|split| overlay_compat::OverlaySplitLayout {
            index: split.index,
            direction: split.direction,
            left: split.left,
            top: split.top,
            size: split.size,
        })
        .collect()
}

pub(crate) fn terminal_handle_arc(pane_id: HostTerminalHandle) -> Option<Arc<dyn HostTerminal>> {
    host_runtime::terminal_by_id(pane_id)
}

pub(crate) fn terminal_handle_arc_by_public_id(pane_id: u64) -> Option<Arc<dyn HostTerminal>> {
    let pane_id = HostTerminalHandle::try_from(pane_id).ok()?;
    terminal_handle_arc(pane_id)
}

pub(crate) fn remove_terminal_handle(pane_id: HostTerminalHandle) {
    Mux::get().remove_pane(pane_id);
}

pub(crate) fn with_host_window<R, F>(func: F) -> Option<R>
where
    F: FnOnce(&MuxWindow) -> R,
{
    let mux = Mux::get();
    let window = mux.root_window();
    Some(func(&window))
}

pub(crate) fn with_host_window_mut<R, F>(func: F) -> Option<R>
where
    F: FnOnce(&mut MuxWindow) -> R,
{
    let mux = Mux::get();
    let mut window = mux.root_window_mut();
    Some(func(&mut window))
}

pub(crate) fn remove_runtime_entry_scope(render_scope_id: u64) {
    Mux::get().remove_tab(render_scope_id as _);
}

pub(crate) fn host_window_exists() -> bool {
    with_host_window(|_| ()).is_some()
}

pub(crate) fn host_window_contains_render_scope(render_scope_id: u64) -> bool {
    Mux::get().get_tab(render_scope_id as _).is_some()
}

pub(crate) fn host_workspace_name() -> String {
    Mux::get().active_workspace().to_string()
}

pub(crate) fn set_host_workspace(name: &str) {
    Mux::get().set_active_workspace(name);
    let _ = with_host_window_mut(|window| {
        window.set_workspace(name);
    });
}

pub(crate) fn configured_default_workspace_name(config: &ConfigHandle) -> String {
    config
        .default_workspace
        .as_deref()
        .unwrap_or(host_runtime::DEFAULT_WORKSPACE)
        .to_string()
}

pub(crate) fn host_workspace_has_windows(name: &str) -> bool {
    with_host_window(|window| {
        window.get_workspace() == name
    })
    .unwrap_or(false)
}

pub(crate) fn focus_terminal_handle(pane_id: HostTerminalHandle) -> bool {
    Mux::get().focus_pane_and_containing_tab(pane_id).is_ok()
}

pub(crate) fn record_host_focus_for_current_identity(pane_id: HostTerminalHandle) {
    Mux::get().record_focus_for_current_identity(pane_id);
}

pub(crate) fn resize_host_window_tabs(size: TerminalSize) {
    let _ = with_host_window(|window| {
        for tab in window.iter() {
            tab.resize(size);
        }
    });
}

pub(crate) fn host_window_initial_position() -> Option<config::GuiPosition> {
    with_host_window(|window| window.get_initial_position().clone()).flatten()
}

pub(crate) fn active_host_runtime_entry_size() -> Option<TerminalSize> {
    with_host_window(|window| {
        window
            .get_by_idx(window.get_active_idx())
            .cloned()
            .map(|tab| tab.get_size())
    })
    .flatten()
}

pub(crate) fn subscribe_runtime_notifications<F>(subscriber: F)
where
    F: Fn(MuxNotification) -> bool + 'static + Send + Sync,
{
    Mux::get().subscribe(subscriber);
}

pub(crate) fn resolved_window_title() -> Option<String> {
    with_host_window(|window| window.get_title().to_string())
}

pub(crate) fn resolve_public_pane(
    host_terminal_handle: u64,
    terminal_instance_id: u64,
) -> Option<Arc<dyn HostTerminal>> {
    HostTerminalHandle::try_from(host_terminal_handle)
        .ok()
        .and_then(host_runtime::terminal_by_id)
        .or_else(|| {
            HostTerminalHandle::try_from(terminal_instance_id)
                .ok()
                .and_then(host_runtime::terminal_by_id)
        })
        .or_else(|| {
            Mux::try_get()?
                .iter_panes()
                .into_iter()
                .find(|pane| terminal_handle_matches_public_id(&**pane, terminal_instance_id))
        })
}

pub(crate) fn launcher_sessions() -> Vec<LauncherSessionEntry> {
    with_host_window(|window| {
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

pub(crate) fn active_frontend_client() -> Option<FrontendClientHandle> {
    Mux::get().active_identity()
}

pub(crate) fn subscribe_frontend_notifications<F>(subscriber: F)
where
    F: Fn(MuxNotification) -> bool + 'static + Send + Sync,
{
    Mux::get().subscribe(subscriber);
}

pub(crate) fn primary_host_window_id() -> u64 {
    primary_host_window_engine_id() as u64
}

pub(crate) fn primary_host_window_exists() -> bool {
    host_window_exists()
}

pub(crate) fn active_workspace_for_client(client_id: &FrontendClientHandle) -> String {
    Mux::get().active_workspace_for_client(client_id)
}

pub(crate) fn set_active_workspace_for_client(client_id: &FrontendClientHandle, workspace: &str) {
    Mux::get().set_active_workspace_for_client(client_id, workspace);
}

pub(crate) fn workspace_is_empty(workspace: &str) -> bool {
    Mux::get().is_workspace_empty(workspace)
}

pub(crate) fn workspace_names() -> Vec<String> {
    Mux::get().iter_workspaces()
}

pub(crate) fn focus_terminal_handle_by_id(pane_id: u64) -> anyhow::Result<()> {
    let pane_id =
        HostTerminalHandle::try_from(pane_id).map_err(|_| anyhow!("invalid pane id {pane_id}"))?;
    Mux::get()
        .focus_pane_and_containing_tab(pane_id)
        .map_err(anyhow::Error::from)
}

pub(crate) fn frontend_resolve_pane(pane_id: u64) -> Option<FrontendResolvedPane> {
    let pane_id = HostTerminalHandle::try_from(pane_id).ok()?;
    let runtime_entry_id = Mux::get().resolve_pane_id(pane_id)?;
    Some(FrontendResolvedPane {
        runtime_entry_id: runtime_entry_id as u64,
    })
}

pub(crate) fn frontend_resolve_focused_pane(
    client_id: &FrontendClientHandle,
) -> Option<FrontendFocusedPane> {
    let (runtime_entry_id, pane_id) = Mux::get().resolve_focused_pane(client_id)?;
    Some(FrontendFocusedPane {
        runtime_entry_id: runtime_entry_id as u64,
        pane_id: pane_id as u64,
    })
}

pub(crate) async fn spawn_local_shell_runner() -> anyhow::Result<Arc<dyn HostTerminal>> {
    let (_runtime_entry, pane) = Mux::get()
        .spawn_tab(None, None, TerminalSize::default(), None)
        .await?;
    Ok(pane)
}

pub(crate) async fn spawn_host_runtime_entry(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    current_pane_id: Option<u64>,
    workspace: String,
    _position: Option<config::GuiPosition>,
) -> anyhow::Result<Arc<dyn HostTerminal>> {
    set_host_workspace(&workspace);
    let (_runtime_entry, pane) = Mux::get()
        .spawn_tab(
            command,
            command_dir,
            size,
            current_pane_id.and_then(|pane_id| HostTerminalHandle::try_from(pane_id).ok()),
        )
        .await?;
    Ok(pane)
}

pub(crate) fn set_host_spawn_target(spawn_target: &HostSpawnTargetHandle) {
    Mux::get().set_primary_spawn_target(spawn_target);
}

pub(crate) fn primary_host_spawn_target() -> HostSpawnTargetHandle {
    Mux::get().primary_spawn_target()
}
pub(crate) fn build_initial_host_mux(
    config: &ConfigHandle,
    default_workspace_name: Option<&str>,
) -> anyhow::Result<()> {
    let desktop_spawn_target: HostSpawnTargetHandle =
        Arc::new(spawn_target::DesktopSpawnTarget::new_local()?);
    let mux = Arc::new(Mux::new(Some(desktop_spawn_target)));
    Mux::set_mux(&mux);
    let client_id = Arc::new(ClientId::new());
    mux.register_client(client_id.clone());
    mux.replace_identity(Some(client_id));

    let workspace = default_workspace_name
        .map(str::to_string)
        .unwrap_or_else(|| configured_default_workspace_name(config));
    mux.set_active_workspace(&workspace);
    let _ = PRIMARY_HOST_WINDOW_ID.set(ROOT_WINDOW_ID);
    Ok(())
}

pub(crate) fn host_activity_count() -> usize {
    Activity::count()
}

pub(crate) fn start_host_activity() -> HostActivityGuard {
    HostActivityGuard(Activity::new())
}

pub(crate) fn show_host_configuration_error_message(err: &str) {
    log::error!("Configuration Error: {}", err);
}

pub(crate) fn create_serial_spawn_target(
    serial_target: config::SerialTarget,
) -> anyhow::Result<HostSpawnTargetHandle> {
    Ok(Arc::new(spawn_target::DesktopSpawnTarget::new_serial(
        serial_target,
    )?))
}

pub(crate) fn shutdown_host_mux() {
    Mux::shutdown();
}

pub(crate) fn activate_host_runtime_entry(render_scope_id: u64) -> anyhow::Result<()> {
    let render_scope_id = usize::try_from(render_scope_id)
        .map_err(|_| anyhow!("invalid runtime entry id {render_scope_id}"))?;
    let mux = Mux::get();
    let mut window = mux.root_window_mut();
    let tab_idx = window.idx_by_id(render_scope_id).ok_or_else(|| {
        anyhow!("runtime entry {render_scope_id} not attached to root window")
    })?;
    window.set_active_without_saving(tab_idx);
    Ok(())
}

pub(crate) fn host_has_panes_in_workspace(workspace: Option<&str>) -> bool {
    let mux = Mux::get();
    if mux.iter_panes().is_empty() {
        return false;
    }
    let Some(workspace) = workspace else {
        return true;
    };
    if !host_workspace_has_windows(workspace) {
        return false;
    }
    with_host_window(|window| {
        window.iter().any(|runtime_entry| {
            runtime_entry
                .iter_panes_ignoring_zoom()
                .iter()
                .next()
                .is_some()
        })
    })
    .unwrap_or(false)
}
