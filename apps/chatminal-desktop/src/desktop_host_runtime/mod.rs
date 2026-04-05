mod activity;
mod lua_bridge_backend;
pub(crate) mod session_engine;
mod session_host;
mod session_pane;
mod spawn_target;
mod window;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::chatminal_runtime::{
    ChatminalRuntimeClient, desktop_activate_session,
    desktop_close_view_or_session_for_render_target, desktop_focus_session_terminal_handle,
    desktop_render_state_for_session, desktop_session_host, desktop_session_id_for_render_target,
};
use activity::Activity;
use anyhow::anyhow;
use chatminal_runtime::{
    ClientId, FocusedPaneBinding, HostRuntimeNotification, RenderableDimensions, RuntimeEntryInfo,
    RuntimeEntryTerminalInfo, RuntimeId, RuntimeState, SessionTerminalHandle, StableCursorPosition,
};
use config::ConfigHandle;
use config::keyassignment::SessionDirection;
use engine_dynamic::Value;
use engine_term::{ClipboardSelection, TerminalSize};
use portable_pty::CommandBuilder;
use window::{Window as HostWindow, WindowId as HostWindowId};

use lua_bridge_backend::DesktopLuaBridgeBackend;
pub(crate) use session_host::{DesktopSessionHost, get_or_init_session_host};
use session_host::terminal_handle_for_host_terminal;
pub(crate) use session_pane::ChatminalSessionPane;

pub(crate) const CHATMINAL_RUNTIME_SPAWN_TARGET_NAME: &str = "chatminal-runtime";
pub(crate) const DESKTOP_PROXY_COMMAND: &str = "proxy-desktop-session";

pub(crate) struct HostActivityGuard(Activity);

pub(crate) mod overlay_shell {
    use chatminal_runtime::SplitDirection;

    pub use chatminal_runtime::pane::{
        CachePolicy as OverlayCachePolicy, CloseReason as OverlayCloseReason,
        ForEachPaneLogicalLine as OverlayForEachLogicalLine, LogicalLine as OverlayLogicalLine,
        Pane as OverlayPane, PerformAssignmentResult as OverlayAssignmentResult,
        WithPaneLines as OverlayWithPaneLines,
    };
    pub use chatminal_runtime::renderable::*;
    pub use chatminal_runtime::termwiztermtab::{
        TermWizTerminal as OverlayTerminal, allocate as allocate_overlay_terminal,
    };
    pub use chatminal_runtime::{
        Pattern as OverlayPattern, PatternType as OverlayPatternType,
        SearchResult as OverlaySearchResult,
    };
    pub type OverlaySplitDirection = SplitDirection;
    pub type OverlayRuntimeEntryHandle = chatminal_runtime::RuntimeId;

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
}

#[async_trait::async_trait(?Send)]
pub(crate) trait DesktopSpawnTargetBackend: Send + Sync {
    async fn spawn(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<HostSpawnedRuntimeEntry>;

    fn spawn_target_name(&self) -> &str;
}

#[derive(Clone)]
pub(crate) struct HostSpawnTargetHandle(Arc<dyn DesktopSpawnTargetBackend>);

pub(crate) struct HostSpawnedRuntimeEntry {
    pub runtime_id: RuntimeId,
    pub pane: Arc<dyn HostTerminal>,
}

impl HostSpawnTargetHandle {
    pub(crate) fn new(inner: Arc<dyn DesktopSpawnTargetBackend>) -> Self {
        Self(inner)
    }

    pub(crate) async fn spawn(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<HostSpawnedRuntimeEntry> {
        self.0.spawn(size, command, command_dir).await
    }

    pub(crate) fn spawn_target_name(&self) -> &str {
        self.0.spawn_target_name()
    }
}



pub(crate) type FrontendClientHandle = Arc<ClientId>;
pub(crate) type RuntimeWindow = HostWindow;
pub(crate) type PrimaryHostWindowId = HostWindowId;
pub(crate) type HostFocusedPaneBinding = FocusedPaneBinding;
pub(crate) use chatminal_runtime::pane::Pane as HostTerminal;
pub(crate) type HostTerminalHandle = SessionTerminalHandle;
pub(crate) type HostCachePolicy = chatminal_runtime::pane::CachePolicy;
pub(crate) type HostCloseReason = chatminal_runtime::pane::CloseReason;
pub(crate) type HostLogicalLine = chatminal_runtime::pane::LogicalLine;
pub(crate) type HostSearchResult = chatminal_runtime::SearchResult;
pub(crate) type HostPattern = chatminal_runtime::Pattern;
pub(crate) type HostRenderableDimensions = RenderableDimensions;
pub(crate) type HostStableCursorPosition = StableCursorPosition;
pub(crate) const ROOT_HOST_WINDOW_ID: PrimaryHostWindowId = window::ROOT_WINDOW_ID;

pub(crate) use chatminal_runtime::pane::alloc_pane_id as alloc_host_terminal_handle;
pub(crate) use chatminal_runtime::pane::impl_get_logical_lines_via_get_lines as host_impl_get_logical_lines_via_get_lines;
pub(crate) use chatminal_runtime::renderable::{
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
static PRIMARY_HOST_WINDOW_ID: OnceLock<HostWindowId> = OnceLock::new();

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

impl EmbeddedRuntime {
    pub(crate) fn session_engine_shared(&self) -> Arc<session_engine::SessionEngineShared> {
        self.state.session_engine_shared()
    }

    pub(crate) fn global() -> Result<&'static Arc<Self>, String> {
        if let Some(runtime) = EMBEDDED_RUNTIME.get() {
            return Ok(runtime);
        }

        let (state, _config) = RuntimeState::initialize_default()?;
        install_lua_bridge_backend_once();
        let runtime = Arc::new(Self { state });
        let _ = EMBEDDED_RUNTIME.set(runtime);
        EMBEDDED_RUNTIME
            .get()
            .ok_or_else(|| "failed to initialize embedded chatminal runtime".to_string())
    }
}

pub(crate) fn install_lua_bridge_backend_once() {
    let _ = chatminal_lua_bridge::install_backend(Arc::new(DesktopLuaBridgeBackend));
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

pub(crate) fn terminal_handle_for_pane(pane: &dyn HostTerminal) -> SessionTerminalHandle {
    terminal_handle_for_host_terminal(pane)
}

pub(crate) fn session_id_from_pane_metadata(pane: &dyn HostTerminal) -> Option<String> {
    match pane.get_metadata() {
        Value::Object(map) => {
            map.get_by_str("chatminal_session_id")
                .and_then(|value| match value {
                    Value::String(session_id) if !session_id.is_empty() => Some(session_id.clone()),
                    _ => None,
                })
        }
        _ => None,
    }
}

pub(crate) fn host_root_runtime_entry_infos() -> Vec<RuntimeEntryInfo> {
    desktop_session_host()
        .map(|host| host.root_runtime_entry_infos())
        .unwrap_or_default()
}

pub(crate) fn host_runtime_entry_info_by_session_id(session_id: &str) -> Option<RuntimeEntryInfo> {
    desktop_session_host().and_then(|host| host.runtime_entry_info_by_session_id(session_id))
}

pub(crate) fn host_root_active_runtime_id() -> Option<RuntimeId> {
    desktop_session_host().and_then(|host| host.root_active_runtime_id())
}

pub(crate) fn host_runtime_available() -> bool {
    desktop_session_host()
        .map(|host| host.runtime_available())
        .unwrap_or(false)
}

pub(crate) fn host_iter_all_panes() -> Vec<Arc<dyn HostTerminal>> {
    desktop_session_host()
        .map(|host| host.iter_all_panes())
        .unwrap_or_default()
}

pub(crate) async fn host_spawn_tab_raw(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    _current_terminal_handle: Option<SessionTerminalHandle>,
) -> anyhow::Result<HostSpawnedRuntimeEntry> {
    spawn_in_primary_target(size, command, command_dir).await
}


pub(crate) fn host_root_window_workspace_name_value() -> Option<String> {
    desktop_session_host().and_then(|host| host.root_window_workspace_name())
}

pub(crate) fn set_host_root_window_workspace_name(workspace: &str) -> bool {
    desktop_session_host()
        .map(|host| host.set_root_window_workspace_name(workspace))
        .unwrap_or(false)
}

pub(crate) fn set_host_active_workspace_name(workspace: &str) -> bool {
    desktop_session_host()
        .map(|host| host.set_active_workspace_name(workspace))
        .unwrap_or(false)
}

pub(crate) fn rename_host_workspace(old_workspace: &str, new_workspace: &str) -> bool {
    desktop_session_host()
        .map(|host| host.rename_workspace(old_workspace, new_workspace))
        .unwrap_or(false)
}

pub(crate) fn host_root_window_title() -> Option<String> {
    desktop_session_host().and_then(|host| host.root_window_title())
}

pub(crate) fn set_host_root_window_title(title: &str) -> bool {
    desktop_session_host()
        .map(|host| host.set_root_window_title(title))
        .unwrap_or(false)
}

pub(crate) fn host_root_window_spawn_context() -> (TerminalSize, Option<SessionTerminalHandle>) {
    desktop_session_host()
        .map(|host| host.root_window_spawn_context())
        .unwrap_or((TerminalSize::default(), None))
}

pub(crate) fn set_host_runtime_entry_title_by_session_id(session_id: &str, title: &str) -> bool {
    desktop_session_host()
        .map(|host| host.set_runtime_entry_title_by_session_id(session_id, title))
        .unwrap_or(false)
}

pub(crate) fn host_runtime_entry_exists_for_session(session_id: &str) -> bool {
    desktop_session_host()
        .map(|host| host.runtime_entry_exists_for_session(session_id))
        .unwrap_or(false)
}

pub(crate) fn host_runtime_entry_terminal_handles_by_session_id(
    session_id: &str,
) -> Vec<SessionTerminalHandle> {
    desktop_session_host()
        .map(|host| host.runtime_entry_terminal_handles_by_session_id(session_id))
        .unwrap_or_default()
}

pub(crate) fn host_runtime_entry_terminal_handle_in_direction_by_session_id(
    session_id: &str,
    direction: SessionDirection,
) -> Option<SessionTerminalHandle> {
    desktop_session_host().and_then(|host| {
        host.runtime_entry_terminal_handle_in_direction_by_session_id(session_id, direction)
    })
}

pub(crate) fn set_host_runtime_entry_zoomed_by_session_id(
    session_id: &str,
    zoomed: bool,
) -> Option<bool> {
    desktop_session_host()
        .and_then(|host| host.set_runtime_entry_zoomed_by_session_id(session_id, zoomed))
}

pub(crate) fn host_runtime_entry_terminal_infos_by_session_id(
    session_id: &str,
) -> Vec<RuntimeEntryTerminalInfo> {
    desktop_session_host()
        .map(|host| host.runtime_entry_terminal_infos_by_session_id(session_id))
        .unwrap_or_default()
}

pub(crate) fn rotate_host_runtime_entry_counter_clockwise_by_session_id(session_id: &str) -> bool {
    desktop_session_host()
        .map(|host| host.rotate_runtime_entry_counter_clockwise_by_session_id(session_id))
        .unwrap_or(false)
}

pub(crate) fn rotate_host_runtime_entry_clockwise_by_session_id(session_id: &str) -> bool {
    desktop_session_host()
        .map(|host| host.rotate_runtime_entry_clockwise_by_session_id(session_id))
        .unwrap_or(false)
}

pub(crate) fn resolve_host_runtime_id_for_terminal_handle(
    terminal_handle: SessionTerminalHandle,
) -> Option<RuntimeId> {
    desktop_session_host()
        .and_then(|host| host.resolve_runtime_id_for_terminal_handle(terminal_handle))
}

pub(crate) fn focus_host_root_runtime_entry(runtime_id: RuntimeId) -> bool {
    desktop_session_host()
        .map(|host| host.focus_root_runtime_entry(runtime_id))
        .unwrap_or(false)
}

pub(crate) fn set_host_runtime_entry_active_terminal(
    runtime_id: RuntimeId,
    terminal_handle: SessionTerminalHandle,
) -> bool {
    desktop_session_host()
        .map(|host| host.set_runtime_entry_active_terminal(runtime_id, terminal_handle))
        .unwrap_or(false)
}

fn primary_host_window_engine_id() -> HostWindowId {
    *PRIMARY_HOST_WINDOW_ID
        .get()
        .expect("primary host window to be initialized")
}

pub(crate) fn host_overlay_pane_layouts_by_id(
    render_scope_id: u64,
) -> Vec<overlay_shell::OverlayPaneLayout> {
    desktop_session_host()
        .map(|host| host.overlay_pane_layouts_by_id(render_scope_id))
        .unwrap_or_default()
}

pub(crate) fn host_resize_render_scope(render_scope_id: u64, size: TerminalSize) -> bool {
    desktop_session_host()
        .map(|host| host.resize_render_scope(render_scope_id, size))
        .unwrap_or(false)
}

pub(crate) fn host_resize_render_scope_split(
    render_scope_id: u64,
    split_index: usize,
    delta: isize,
) -> Option<overlay_shell::OverlaySplitLayout> {
    desktop_session_host()
        .and_then(|host| host.resize_render_scope_split(render_scope_id, split_index, delta))
}

pub(crate) fn terminal_handle_arc(
    terminal_handle: SessionTerminalHandle,
) -> Option<Arc<dyn HostTerminal>> {
    desktop_session_host()?.pane_for_terminal_handle(terminal_handle)
}

pub(crate) fn terminal_handle_arc_by_public_id(pane_id: u64) -> Option<Arc<dyn HostTerminal>> {
    desktop_session_host()?.pane_for_public_id(pane_id)
}

pub(crate) fn remove_terminal_handle(terminal_handle: SessionTerminalHandle) {
    if let Some(host) = desktop_session_host() {
        host.remove_terminal_handle(terminal_handle);
    }
}

pub(crate) fn remove_runtime_entry_scope(render_scope_id: u64) {
    if desktop_close_view_or_session_for_render_target(
        chatminal_runtime::SessionRenderTargetId::new(render_scope_id),
    ) {
        return;
    }
    if let Some(host) = desktop_session_host() {
        host.remove_runtime_entry_scope(render_scope_id);
    }
}

pub(crate) fn host_window_exists() -> bool {
    desktop_session_host()
        .map(|host| host.window_exists())
        .unwrap_or(false)
}

pub(crate) fn host_window_contains_render_scope(render_scope_id: u64) -> bool {
    desktop_session_host().is_some_and(|host| {
        host.render_state_for_runtime(RuntimeId::new(render_scope_id))
            .is_some()
            || host.host_window_contains_render_scope(render_scope_id)
    })
}

pub(crate) fn host_workspace_name() -> String {
    desktop_session_host()
        .map(|host| host.host_workspace_name())
        .unwrap_or_default()
}

pub(crate) fn configured_default_workspace_name(config: &ConfigHandle) -> String {
    config
        .default_workspace
        .as_deref()
        .unwrap_or("default")
        .to_string()
}

pub(crate) fn apply_host_runtime_config(config: &ConfigHandle) {
    if let Some(host) = desktop_session_host() {
        host.set_config(config);
    }
}

pub(crate) fn host_workspace_has_windows(name: &str) -> bool {
    desktop_session_host()
        .map(|host| host.workspace_has_windows(name))
        .unwrap_or(false)
}

pub(crate) fn record_host_focus_for_current_identity(terminal_handle: SessionTerminalHandle) {
    if let Some(host) = desktop_session_host() {
        host.record_focus_for_current_identity(terminal_handle);
    }
}

pub(crate) fn record_host_input_for_current_identity() {
    if let Some(host) = desktop_session_host() {
        host.record_input_for_current_identity();
    }
}

pub(crate) fn host_window_initial_position() -> Option<config::GuiPosition> {
    desktop_session_host()?.host_window_initial_position()
}

pub(crate) fn subscribe_runtime_notifications<F>(subscriber: F)
where
    F: Fn(RuntimeNotification) -> bool + 'static + Send + Sync,
{
    if let Some(host) = desktop_session_host() {
        host.subscribe_notifications(subscriber);
        return;
    }
    subscribe_desktop_runtime_notifications(subscriber);
}

pub(crate) fn resolved_window_title() -> Option<String> {
    desktop_session_host()?.resolved_window_title()
}

pub(crate) fn resolve_public_pane(
    host_terminal_handle: u64,
    terminal_instance_id: u64,
) -> Option<Arc<dyn HostTerminal>> {
    desktop_session_host()?.resolve_public_pane_fallback(host_terminal_handle, terminal_instance_id)
}

pub(crate) fn launcher_sessions() -> Vec<LauncherSessionEntry> {
    if let Some(host) = desktop_session_host() {
        return host.launcher_sessions();
    }
    Vec::new()
}

pub(crate) fn active_frontend_client() -> Option<FrontendClientHandle> {
    desktop_session_host().and_then(|host| host.active_frontend_client())
}

pub(crate) fn subscribe_frontend_notifications<F>(subscriber: F)
where
    F: Fn(RuntimeNotification) -> bool + 'static + Send + Sync,
{
    if let Some(host) = desktop_session_host() {
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
    desktop_session_host()
        .map(|host| host.active_workspace_for_client(client_id))
        .unwrap_or_default()
}

pub(crate) fn set_active_workspace_for_client(client_id: &FrontendClientHandle, workspace: &str) {
    if let Some(host) = desktop_session_host() {
        host.set_active_workspace_for_client(client_id, workspace);
    }
}

pub(crate) fn workspace_is_empty(workspace: &str) -> bool {
    desktop_session_host()
        .map(|host| host.workspace_is_empty(workspace))
        .unwrap_or(true)
}

pub(crate) fn workspace_names() -> Vec<String> {
    desktop_session_host()
        .map(|host| host.workspace_names())
        .unwrap_or_default()
}

pub(crate) fn focus_terminal_handle_by_id(pane_id: SessionTerminalHandle) -> anyhow::Result<()> {
    if desktop_focus_session_terminal_handle(pane_id).is_some() {
        return Ok(());
    }
    Err(anyhow!(
        "terminal handle {} not found in desktop session host",
        pane_id.as_u64()
    ))
}

pub(crate) fn frontend_resolve_pane(
    pane_id: SessionTerminalHandle,
) -> Option<FrontendResolvedPane> {
    desktop_session_host()?.frontend_resolve_pane_fallback(pane_id)
}

pub(crate) fn frontend_resolve_focused_pane(
    client_id: &FrontendClientHandle,
) -> Option<FrontendFocusedPane> {
    desktop_session_host()?.frontend_resolve_focused_pane_fallback(client_id)
}

pub(crate) async fn spawn_local_shell_runner() -> anyhow::Result<Arc<dyn HostTerminal>> {
    let host = desktop_session_host()
        .ok_or_else(|| anyhow!("desktop session host missing for local shell runner"))?;
    host.spawn_local_shell_runner().await
}

pub(crate) async fn spawn_desktop_terminal(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    current_terminal_handle: Option<SessionTerminalHandle>,
    workspace: String,
) -> anyhow::Result<Arc<dyn HostTerminal>> {
    let host = desktop_session_host()
        .ok_or_else(|| anyhow!("desktop session host missing for desktop terminal spawn"))?;
    host.spawn_desktop_terminal(
        command,
        command_dir,
        size,
        current_terminal_handle,
        workspace,
    )
    .await
}

pub(crate) fn set_host_spawn_target(spawn_target: &HostSpawnTargetHandle) {
    if let Some(host) = desktop_session_host() {
        host.set_primary_spawn_target(spawn_target);
    }
}

pub(crate) fn primary_host_spawn_target() -> HostSpawnTargetHandle {
    desktop_session_host()
        .expect("desktop session host to expose primary spawn target")
        .primary_spawn_target()
}

#[cfg(test)]
pub(crate) fn acquire_host_runtime_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static HOST_RUNTIME_TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    HOST_RUNTIME_TEST_GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn build_initial_host_runtime(
    config: &ConfigHandle,
    default_workspace_name: Option<&str>,
) -> anyhow::Result<()> {
    desktop_session_host()
        .expect("desktop session host to initialize")
        .build_initial_host_runtime(config, default_workspace_name)
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
    if let Some(host) = desktop_session_host() {
        return host.create_serial_spawn_target(serial_target);
    }
    Ok(HostSpawnTargetHandle::new(Arc::new(
        spawn_target::DesktopSpawnTarget::new_serial(serial_target)?,
    )))
}

pub(crate) fn install_serial_spawn_target(
    serial_target: config::SerialTarget,
) -> anyhow::Result<()> {
    let spawn_target = create_serial_spawn_target(serial_target)?;
    set_host_spawn_target(&spawn_target);
    Ok(())
}

pub(crate) async fn spawn_in_primary_target(
    size: TerminalSize,
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
) -> anyhow::Result<HostSpawnedRuntimeEntry> {
    primary_host_spawn_target()
        .spawn(size, command, command_dir)
        .await
}

pub(crate) fn shutdown_host_runtime() {
    if let Some(host) = desktop_session_host() {
        host.shutdown_host_runtime();
    }
}

pub(crate) fn activate_host_runtime_entry(render_scope_id: u64) -> anyhow::Result<()> {
    if let Some(session_id) = desktop_session_id_for_render_target(
        chatminal_runtime::SessionRenderTargetId::new(render_scope_id),
    ) {
        let size = desktop_render_state_for_session(&session_id)
            .map(|state| state.terminal_size)
            .ok_or_else(|| {
                anyhow!("missing render state for session render target {render_scope_id}")
            })?;
        desktop_activate_session(&session_id, None, size)
            .ok_or_else(|| anyhow!("failed to activate session render target {render_scope_id}"))?;
        return Ok(());
    }

    let host = desktop_session_host()
        .ok_or_else(|| anyhow!("desktop session host missing for render scope activation"))?;
    host.activate_runtime_entry(render_scope_id)
}

pub(crate) fn host_has_panes_in_workspace(workspace: Option<&str>) -> bool {
    desktop_session_host()
        .map(|host| host.has_panes_in_workspace(workspace))
        .unwrap_or(false)
}
