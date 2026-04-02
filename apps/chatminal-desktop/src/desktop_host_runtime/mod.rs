pub(crate) mod execution_bridge;
pub(crate) mod session_engine;
mod session_host;
mod session_pane;
mod spawn_target;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

use crate::chatminal_runtime::ChatminalRuntimeClient;
use anyhow::anyhow;
use chatminal_runtime::{RuntimeId, RuntimeState, SessionTerminalHandle};
use config::keyassignment::{RotationDirection, SessionDirection};
use config::ConfigHandle;
use engine_dynamic::Value;
use engine_term::{ClipboardSelection, TerminalSize};
use host_runtime::activity::Activity;
use host_runtime::client::ClientId;
use host_runtime::pane::Pane;
use host_runtime::spawn_target::SpawnTarget;
use host_runtime::window::{Window as MuxWindow, WindowId as EngineWindowId};
use host_runtime::{HostRuntimeNotification, MuxHandle};
use portable_pty::CommandBuilder;
use session_engine::TerminalInstanceId;

pub(crate) use execution_bridge::DesktopRuntimeExecutionBridge;
pub(crate) use session_host::{
    get_or_init_session_host, legacy_activate_runtime_entry, legacy_active_frontend_client,
    legacy_active_render_scope_id, legacy_active_workspace_for_client,
    legacy_build_initial_host_mux, legacy_create_serial_spawn_target,
    legacy_focus_terminal_handle_by_id, legacy_frontend_resolve_focused_pane,
    legacy_frontend_resolve_pane, legacy_has_panes_in_workspace,
    legacy_host_window_contains_render_scope, legacy_host_workspace_name,
    legacy_primary_spawn_target, legacy_record_input_for_current_identity,
    legacy_remove_runtime_entry_scope, legacy_resolve_public_pane_fallback,
    legacy_set_active_workspace_for_client, legacy_set_primary_spawn_target,
    legacy_shutdown_host_mux, legacy_spawn_host_runtime_entry, legacy_spawn_local_shell_runner,
    legacy_with_window, legacy_with_window_mut, legacy_workspace_has_windows,
    legacy_workspace_is_empty, legacy_workspace_names, DesktopSessionHost,
};
pub(crate) use session_pane::ChatminalSessionPane;

pub(crate) const CHATMINAL_RUNTIME_SPAWN_TARGET_NAME: &str = "chatminal-runtime";
pub(crate) const DESKTOP_PROXY_COMMAND: &str = "proxy-desktop-session";
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
pub(crate) type RuntimeWindow = MuxWindow;
pub(crate) type PrimaryHostWindowId = EngineWindowId;
pub(crate) use host_runtime::pane::Pane as HostTerminal;
pub(crate) use host_runtime::spawn_target::SpawnTarget as HostSpawnTarget;
pub(crate) type HostTerminalHandle = usize;
pub(crate) type HostCachePolicy = host_runtime::pane::CachePolicy;
pub(crate) type HostCloseReason = host_runtime::pane::CloseReason;
pub(crate) type HostLogicalLine = host_runtime::pane::LogicalLine;
pub(crate) type HostSearchResult = host_runtime::pane::SearchResult;
pub(crate) type HostPattern = host_runtime::pane::Pattern;
pub(crate) type HostRenderableDimensions = host_runtime::renderable::RenderableDimensions;
pub(crate) type HostStableCursorPosition = host_runtime::renderable::StableCursorPosition;
pub(crate) type HostRenderScope = host_runtime::tab::Tab;

pub(crate) use host_runtime::alloc_terminal_handle_value as alloc_host_terminal_handle;
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
    pub runtime_id: RuntimeId,
}

#[derive(Clone, Debug)]
pub(crate) struct FrontendFocusedPane {
    pub runtime_id: RuntimeId,
    pub terminal_handle: SessionTerminalHandle,
}

#[derive(Clone, Debug)]
pub(crate) struct LauncherSessionEntry {
    pub title: String,
    pub tab_idx: usize,
    pub pane_count: Option<usize>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) enum RuntimeNotification {
    PaneOutput(SessionTerminalHandle),
    PaneAdded(SessionTerminalHandle),
    PaneRemoved(SessionTerminalHandle),
    WindowInvalidated,
    WindowWorkspaceChanged,
    ActiveWorkspaceChanged(FrontendClientHandle),
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

impl From<HostRuntimeNotification> for RuntimeNotification {
    fn from(notification: HostRuntimeNotification) -> Self {
        match notification {
            HostRuntimeNotification::PaneOutput(pane_id) => Self::PaneOutput(pane_id),
            HostRuntimeNotification::PaneAdded(pane_id) => Self::PaneAdded(pane_id),
            HostRuntimeNotification::PaneRemoved(pane_id) => Self::PaneRemoved(pane_id),
            HostRuntimeNotification::WindowInvalidated => Self::WindowInvalidated,
            HostRuntimeNotification::WindowWorkspaceChanged => Self::WindowWorkspaceChanged,
            HostRuntimeNotification::ActiveWorkspaceChanged(client_id) => {
                Self::ActiveWorkspaceChanged(client_id)
            }
            HostRuntimeNotification::Alert { pane_id, alert } => Self::Alert { pane_id, alert },
            HostRuntimeNotification::Empty => Self::Empty,
            HostRuntimeNotification::AssignClipboard {
                pane_id,
                selection,
                clipboard,
            } => Self::AssignClipboard {
                pane_id,
                selection,
                clipboard,
            },
            HostRuntimeNotification::SaveToDownloads { name, data } => {
                Self::SaveToDownloads { name, data }
            }
            HostRuntimeNotification::TabAddedToWindow { runtime_id } => {
                Self::TabAddedToWindow { runtime_id }
            }
            HostRuntimeNotification::PaneFocused(pane_id) => Self::PaneFocused(pane_id),
            HostRuntimeNotification::TabResized(runtime_id) => Self::TabResized(runtime_id),
            HostRuntimeNotification::TabTitleChanged { runtime_id, title } => {
                Self::TabTitleChanged { runtime_id, title }
            }
            HostRuntimeNotification::WindowTitleChanged { title } => {
                Self::WindowTitleChanged { title }
            }
            HostRuntimeNotification::WorkspaceRenamed {
                old_workspace,
                new_workspace,
            } => Self::WorkspaceRenamed {
                old_workspace,
                new_workspace,
            },
        }
    }
}

type RuntimeNotificationSubscriber = Box<dyn Fn(RuntimeNotification) -> bool + Send + Sync>;

struct RuntimeNotificationHub {
    subscribers: Mutex<HashMap<usize, RuntimeNotificationSubscriber>>,
}

impl RuntimeNotificationHub {
    fn subscribe<F>(&self, subscriber: F)
    where
        F: Fn(RuntimeNotification) -> bool + 'static + Send + Sync,
    {
        let subscriber_id = RUNTIME_NOTIFICATION_SUBSCRIBER_ID.fetch_add(1, Ordering::Relaxed);
        self.subscribers
            .lock()
            .unwrap()
            .insert(subscriber_id, Box::new(subscriber));
    }

    fn publish(&self, notification: RuntimeNotification) {
        self.subscribers
            .lock()
            .unwrap()
            .retain(|_, subscriber| subscriber(notification.clone()));
    }
}

static RUNTIME_NOTIFICATION_SUBSCRIBER_ID: AtomicUsize = AtomicUsize::new(1);
static RUNTIME_NOTIFICATION_HUB: OnceLock<RuntimeNotificationHub> = OnceLock::new();

static EMBEDDED_RUNTIME: OnceLock<Arc<EmbeddedRuntime>> = OnceLock::new();
static PRIMARY_HOST_WINDOW_ID: OnceLock<EngineWindowId> = OnceLock::new();

fn runtime_notification_hub() -> &'static RuntimeNotificationHub {
    RUNTIME_NOTIFICATION_HUB.get_or_init(|| RuntimeNotificationHub {
        subscribers: Mutex::new(HashMap::new()),
    })
}

pub(crate) fn subscribe_desktop_runtime_notifications<F>(subscriber: F)
where
    F: Fn(RuntimeNotification) -> bool + 'static + Send + Sync,
{
    runtime_notification_hub().subscribe(subscriber);
}

pub(crate) fn publish_runtime_notification(notification: RuntimeNotification) {
    runtime_notification_hub().publish(notification);
}

pub(crate) fn publish_runtime_notification_from_any_thread(notification: RuntimeNotification) {
    if !promise::spawn::is_scheduler_configured() {
        publish_runtime_notification(notification);
        return;
    }
    promise::spawn::spawn_into_main_thread(async move {
        publish_runtime_notification(notification);
    })
    .detach();
}

pub(crate) fn bridge_mux_notifications(mux_handle: &Arc<MuxHandle>) {
    mux_handle.subscribe(|notification| {
        publish_runtime_notification(notification.into());
        true
    });
}

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
    host_runtime::terminal_handle_for_pane(pane).as_u64() == public_id
        || terminal_handle_terminal_instance_id(pane)
            .map(|terminal_instance_id| terminal_instance_id.as_u64() == public_id)
            .unwrap_or(false)
}

pub(crate) fn host_active_render_scope_id() -> Option<u64> {
    crate::chatminal_runtime::desktop_current_active_session_id()
        .as_deref()
        .and_then(crate::chatminal_runtime::desktop_render_state_for_session)
        .map(|state| state.render_target_id().as_u64())
        .or_else(|| {
            crate::chatminal_runtime::desktop_session_host()
                .and_then(|host| host.active_render_scope_id())
        })
        .or_else(legacy_active_render_scope_id)
}

fn primary_host_window_engine_id() -> EngineWindowId {
    *PRIMARY_HOST_WINDOW_ID
        .get()
        .expect("primary host window to be initialized")
}

pub(crate) fn host_render_scope_capability(render_scope_id: u64) -> Option<Arc<HostRenderScope>> {
    host_runtime::runtime_entry_by_runtime_id(RuntimeId::new(render_scope_id))
}

fn with_host_render_scope_by_id<R, F>(render_scope_id: u64, func: F) -> Option<R>
where
    F: FnOnce(&Arc<HostRenderScope>) -> R,
{
    let render_scope = host_render_scope_capability(render_scope_id)?;
    Some(func(&render_scope))
}

pub(crate) fn host_render_scope_size(render_scope_id: u64) -> Option<TerminalSize> {
    if let Some(session_id) = crate::chatminal_runtime::desktop_session_id_for_render_target(
        chatminal_runtime::SessionRenderTargetId::new(render_scope_id),
    ) {
        return crate::chatminal_runtime::desktop_render_state_for_session(&session_id)
            .map(|state| state.terminal_size);
    }
    host_runtime::runtime_entry_info_by_runtime_id(RuntimeId::new(render_scope_id))
        .map(|info| info.size)
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
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        if let Some(pane) = host.pane_for_terminal_handle(pane_id) {
            return Some(pane);
        }
    }
    host_runtime::terminal_by_handle(SessionTerminalHandle::new(pane_id as u64))
}

pub(crate) fn terminal_handle_arc_by_public_id(pane_id: u64) -> Option<Arc<dyn HostTerminal>> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        if let Some(pane) = host.pane_for_public_id(pane_id) {
            return Some(pane);
        }
    }
    let pane_id = HostTerminalHandle::try_from(pane_id).ok()?;
    terminal_handle_arc(pane_id)
}

pub(crate) fn remove_terminal_handle(pane_id: HostTerminalHandle) {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.remove_terminal_handle(pane_id);
        return;
    }
    let _ = host_runtime::remove_terminal_handle(SessionTerminalHandle::new(pane_id as u64));
}

pub(crate) fn with_host_window<R, F>(func: F) -> Option<R>
where
    F: FnOnce(&MuxWindow) -> R,
{
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.with_window(func);
    }
    legacy_with_window(func)
}

pub(crate) fn with_host_window_mut<R, F>(func: F) -> Option<R>
where
    F: FnOnce(&mut MuxWindow) -> R,
{
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.with_window_mut(func);
    }
    legacy_with_window_mut(func)
}

pub(crate) fn remove_runtime_entry_scope(render_scope_id: u64) {
    if crate::chatminal_runtime::desktop_close_view_or_session_for_render_target(
        chatminal_runtime::SessionRenderTargetId::new(render_scope_id),
    ) {
        return;
    }
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.remove_runtime_entry_scope(render_scope_id);
        return;
    }
    legacy_remove_runtime_entry_scope(render_scope_id);
}

pub(crate) fn host_window_exists() -> bool {
    crate::chatminal_runtime::desktop_session_host()
        .map(|host| host.window_exists())
        .unwrap_or_else(|| with_host_window(|_| ()).is_some())
}

pub(crate) fn host_window_contains_render_scope(render_scope_id: u64) -> bool {
    crate::chatminal_runtime::desktop_session_host().is_some_and(|host| {
        host.render_state_for_runtime(RuntimeId::new(render_scope_id))
            .is_some()
            || host.host_window_contains_render_scope(render_scope_id)
    }) || legacy_host_window_contains_render_scope(render_scope_id)
}

pub(crate) fn host_workspace_name() -> String {
    crate::chatminal_runtime::desktop_session_host()
        .map(|host| host.host_workspace_name())
        .unwrap_or_else(legacy_host_workspace_name)
}

pub(crate) fn configured_default_workspace_name(config: &ConfigHandle) -> String {
    config
        .default_workspace
        .as_deref()
        .unwrap_or(host_runtime::DEFAULT_WORKSPACE)
        .to_string()
}

pub(crate) fn host_workspace_has_windows(name: &str) -> bool {
    crate::chatminal_runtime::desktop_session_host()
        .map(|host| host.workspace_has_windows(name))
        .unwrap_or_else(|| legacy_workspace_has_windows(name))
}

pub(crate) fn record_host_focus_for_current_identity(pane_id: HostTerminalHandle) {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.record_focus_for_current_identity(pane_id);
        return;
    }
    let _ =
        host_runtime::record_focus_for_terminal_handle(SessionTerminalHandle::new(pane_id as u64));
}

pub(crate) fn record_host_input_for_current_identity() {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.record_input_for_current_identity();
        return;
    }
    legacy_record_input_for_current_identity();
}

pub(crate) fn resize_host_window_tabs(size: TerminalSize) {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.resize_window_tabs(size);
        return;
    }
    let _ = with_host_window(|window| {
        for tab in window.iter() {
            tab.resize(size);
        }
    });
}

pub(crate) fn host_window_initial_position() -> Option<config::GuiPosition> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.host_window_initial_position();
    }
    with_host_window(|window| window.get_initial_position().clone()).flatten()
}

pub(crate) fn active_host_runtime_entry_size() -> Option<TerminalSize> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.active_runtime_entry_size();
    }
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
    F: Fn(RuntimeNotification) -> bool + 'static + Send + Sync,
{
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.subscribe_notifications(subscriber);
        return;
    }
    subscribe_desktop_runtime_notifications(subscriber);
}

pub(crate) fn resolved_window_title() -> Option<String> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.resolved_window_title();
    }
    with_host_window(|window| window.get_title().to_string())
}

pub(crate) fn resolve_public_pane(
    host_terminal_handle: u64,
    terminal_instance_id: u64,
) -> Option<Arc<dyn HostTerminal>> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.resolve_public_pane_fallback(host_terminal_handle, terminal_instance_id);
    }

    legacy_resolve_public_pane_fallback(host_terminal_handle, terminal_instance_id)
}

pub(crate) fn launcher_sessions() -> Vec<LauncherSessionEntry> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.launcher_sessions();
    }
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
    crate::chatminal_runtime::desktop_session_host()
        .and_then(|host| host.active_frontend_client())
        .or_else(legacy_active_frontend_client)
}

pub(crate) fn subscribe_frontend_notifications<F>(subscriber: F)
where
    F: Fn(RuntimeNotification) -> bool + 'static + Send + Sync,
{
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.subscribe_notifications(subscriber);
        return;
    }
    subscribe_desktop_runtime_notifications(subscriber);
}

pub(crate) fn primary_host_window_id() -> u64 {
    primary_host_window_engine_id() as u64
}

pub(crate) fn primary_host_window_exists() -> bool {
    host_window_exists()
}

pub(crate) fn active_workspace_for_client(client_id: &FrontendClientHandle) -> String {
    crate::chatminal_runtime::desktop_session_host()
        .map(|host| host.active_workspace_for_client(client_id))
        .unwrap_or_else(|| legacy_active_workspace_for_client(client_id))
}

pub(crate) fn set_active_workspace_for_client(client_id: &FrontendClientHandle, workspace: &str) {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.set_active_workspace_for_client(client_id, workspace);
        return;
    }
    legacy_set_active_workspace_for_client(client_id, workspace);
}

pub(crate) fn workspace_is_empty(workspace: &str) -> bool {
    crate::chatminal_runtime::desktop_session_host()
        .map(|host| host.workspace_is_empty(workspace))
        .unwrap_or_else(|| legacy_workspace_is_empty(workspace))
}

pub(crate) fn workspace_names() -> Vec<String> {
    crate::chatminal_runtime::desktop_session_host()
        .map(|host| host.workspace_names())
        .unwrap_or_else(legacy_workspace_names)
}

pub(crate) fn focus_terminal_handle_by_id(pane_id: SessionTerminalHandle) -> anyhow::Result<()> {
    if crate::chatminal_runtime::desktop_focus_session_terminal_handle(pane_id).is_some() {
        return Ok(());
    }
    legacy_focus_terminal_handle_by_id(pane_id)
}

pub(crate) fn frontend_resolve_pane(
    pane_id: SessionTerminalHandle,
) -> Option<FrontendResolvedPane> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.frontend_resolve_pane_fallback(pane_id);
    }

    legacy_frontend_resolve_pane(pane_id)
}

pub(crate) fn frontend_resolve_focused_pane(
    client_id: &FrontendClientHandle,
) -> Option<FrontendFocusedPane> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.frontend_resolve_focused_pane_fallback(client_id);
    }
    legacy_frontend_resolve_focused_pane(client_id)
}

pub(crate) async fn spawn_local_shell_runner() -> anyhow::Result<Arc<dyn HostTerminal>> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.spawn_local_shell_runner().await;
    }
    legacy_spawn_local_shell_runner().await
}

pub(crate) async fn spawn_host_runtime_entry(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    current_pane_id: Option<u64>,
    workspace: String,
    _position: Option<config::GuiPosition>,
) -> anyhow::Result<Arc<dyn HostTerminal>> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host
            .spawn_host_runtime_entry(command, command_dir, size, current_pane_id, workspace)
            .await;
    }
    legacy_spawn_host_runtime_entry(command, command_dir, size, current_pane_id, workspace).await
}

pub(crate) fn set_host_spawn_target(spawn_target: &HostSpawnTargetHandle) {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.set_primary_spawn_target(spawn_target);
        return;
    }
    legacy_set_primary_spawn_target(spawn_target);
}

pub(crate) fn primary_host_spawn_target() -> HostSpawnTargetHandle {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.primary_spawn_target();
    }
    legacy_primary_spawn_target()
}
pub(crate) fn build_initial_host_mux(
    config: &ConfigHandle,
    default_workspace_name: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.build_initial_host_mux(config, default_workspace_name);
    }
    legacy_build_initial_host_mux(config, default_workspace_name)
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
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.create_serial_spawn_target(serial_target);
    }
    legacy_create_serial_spawn_target(serial_target)
}

pub(crate) fn shutdown_host_mux() {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        host.shutdown_host_mux();
        return;
    }
    legacy_shutdown_host_mux();
}

pub(crate) fn activate_host_runtime_entry(render_scope_id: u64) -> anyhow::Result<()> {
    if let Some(session_id) = crate::chatminal_runtime::desktop_session_id_for_render_target(
        chatminal_runtime::SessionRenderTargetId::new(render_scope_id),
    ) {
        let size = crate::chatminal_runtime::desktop_render_state_for_session(&session_id)
            .map(|state| state.terminal_size)
            .ok_or_else(|| {
                anyhow!("missing render state for session render target {render_scope_id}")
            })?;
        crate::chatminal_runtime::desktop_activate_session(&session_id, None, size)
            .ok_or_else(|| anyhow!("failed to activate session render target {render_scope_id}"))?;
        return Ok(());
    }

    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.activate_runtime_entry(render_scope_id);
    }
    legacy_activate_runtime_entry(render_scope_id)
}

pub(crate) fn host_has_panes_in_workspace(workspace: Option<&str>) -> bool {
    if let Some(host) = crate::chatminal_runtime::desktop_session_host() {
        return host.has_panes_in_workspace(workspace);
    }
    legacy_has_panes_in_workspace(workspace)
}
