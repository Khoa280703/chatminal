#![allow(clippy::range_plus_one)]
use super::renderstate::*;
use super::utilsprites::RenderMetrics;
use crate::chatminal_layout::workspace_store::{
    DesktopWorkspaceLayoutStore, DEFAULT_LAYOUT_WORKSPACE_ID,
};
use crate::chatminal_runtime::overlay_compat::{
    OverlayAssignmentResult as PerformAssignmentResult, OverlayCachePolicy as CachePolicy,
    OverlayCloseReason as CloseReason, OverlayPane, OverlayPattern, OverlayTerminal,
    RenderableDimensions,
};
use crate::chatminal_runtime::{
    DesktopSessionBridgeAction, PrimaryHostWindowId, RuntimeId, RuntimeNotification,
    RuntimeWindow, SessionViewId, TerminalInstanceId,
};
use crate::chatminal_sidebar::{ChatminalSidebar, SidebarSessionDropTarget};
use crate::colorease::ColorEase;
use crate::desktop_overlay_actions::show_close_runtime_entry_overlay;
use crate::desktop_termwindow_types::{TerminalPaneLayout, TerminalSplit, TerminalUiKey};
use crate::frontend::{front_end, try_front_end};
use crate::inputmap::InputMap;
use crate::overlay::{
    confirm_quit_program, launcher, start_overlay, CopyModeParams, CopyOverlay, LauncherArgs,
    LauncherFlags, QuickSelectOverlay,
};
use crate::resize_increment_calculator::ResizeIncrementCalculator;
use crate::scripting::guiwin::PrimaryGuiWindowId;
use crate::scripting::guiwin::GuiWin;
use crate::scrollbar::*;
use crate::selection::{Selection, SelectionMode};
use crate::shapecache::*;
use crate::tabbar::{SessionBarItem, SessionBarState};
use crate::termwindow::background::{
    load_background_image, reload_background_image, LoadedBackgroundLayer,
};
use crate::termwindow::keyevent::{KeyTableArgs, KeyTableState};
use crate::termwindow::modal::Modal;
use crate::termwindow::startup_recipe_modal::StartupRecipeModal;
use crate::termwindow::render::paint::AllowImage;
use crate::termwindow::render::{
    CachedLineState, LineQuadCacheKey, LineQuadCacheValue, LineToEleShapeCacheKey,
    LineToElementShapeItem,
};
use crate::termwindow::webgpu::WebGpuState;
use ::engine_term::input::{ClickPosition, MouseButton as TMB};
use ::window::*;
use anyhow::{anyhow, ensure, Context};
use chatminal_runtime::RuntimeWorkspace;
use config::keyassignment::{
    Confirmation, KeyAssignment, LauncherActionArgs, Pattern, PromptInputLine,
    QuickSelectArguments, RotationDirection, SessionDirection, SpawnCommand, SplitSize,
};
use config::window::WindowLevel;
use config::{
    configuration, AudibleBell, ConfigHandle, Dimension, DimensionContext, FrontEndSelection,
    GeometryOrigin, GuiPosition, TermConfig, WindowCloseConfirmation,
};
use engine_dynamic::Value;
use engine_font::FontConfiguration;
use engine_term::color::ColorPalette;
use engine_term::input::LastMouseClick;
use engine_term::{Alert, Progress, StableRowIndex, TerminalConfiguration, TerminalSize};
use lfucache::*;
use mlua::{FromLua, LuaSerdeExt, UserData, UserDataFields};
use smol::channel::Sender;
use smol::Timer;
use std::cell::{RefCell, RefMut};
use std::collections::{BTreeSet, HashMap, LinkedList};
use std::convert::TryFrom;
use std::ops::Add;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use termwiz::hyperlink::Hyperlink;
use termwiz::surface::{Line, SequenceNo};
use termwiz_funcs::lines_to_escapes;

pub mod background;
pub mod box_model;
pub mod clipboard;
pub mod keyevent;
mod layout_render;
pub mod modal;
mod mouseevent;
pub mod palette;
pub mod paneselect;
mod prevcursor;
pub mod render;
pub mod resize;
mod selection;
mod startup_recipe_modal;
pub mod spawn;
pub mod webgpu;
use crate::spawn::SpawnWhere;
use prevcursor::PrevCursorPos;

const ATLAS_SIZE: usize = 128;

lazy_static::lazy_static! {
    static ref WINDOW_CLASS: Mutex<String> = Mutex::new(engine_gui_subcommands::DEFAULT_WINDOW_CLASS.to_owned());
    static ref POSITION: Mutex<Option<GuiPosition>> = Mutex::new(None);
}

pub const ICON_DATA: &'static [u8] = include_bytes!("../../assets/icon/terminal.png");

pub fn set_window_position(pos: GuiPosition) {
    POSITION.lock().unwrap().replace(pos);
}

pub fn set_window_class(cls: &str) {
    *WINDOW_CLASS.lock().unwrap() = cls.to_owned();
}

pub fn get_window_class() -> String {
    WINDOW_CLASS.lock().unwrap().clone()
}

fn pane_metadata_u64(pane: &dyn OverlayPane, key: &str) -> Option<u64> {
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

fn pane_metadata_terminal_instance_id(pane: &dyn OverlayPane) -> Option<TerminalInstanceId> {
    pane_metadata_u64(pane, "chatminal_terminal_instance_id").map(TerminalInstanceId::new)
}

fn pane_matches_public_id(pane: &dyn OverlayPane, public_id: u64) -> bool {
    pane.pane_id() as u64 == public_id
        || pane_metadata_terminal_instance_id(pane)
            .map(|terminal_instance_id| terminal_instance_id.as_u64() == public_id)
            .unwrap_or(false)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MouseCapture {
    UI,
    TerminalPane(TerminalUiKey),
}

/// Type used together with Window::notify to do something in the
/// context of the window-specific event loop
pub enum TermWindowNotif {
    InvalidateShapeCache,
    PerformAssignmentForTerminalHandle {
        terminal_handle: u64,
        assignment: KeyAssignment,
        tx: Option<Sender<anyhow::Result<()>>>,
    },
    SetLeftStatus(String),
    SetRightStatus(String),
    GetDimensions(Sender<(Dimensions, WindowState)>),
    GetEffectiveConfig(Sender<ConfigHandle>),
    FinishWindowEvent {
        name: String,
        again: bool,
    },
    GetConfigOverrides(Sender<engine_dynamic::Value>),
    SetConfigOverrides(engine_dynamic::Value),
    CancelOverlayForTerminalHandle(u64),
    CancelOverlayForRenderScope {
        render_target_id: u64,
        pane_id: Option<u64>,
    },
    RuntimeNotification(RuntimeNotification),
    EmitStatusUpdate,
    Apply(Box<dyn FnOnce(&mut TermWindow) + Send + Sync>),
    SetInnerSize {
        width: usize,
        height: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UIItemType {
    SessionBar(SessionBarItem),
    CloseSessionEntry(usize),
    CloseSession(String),
    AboveScrollThumb(u64),
    ScrollThumb(u64),
    BelowScrollThumb(u64),
    Split(TerminalSplit),
    ChatminalSidebarBackground,
    ChatminalSidebarResizeHandle,
    ChatminalSidebarSettings,
    ChatminalSidebarCreateProfile,
    ChatminalSidebarProfile(String),
    ChatminalSidebarCreateSession,
    ChatminalSidebarSession(String),
    ChatminalSidebarSessionMenu,
    ChatminalSidebarSessionMenuJoin(String),
    ChatminalSidebarSessionMenuUnjoin(String),
    ChatminalSidebarSessionMenuRename(String),
    ChatminalSidebarSessionMenuStartupCommand(String),
    ChatminalSidebarSessionMenuRunStartupCommand(String),
    ChatminalSidebarSessionMenuDelete(String),
    ChatminalStartupRecipeModalBackdrop,
    ChatminalStartupRecipeModalPanel,
    ChatminalStartupRecipeModalInput,
    ChatminalStartupRecipeModalCancel,
    ChatminalStartupRecipeModalRun,
    ChatminalStartupRecipeModalSave,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UIItem {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub item_type: UIItemType,
}

impl UIItem {
    pub fn hit_test(&self, x: isize, y: isize) -> bool {
        x >= self.x as isize
            && x <= (self.x + self.width) as isize
            && y >= self.y as isize
            && y <= (self.y + self.height) as isize
    }
}

#[derive(Clone, Default)]
pub struct SemanticZoneCache {
    seqno: SequenceNo,
    zones: Vec<StableRowIndex>,
}

pub struct OverlayState {
    pub pane: Arc<dyn OverlayPane>,
    pub key_table_state: KeyTableState,
}

#[derive(Default)]
pub struct TerminalUiState {
    /// If is_some(), the top row of the visible screen.
    /// Otherwise, the viewport is at the bottom of the
    /// scrollback.
    viewport: Option<StableRowIndex>,
    selection: Selection,
    /// If is_some(), rather than display the actual tab
    /// contents, we're overlaying a little internal application
    /// tab.  We'll also route input to it.
    pub overlay: Option<OverlayState>,

    bell_start: Option<Instant>,
    pub mouse_terminal_coords: Option<(ClickPosition, StableRowIndex)>,
}

/// Data used when synchronously formatting pane and window titles
#[derive(Debug, Clone)]
pub struct SessionEntryInformation {
    pub render_target_id: u64,
    pub entry_index: usize,
    pub is_active: bool,
    pub is_last_active: bool,
    pub active_terminal_instance: Option<TerminalInstanceInformation>,
    pub terminal_instances: Vec<TerminalInstanceInformation>,
    pub entry_title: String,
    pub session_id: Option<String>,
    pub view_id: Option<SessionViewId>,
}

impl SessionEntryInformation {
    fn resolved_window_title(&self) -> mlua::Result<String> {
        crate::chatminal_runtime::resolved_window_title()
            .ok_or_else(|| mlua::Error::external("primary window not found"))
    }
}

impl UserData for SessionEntryInformation {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("render_target_id", |_, this| Ok(this.render_target_id));
        fields.add_field_method_get("render_scope_id", |_, this| Ok(this.render_target_id));
        fields.add_field_method_get("entry_index", |_, this| Ok(this.entry_index));
        fields.add_field_method_get("is_active", |_, this| Ok(this.is_active));
        fields.add_field_method_get("is_last_active", |_, this| Ok(this.is_last_active));
        fields.add_field_method_get("active_terminal_instance", |_, this| {
            Ok(this.active_terminal_instance.clone())
        });
        fields.add_field_method_get("terminal_instances", |_, this| {
            Ok(this.terminal_instances.clone())
        });
        fields.add_field_method_get("entry_title", |_, this| Ok(this.entry_title.clone()));
        fields.add_field_method_get("session_id", |_, this| Ok(this.session_id.clone()));
        fields.add_field_method_get("view_id", |_, this| {
            Ok(this.view_id.map(|value| value.as_u64()))
        });
        fields.add_field_method_get("window_title", |_, this| this.resolved_window_title());
    }
}

/// Data used when synchronously formatting pane and window titles
#[derive(Debug, Clone)]
pub struct TerminalInstanceInformation {
    pub host_terminal_handle: u64,
    pub terminal_instance_id: u64,
    pub terminal_index: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
    pub has_unseen_output: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
    pub title: String,
    pub user_vars: HashMap<String, String>,
    pub progress: Progress,
}

impl TerminalInstanceInformation {
    fn resolved_pane(&self) -> Option<Arc<dyn OverlayPane>> {
        crate::chatminal_runtime::resolve_public_pane(
            self.host_terminal_handle,
            self.terminal_instance_id,
        )
    }
}

impl UserData for TerminalInstanceInformation {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("host_terminal_handle", |_, this| {
            Ok(this.host_terminal_handle)
        });
        fields.add_field_method_get("terminal_instance_id", |_, this| {
            Ok(this.terminal_instance_id)
        });
        fields.add_field_method_get("terminal_index", |_, this| Ok(this.terminal_index));
        fields.add_field_method_get("is_active", |_, this| Ok(this.is_active));
        fields.add_field_method_get("is_zoomed", |_, this| Ok(this.is_zoomed));
        fields.add_field_method_get("has_unseen_output", |_, this| Ok(this.has_unseen_output));
        fields.add_field_method_get("left", |_, this| Ok(this.left));
        fields.add_field_method_get("top", |_, this| Ok(this.top));
        fields.add_field_method_get("width", |_, this| Ok(this.width));
        fields.add_field_method_get("height", |_, this| Ok(this.height));
        fields.add_field_method_get("pixel_width", |_, this| Ok(this.pixel_width));
        fields.add_field_method_get("pixel_height", |_, this| Ok(this.pixel_height));
        fields.add_field_method_get("progress", |lua, this| lua.to_value(&this.progress));
        fields.add_field_method_get("title", |_, this| Ok(this.title.clone()));
        fields.add_field_method_get("user_vars", |_, this| Ok(this.user_vars.clone()));
        fields.add_field_method_get("foreground_process_name", |_, this| {
            Ok(this
                .resolved_pane()
                .and_then(|pane| pane.get_foreground_process_name(CachePolicy::AllowStale))
                .unwrap_or_default())
        });
        fields.add_field_method_get("tty_name", |_, this| {
            Ok(this.resolved_pane().and_then(|pane| pane.tty_name()))
        });
        fields.add_field_method_get("current_working_dir", |_, this| {
            Ok(this
                .resolved_pane()
                .and_then(|pane| pane.get_current_working_dir(CachePolicy::AllowStale))
                .map(|url| url_funcs::Url { url }))
        });
    }
}

#[derive(Default)]
pub struct RuntimeUiState {
    /// If is_some(), rather than display the actual tab
    /// contents, we're overlaying a little internal application
    /// tab.  We'll also route input to it.
    pub overlay: Option<OverlayState>,
}

/// Manages the state/queue of lua based event handlers.
/// We don't want to queue more than 1 event at a time,
/// so we use this enum to allow for at most 1 executing
/// and 1 pending event.
#[derive(Copy, Clone, Debug)]
enum EventState {
    /// The event is not running
    None,
    /// The event is running
    InProgress,
    /// The event is running, and we have another one ready to
    /// run once it completes
    InProgressWithQueued(Option<TerminalUiKey>),
}

pub struct TermWindow {
    pub window: Option<Window>,
    pub config: ConfigHandle,
    pub config_overrides: engine_dynamic::Value,
    os_parameters: Option<parameters::Parameters>,
    /// When we most recently received keyboard focus
    pub focused: Option<Instant>,
    fonts: Rc<FontConfiguration>,
    /// Window dimensions and dpi
    pub dimensions: Dimensions,
    pub window_state: WindowState,
    pub resizes_pending: usize,
    is_repaint_pending: bool,
    pending_scale_changes: LinkedList<resize::ScaleChange>,
    /// Terminal dimensions
    terminal_size: TerminalSize,
    pub primary_host_window_id: PrimaryHostWindowId,
    pub render_metrics: RenderMetrics,
    render_state: Option<RenderState>,
    input_map: InputMap,
    /// If is_some, the LEADER modifier is active until the specified instant.
    leader_is_down: Option<std::time::Instant>,
    dead_key_status: DeadKeyStatus,
    key_table_state: KeyTableState,
    show_session_bar: bool,
    show_terminal_footer: bool,
    show_scroll_bar: bool,
    tab_bar: SessionBarState,
    fancy_tab_bar: Option<box_model::ComputedElement>,
    pub right_status: String,
    pub left_status: String,
    last_ui_item: Option<UIItem>,
    /// Tracks whether the current mouse-down event is part of click-focus.
    /// If so, we ignore mouse events until released
    is_click_to_focus_window: bool,
    last_mouse_coords: (usize, i64),
    window_drag_position: Option<MouseEvent>,
    current_mouse_event: Option<MouseEvent>,
    prev_cursor: PrevCursorPos,
    last_scroll_info: RenderableDimensions,

    runtime_ui_state: RefCell<HashMap<u64, RuntimeUiState>>,
    terminal_ui_state_by_handle: RefCell<HashMap<TerminalUiKey, TerminalUiState>>,
    semantic_zones: HashMap<TerminalUiKey, SemanticZoneCache>,

    window_background: Vec<LoadedBackgroundLayer>,

    current_modifier_and_leds: (Modifiers, KeyboardLedStatus),
    current_mouse_buttons: Vec<MousePress>,
    current_mouse_capture: Option<MouseCapture>,

    opengl_info: Option<String>,

    /// Keeps track of double and triple clicks
    last_mouse_click: Option<LastMouseClick>,

    /// The URL over which we are currently hovering
    current_highlight: Option<Arc<Hyperlink>>,

    quad_generation: usize,
    shape_generation: usize,
    shape_cache: RefCell<LfuCache<ShapeCacheKey, anyhow::Result<Rc<Vec<ShapedInfo>>>>>,
    line_to_ele_shape_cache: RefCell<LfuCache<LineToEleShapeCacheKey, LineToElementShapeItem>>,

    line_state_cache: RefCell<LfuCacheU64<Arc<CachedLineState>>>,
    next_line_state_id: u64,

    line_quad_cache: RefCell<LfuCache<LineQuadCacheKey, LineQuadCacheValue>>,

    last_status_call: Instant,
    cursor_blink_state: RefCell<ColorEase>,
    blink_state: RefCell<ColorEase>,
    rapid_blink_state: RefCell<ColorEase>,

    palette: Option<ColorPalette>,

    ui_items: Vec<UIItem>,
    dragging: Option<(UIItem, MouseEvent)>,

    modal: RefCell<Option<Rc<dyn Modal>>>,

    event_states: HashMap<String, EventState>,
    pub current_event: Option<Value>,
    has_animation: RefCell<Option<Instant>>,
    /// We use this to attempt to do something reasonable
    /// if we run out of texture space
    allow_images: AllowImage,
    scheduled_animation: RefCell<Option<Instant>>,

    created: Instant,

    pub last_frame_duration: Duration,
    last_fps_check_time: Instant,
    num_frames: usize,
    pub fps: f32,

    connection_name: String,

    gl: Option<Rc<glium::backend::Context>>,
    webgpu: Option<Rc<WebGpuState>>,
    config_subscription: Option<config::ConfigSubscription>,
    chatminal_sidebar: ChatminalSidebar,
    chatminal_sidebar_seen_version: u64,
    chatminal_sidebar_poll_started: bool,
    sidebar_tree_cache: Option<render::chatminal_sidebar::SidebarTreeCache>,
    sidebar_header_cache: Option<render::chatminal_sidebar::SidebarHeaderCache>,
    sidebar_footer_background_cache:
        Option<render::chatminal_sidebar::SidebarFooterBackgroundCache>,
    system_metrics: crate::system_metrics::SystemMetricsHandle,
    metrics_tick_started: bool,
}

impl TermWindow {
    fn chatminal_sidebar_width_for_dimensions(pixel_width: usize, dpi: usize) -> usize {
        ChatminalSidebar::width_pixels(pixel_width, dpi)
    }

    pub(crate) fn chatminal_sidebar_width(&self) -> usize {
        self.chatminal_sidebar
            .width_pixels_for_window(self.dimensions.pixel_width, self.dimensions.dpi)
    }

    fn chatminal_shell_enabled_for_dimensions(pixel_width: usize, dpi: usize) -> bool {
        Self::chatminal_sidebar_width_for_dimensions(pixel_width, dpi) > 0
    }

    fn chatminal_terminal_chrome_height_for_dimensions(_pixel_width: usize, _dpi: usize) -> usize {
        0
    }

    fn chatminal_terminal_footer_height_for_dimensions(
        pixel_width: usize,
        dpi: usize,
        show_terminal_footer: bool,
    ) -> usize {
        if show_terminal_footer && Self::chatminal_shell_enabled_for_dimensions(pixel_width, dpi) {
            52
        } else {
            0
        }
    }

    pub(crate) fn chatminal_terminal_chrome_height(&self) -> f32 {
        Self::chatminal_terminal_chrome_height_for_dimensions(
            self.dimensions.pixel_width,
            self.dimensions.dpi,
        ) as f32
    }

    pub(crate) fn terminal_tab_bar_left(&self) -> f32 {
        self.chatminal_sidebar_width() as f32
    }

    pub(crate) fn terminal_tab_bar_width(&self) -> f32 {
        (self.dimensions.pixel_width as f32 - self.terminal_tab_bar_left()).max(0.0)
    }

    pub(crate) fn terminal_tab_bar_cols(&self) -> usize {
        ((self.terminal_tab_bar_width() / self.render_metrics.cell_size.width as f32).floor()
            as usize)
            .max(1)
    }

    pub(crate) fn chatminal_terminal_footer_height(&self) -> f32 {
        Self::chatminal_terminal_footer_height_for_dimensions(
            self.dimensions.pixel_width,
            self.dimensions.dpi,
            self.show_terminal_footer,
        ) as f32
    }

    /// Compute shell chrome geometry for the current frame.
    ///
    /// Both render and hit-test paths should use this as the single source
    /// of truth for sidebar/session-bar/footer/content bounds.
    pub(crate) fn shell_bounds(&self) -> crate::shell_bounds::ShellBounds {
        let border = self.get_os_border();
        let bt = border.top.get() as f32;
        let bb = border.bottom.get() as f32;
        let bl = border.left.get() as f32;
        let br = border.right.get() as f32;
        let ww = self.dimensions.pixel_width as f32;
        let wh = self.dimensions.pixel_height as f32;
        let shell_enabled = self.chatminal_sidebar_width() > 0;
        let sidebar_w = if shell_enabled {
            self.chatminal_sidebar_width() as f32
        } else {
            0.0
        };
        let footer_h = self.chatminal_terminal_footer_height();
        let session_bar_h = if self.show_session_bar {
            self.tab_bar_pixel_height().unwrap_or(0.0)
        } else {
            0.0
        };
        let session_bar_at_bottom = self.config.session_bar_at_bottom;

        // Sidebar: left edge, full height inside border
        let sidebar_x = 0.0;
        let sidebar_y = bt;
        let sidebar_height = (wh - bt - bb).max(0.0);

        // Session bar: right of sidebar
        let session_bar_x = sidebar_w;
        let session_bar_width = (ww - sidebar_w).max(0.0);
        let session_bar_y = if session_bar_at_bottom {
            (wh - bb - footer_h - session_bar_h).max(bt)
        } else {
            bt
        };

        // Footer: below content, right of sidebar
        let footer_x = sidebar_w;
        let footer_width = (ww - sidebar_w).max(0.0);
        let footer_y = (wh - bb - footer_h).max(0.0);

        // Content: between session bar and footer, right of sidebar
        let content_x = sidebar_w;
        let content_y = if session_bar_at_bottom {
            bt
        } else {
            bt + session_bar_h
        };
        let content_bottom = if session_bar_at_bottom {
            (wh - bb - footer_h - session_bar_h).max(content_y)
        } else {
            (wh - bb - footer_h).max(content_y)
        };
        let content_width = (ww - sidebar_w).max(0.0);
        let content_height = (content_bottom - content_y).max(0.0);

        crate::shell_bounds::ShellBounds {
            window_width: ww,
            window_height: wh,
            border_top: bt,
            border_bottom: bb,
            border_left: bl,
            border_right: br,
            sidebar_x,
            sidebar_y,
            sidebar_width: sidebar_w,
            sidebar_height,
            session_bar_x,
            session_bar_y,
            session_bar_width,
            session_bar_height: session_bar_h,
            footer_x,
            footer_y,
            footer_width,
            footer_height: footer_h,
            content_x,
            content_y,
            content_width,
            content_height,
            shell_enabled,
            session_bar_at_bottom,
        }
    }

    pub(crate) fn terminal_grid_origin(&self) -> ::window::PointF {
        let bounds = self.shell_bounds();
        let (padding_left, padding_top) = self.padding_left_top();
        euclid::point2(
            bounds.border_left + padding_left,
            bounds.content_y + padding_top,
        )
    }

    fn should_show_session_bar_for_count(
        config: &ConfigHandle,
        num_tabs: usize,
        sidebar_enabled: bool,
    ) -> bool {
        if sidebar_enabled {
            return false;
        }
        if num_tabs <= 1 {
            config.enable_session_bar && !config.hide_session_bar_if_only_one_session
        } else {
            config.enable_session_bar
        }
    }

    fn load_os_parameters(&mut self) {
        if let Some(ref window) = self.window {
            self.os_parameters = match window.get_os_parameters(&self.config, self.window_state) {
                Ok(os_parameters) => os_parameters,
                Err(err) => {
                    log::warn!("Error while getting OS parameters: {:#}", err);
                    None
                }
            };
        }
    }

    fn initialize_chatminal_sidebar(&mut self) {
        if !self.chatminal_sidebar.is_enabled() || self.chatminal_sidebar_poll_started {
            return;
        }
        self.chatminal_sidebar.start_background_sync();
        self.chatminal_sidebar_seen_version = self.chatminal_sidebar.version();
        self.chatminal_sidebar_poll_started = true;
        self.ensure_chatminal_active_session_runtime();
        let _ = crate::chatminal_runtime::desktop_prepare_workspace_layout(self.terminal_size);
        let _ = crate::chatminal_runtime::desktop_resize_visible_sessions(self.terminal_size);
        self.schedule_chatminal_sidebar_tick();
    }

    fn schedule_chatminal_sidebar_tick(&self) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        let Some(window) = self.window.clone() else {
            return;
        };
        promise::spawn::spawn(async move {
            Timer::after(Duration::from_millis(250)).await;
            window.notify(TermWindowNotif::Apply(Box::new(|term_window| {
                term_window.handle_chatminal_sidebar_tick();
            })));
        })
        .detach();
    }

    fn handle_chatminal_sidebar_tick(&mut self) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        let version = self.chatminal_sidebar.version();
        if version != self.chatminal_sidebar_seen_version {
            self.chatminal_sidebar_seen_version = version;
            self.ensure_chatminal_active_session_runtime();
            let _ = crate::chatminal_runtime::desktop_prepare_workspace_layout(self.terminal_size);
            let _ = crate::chatminal_runtime::desktop_resize_visible_sessions(self.terminal_size);
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }
        self.schedule_chatminal_sidebar_tick();
    }

    fn ensure_chatminal_active_session_runtime(&mut self) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        if crate::chatminal_runtime::desktop_render_state_for_session(&session_id).is_some() {
            return;
        }
        let _ = self.activate_chatminal_session_target(&session_id, None);
    }

    fn initialize_metrics_tick(&mut self) {
        if self.metrics_tick_started {
            return;
        }
        self.metrics_tick_started = true;
        self.schedule_metrics_tick();
    }

    fn schedule_metrics_tick(&self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        promise::spawn::spawn(async move {
            Timer::after(Duration::from_millis(2000)).await;
            window.notify(TermWindowNotif::Apply(Box::new(|term_window| {
                term_window.handle_metrics_tick();
            })));
        })
        .detach();
    }

    fn handle_metrics_tick(&mut self) {
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
        self.schedule_metrics_tick();
    }

    fn switch_chatminal_session(&mut self, session_id: &str) {
        self.switch_chatminal_session_target(session_id, None);
    }

    fn ordered_selected_chatminal_session_ids(&self, anchor_session_id: &str) -> Vec<String> {
        let selected_ids = self.chatminal_sidebar.selected_session_ids();
        if selected_ids.is_empty() {
            return Vec::new();
        }
        let snapshot = self.chatminal_sidebar.snapshot();
        let Some(anchor_profile_id) = snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == anchor_session_id)
            .map(|session| session.profile_id.as_str())
        else {
            return Vec::new();
        };
        if selected_ids.iter().any(|session_id| {
            snapshot
                .sessions
                .iter()
                .find(|session| session.session_id == *session_id)
                .map(|session| session.profile_id.as_str())
                != Some(anchor_profile_id)
        }) {
            return Vec::new();
        }
        let layout_store = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID);
        let mut expanded_selected_ids = selected_ids.clone();
        for session_id in &selected_ids {
            for joined_session_id in
                layout_store.profile_group_session_ids(anchor_profile_id, session_id)
            {
                if !expanded_selected_ids
                    .iter()
                    .any(|existing| existing == &joined_session_id)
                {
                    expanded_selected_ids.push(joined_session_id);
                }
            }
        }

        let selected: BTreeSet<_> = expanded_selected_ids.iter().map(String::as_str).collect();
        let mut ordered: Vec<String> = self
            .ordered_chatminal_session_ids()
            .into_iter()
            .filter(|session_id| selected.contains(session_id.as_str()))
            .collect();

        for session_id in expanded_selected_ids {
            if !ordered.iter().any(|existing| existing == &session_id) {
                ordered.push(session_id);
            }
        }

        if let Some(index) = ordered
            .iter()
            .position(|session_id| session_id == anchor_session_id)
        {
            ordered.swap(0, index);
        }

        ordered
    }

    fn move_chatminal_sessions_to_sidebar_target(
        &mut self,
        session_ids: &[String],
        target: &SidebarSessionDropTarget,
    ) -> bool {
        if session_ids.is_empty() {
            return false;
        }

        let snapshot = self.chatminal_sidebar.snapshot();
        let Some(source_profile_id) = snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == session_ids[0])
            .map(|session| session.profile_id.clone())
        else {
            return false;
        };
        if !session_ids.iter().all(|session_id| {
            snapshot
                .sessions
                .iter()
                .find(|session| session.session_id == *session_id)
                .map(|session| session.profile_id.as_str())
                == Some(source_profile_id.as_str())
        }) {
            return false;
        }

        let (target_profile_id, target_index) = match target {
            SidebarSessionDropTarget::ProfileAppend { profile_id } => {
                let target_index = snapshot
                    .sessions
                    .iter()
                    .filter(|session| session.profile_id == *profile_id)
                    .filter(|session| {
                        !session_ids
                            .iter()
                            .any(|dragged| dragged == &session.session_id)
                    })
                    .count();
                (profile_id.clone(), target_index)
            }
            SidebarSessionDropTarget::SessionInsertBefore {
                profile_id,
                session_id,
            } => {
                let target_index = snapshot
                    .sessions
                    .iter()
                    .filter(|session| session.profile_id == *profile_id)
                    .filter(|session| {
                        !session_ids
                            .iter()
                            .any(|dragged| dragged == &session.session_id)
                    })
                    .position(|session| session.session_id == *session_id)
                    .unwrap_or_else(|| {
                        snapshot
                            .sessions
                            .iter()
                            .filter(|session| session.profile_id == *profile_id)
                            .filter(|session| {
                                !session_ids
                                    .iter()
                                    .any(|dragged| dragged == &session.session_id)
                            })
                            .count()
                    });
                (profile_id.clone(), target_index)
            }
        };

        let moved = match self.chatminal_sidebar.move_sessions_to_profile(
            session_ids,
            &target_profile_id,
            Some(target_index),
        ) {
            Ok(workspace) => workspace,
            Err(err) => {
                log::error!("failed to move dragged sessions: {err}");
                return false;
            }
        };
        self.chatminal_sidebar.apply_workspace(moved);

        if source_profile_id != target_profile_id && session_ids.len() > 1 {
            let source_store = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID);
            source_store.clear_profile_group_layouts(
                &source_profile_id,
                session_ids.iter().map(String::as_str),
            );
            if let Some(layout) =
                crate::chatminal_runtime::WorkspaceLayoutState::grouped_sessions(session_ids)
            {
                for session_id in session_ids {
                    let _ = DesktopWorkspaceLayoutStore::new(
                        DesktopWorkspaceLayoutStore::profile_group_workspace_id(
                            &target_profile_id,
                            session_id,
                        ),
                    )
                    .replace_layout(layout.clone());
                }
            }
        }

        true
    }

    fn join_chatminal_selected_sessions(&mut self, anchor_session_id: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }

        let snapshot = self.chatminal_sidebar.snapshot();
        let ordered_session_ids = self.ordered_selected_chatminal_session_ids(anchor_session_id);
        if ordered_session_ids.len() < 2 {
            return;
        }

        let anchor_session_id = ordered_session_ids[0].clone();
        let Some(anchor_profile_id) = snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == anchor_session_id)
            .map(|session| session.profile_id.clone())
        else {
            return;
        };
        let is_active_profile =
            snapshot.active_profile_id.as_deref() == Some(anchor_profile_id.as_str());
        let workspace_id = if is_active_profile {
            DEFAULT_LAYOUT_WORKSPACE_ID.to_string()
        } else {
            DesktopWorkspaceLayoutStore::profile_workspace_id(&anchor_profile_id)
        };
        let layout_store = DesktopWorkspaceLayoutStore::new(workspace_id);
        if is_active_profile {
            let _ = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID)
                .save_as_profile_layout(&anchor_profile_id);
        }
        let Some(layout) =
            crate::chatminal_runtime::WorkspaceLayoutState::grouped_sessions(&ordered_session_ids)
        else {
            log::error!("failed to build joined session layout");
            return;
        };
        layout_store.replace_layout(layout);
        if is_active_profile {
            let _ = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID)
                .save_as_profile_layout(&anchor_profile_id);
            let _ = crate::chatminal_runtime::desktop_prepare_workspace_layout(self.terminal_size);
        } else {
            let _ = layout_store.save_as_profile_layout(&anchor_profile_id);
        }
        self.switch_chatminal_session_target(&anchor_session_id, None);
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    fn ordered_chatminal_session_ids(&self) -> Vec<String> {
        let snapshot = self.chatminal_sidebar.snapshot();
        let workspace_ids: Vec<String> = snapshot
            .sessions
            .iter()
            .map(|session| session.session_id.clone())
            .collect();
        if workspace_ids.is_empty() {
            return workspace_ids;
        }
        let Some(layout) = self.chatminal_workspace_layout_snapshot() else {
            return workspace_ids;
        };

        let mut ordered = Vec::new();
        for view in layout.views {
            if workspace_ids
                .iter()
                .any(|session_id| session_id == &view.session_id)
                && !ordered
                    .iter()
                    .any(|session_id| session_id == &view.session_id)
            {
                ordered.push(view.session_id);
            }
        }
        for session_id in workspace_ids {
            if !ordered.iter().any(|existing| existing == &session_id) {
                ordered.push(session_id);
            }
        }
        ordered
    }

    fn activate_chatminal_session_index(&mut self, session_idx: isize) -> anyhow::Result<()> {
        let ordered_session_ids = self.ordered_chatminal_session_ids();
        let max = ordered_session_ids.len();
        ensure!(max > 0, "no more sessions");

        let session_idx = if session_idx < 0 {
            max.saturating_sub(session_idx.unsigned_abs())
        } else {
            session_idx as usize
        };

        if let Some(session_id) = ordered_session_ids.get(session_idx) {
            self.switch_chatminal_session(session_id);
        }
        Ok(())
    }

    fn activate_chatminal_session_relative(
        &mut self,
        delta: isize,
        wrap: bool,
    ) -> anyhow::Result<()> {
        let ordered_session_ids = self.ordered_chatminal_session_ids();
        let max = ordered_session_ids.len();
        ensure!(max > 0, "no more sessions");

        let active = self
            .active_session_id()
            .as_deref()
            .and_then(|session_id| {
                ordered_session_ids
                    .iter()
                    .position(|candidate| candidate == session_id)
            })
            .unwrap_or(0) as isize;
        let session_idx = active + delta;
        let session_idx = if wrap {
            let wrapped = if session_idx < 0 {
                max as isize + session_idx
            } else {
                session_idx
            };
            (wrapped as usize % max) as isize
        } else if session_idx < 0 {
            0
        } else if session_idx >= max as isize {
            max as isize - 1
        } else {
            session_idx
        };

        self.activate_chatminal_session_index(session_idx)
    }

    fn activate_last_chatminal_session(&mut self) -> anyhow::Result<()> {
        let Some(session_id) = crate::chatminal_runtime::desktop_last_active_session_id() else {
            return Ok(());
        };
        self.switch_chatminal_session(&session_id);
        Ok(())
    }

    fn switch_chatminal_session_target(
        &mut self,
        session_id: &str,
        preferred_runtime_id: Option<RuntimeId>,
    ) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        let sidebar_snapshot = self.chatminal_sidebar.snapshot();
        let target_profile_id = sidebar_snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .map(|session| session.profile_id.clone());
        let layout_store = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID);
        if let Some(profile_id) = target_profile_id.as_deref() {
            if sidebar_snapshot.active_profile_id.as_deref() != Some(profile_id) {
                match self.chatminal_sidebar.switch_profile(profile_id) {
                    Ok(workspace) => {
                        let _ = layout_store.swap_profile_layout(
                            sidebar_snapshot.active_profile_id.as_deref(),
                            profile_id,
                            Some(session_id),
                        );
                        self.chatminal_sidebar.apply_workspace(workspace);
                    }
                    Err(err) => {
                        log::error!(
                            "failed to sync active profile {profile_id} for session {session_id}: {err}"
                        );
                        return;
                    }
                }
            } else if layout_store.view_id_for_session(session_id).is_none() && {
                let _ = layout_store.save_as_profile_layout(profile_id);
                layout_store
                    .restore_profile_layout_if_contains(profile_id, session_id)
                    .is_some()
            } {
                let _ =
                    crate::chatminal_runtime::desktop_prepare_workspace_layout(self.terminal_size);
            }
        }
        if self
            .activate_chatminal_session_target(session_id, preferred_runtime_id)
            .is_some()
        {
            self.chatminal_sidebar.set_active_session_local(session_id);
            self.chatminal_sidebar.select_single_session(session_id);
        }
    }

    fn activate_chatminal_session_target(
        &mut self,
        session_id: &str,
        preferred_runtime_id: Option<RuntimeId>,
    ) -> Option<crate::chatminal_runtime::DesktopSessionRuntimeSummary> {
        // Session-native path (Phase 04): route entirely through DesktopSessionHost,
        // without touching the legacy global runtime lookup on the active flow.
        let size = self.terminal_size;

        let runtime_state = crate::chatminal_runtime::desktop_activate_session(
            session_id,
            preferred_runtime_id,
            size,
        );

        if let Some(state) = runtime_state {
            if let Err(err) = crate::chatminal_runtime::notify_runtime_session_activated(
                session_id,
                state.runtime_id,
            ) {
                log::error!("failed to notify runtime bridge about session activation: {err}");
            }
            self.chatminal_sidebar.set_active_session_local(session_id);
            self.chatminal_sidebar
                .set_session_status_local(session_id, "running");
            self.chatminal_sidebar.select_single_session(session_id);
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
            return Some(state);
        }
        log::error!("failed to activate session-native target for session {session_id}");
        None
    }

    fn active_render_scope_id(&self) -> Option<u64> {
        if self.chatminal_sidebar.is_enabled() {
            return self
                .active_session_id()
                .as_deref()
                .and_then(crate::chatminal_runtime::desktop_render_state_for_session)
                .map(|state| state.render_target_id().as_u64());
        }
        crate::chatminal_runtime::host_active_render_scope_id()
    }

    pub(crate) fn active_session_id(&self) -> Option<String> {
        if !self.chatminal_sidebar.is_enabled() {
            return None;
        }
        if let Some(layout) = self.chatminal_workspace_layout_snapshot() {
            if let Some(view) = layout.view(layout.active_view_id) {
                return Some(view.session_id.clone());
            }
        }
        let snapshot = self.chatminal_sidebar.snapshot();
        if let Some(session_id) = snapshot.active_session_id.clone() {
            return Some(session_id);
        }
        if let Some(session_id) = snapshot
            .sessions
            .iter()
            .find(|session| session.is_active)
            .map(|session| session.session_id.clone())
        {
            return Some(session_id);
        }
        None
    }

    pub(crate) fn active_view_id(&self) -> Option<u64> {
        if !self.chatminal_sidebar.is_enabled() {
            return None;
        }
        self.chatminal_workspace_layout_snapshot()
            .map(|layout| layout.active_view_id.as_u64())
    }

    fn chatminal_workspace_layout_snapshot(
        &self,
    ) -> Option<crate::chatminal_runtime::WorkspaceLayoutState> {
        DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID).snapshot()
    }

    fn sync_active_chatminal_session_from_mux(&mut self) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        let lookup = match crate::chatminal_runtime::desktop_session_window_snapshot() {
            Ok(snapshot) => snapshot.lookup,
            Err(err) => {
                log::error!("failed to load desktop session window snapshot: {err}");
                return;
            }
        };
        let action = match crate::chatminal_runtime::reconcile_runtime_session_lookup(&lookup) {
            Ok(action) => action,
            Err(err) => {
                log::error!("failed to reconcile session lookup: {err}");
                return;
            }
        };
        let DesktopSessionBridgeAction::FocusSession { session_id } = action else {
            return;
        };
        let _ = self.activate_chatminal_session_target(&session_id, None);
    }

    fn close_chatminal_session_for_render_scope(&mut self, render_target_id: u64) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let Some(entry) = crate::chatminal_runtime::desktop_session_entry_binding_for_render_target(
            crate::chatminal_runtime::SessionRenderTargetId::new(render_target_id),
        ) else {
            return false;
        };
        self.close_chatminal_view_or_session_by_id(&entry.session_id)
    }

    fn render_scope_can_close_without_prompting(
        &self,
        render_target_id: u64,
        reason: CloseReason,
    ) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            let session_id =
                crate::chatminal_runtime::desktop_session_entry_binding_for_render_target(
                    crate::chatminal_runtime::SessionRenderTargetId::new(render_target_id),
                )
                .map(|entry| entry.session_id);
            return session_id
                .and_then(|session_id| crate::chatminal_runtime::desktop_pane_for_session(&session_id))
                .map(|pane| pane.can_close_without_prompting(reason))
                .unwrap_or(false);
        }
        crate::desktop_host_runtime::host_render_scope_can_close_without_prompting(
            render_target_id,
            reason,
        )
    }

    pub(crate) fn resize_split_via_tab_capability(
        &self,
        split: TerminalSplit,
        delta: isize,
    ) -> Option<TerminalSplit> {
        let render_target_id = if self.chatminal_sidebar.is_enabled() {
            self.render_capability_for_layout_split(split)
        } else {
            self.active_render_scope_id()
        }?;
        self.resize_render_scope_split(render_target_id, split, delta)
    }

    fn close_chatminal_session_by_id(&mut self, session_id: &str) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        if let Err(err) = self.chatminal_sidebar.close_session(&session_id) {
            log::error!("failed to close synced session {session_id}: {err}");
            return false;
        }
        if !crate::chatminal_runtime::desktop_detach_session_runtime_and_notify(session_id) {
            return false;
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
        self.sync_active_chatminal_session_from_mux();
        true
    }

    fn close_chatminal_view_by_id(&mut self, session_id: &str) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        if !crate::chatminal_runtime::desktop_detach_session_runtime_and_notify(session_id) {
            return false;
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
        self.sync_active_chatminal_session_from_mux();
        true
    }

    fn unjoin_chatminal_session_by_id(&mut self, session_id: &str) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }

        let snapshot = self.chatminal_sidebar.snapshot();
        let Some(target_session) = snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == session_id)
        else {
            return false;
        };

        let workspace_id =
            if snapshot.active_profile_id.as_deref() == Some(target_session.profile_id.as_str()) {
                DEFAULT_LAYOUT_WORKSPACE_ID.to_string()
            } else {
                DesktopWorkspaceLayoutStore::profile_workspace_id(&target_session.profile_id)
            };
        let store = DesktopWorkspaceLayoutStore::new(workspace_id);
        let Some(layout) = store.snapshot_or_restore() else {
            return false;
        };
        if layout.views.len() <= 1 {
            return false;
        }

        let Some(view_id) = store.view_id_for_session(session_id) else {
            return false;
        };

        let is_active_profile =
            snapshot.active_profile_id.as_deref() == Some(target_session.profile_id.as_str());

        let updated = match store.close_view(view_id) {
            Some(updated) => updated,
            None => return false,
        };
        let joined_session_ids = layout
            .views
            .iter()
            .map(|view| view.session_id.as_str())
            .collect::<Vec<_>>();
        store.clear_profile_group_layouts(&target_session.profile_id, joined_session_ids);

        if is_active_profile {
            if let Some(next_active_view) = updated.view(updated.active_view_id) {
                let _ = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID)
                    .save_as_profile_layout(&target_session.profile_id);
                let _ = crate::chatminal_runtime::desktop_focus_session_view_with_previous(
                    next_active_view.view_id,
                    Some(session_id.to_string()),
                );
                self.chatminal_sidebar
                    .set_active_session_local(&next_active_view.session_id);
                self.chatminal_sidebar
                    .select_single_session(&next_active_view.session_id);
            }
        } else {
            let _ = store.save_as_profile_layout(&target_session.profile_id);
        }

        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
        true
    }

    fn close_chatminal_view_or_session_by_id(&mut self, session_id: &str) -> bool {
        let can_close_view_only = crate::chatminal_runtime::desktop_can_close_view_only();

        if can_close_view_only {
            return self.close_chatminal_view_by_id(session_id);
        }
        self.close_chatminal_session_by_id(session_id)
    }

    fn prompt_rename_chatminal_session(&mut self, session_id: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        if self.chatminal_sidebar.start_inline_rename(session_id) {
            self.cancel_modal();
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }
    }

    fn prompt_startup_command_chatminal_session(&mut self, session_id: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        let snapshot = self.chatminal_sidebar.snapshot();
        let Some(session) = snapshot
            .sessions
            .into_iter()
            .find(|session| session.session_id == session_id)
        else {
            return;
        };
        self.set_modal(Rc::new(StartupRecipeModal::new(
            session.session_id,
            session.startup_command.unwrap_or_default(),
        )));
    }

    fn rename_chatminal_session(&mut self, session_id: &str, name: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        match self.chatminal_sidebar.rename_session(session_id, name) {
            Ok(workspace) => {
                self.chatminal_sidebar.apply_workspace(workspace);
                self.update_title_post_status();
                if let Some(window) = self.window.as_ref() {
                    window.invalidate();
                }
            }
            Err(err) => {
                log::error!("failed to rename sidebar session {session_id}: {err}");
            }
        }
    }

    fn set_startup_command_chatminal_session(&mut self, session_id: &str, command: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        match self
            .chatminal_sidebar
            .set_session_startup_command(session_id, Some(command))
        {
            Ok(workspace) => {
                self.chatminal_sidebar.apply_workspace(workspace);
                self.update_title_post_status();
                if let Some(window) = self.window.as_ref() {
                    window.invalidate();
                }
            }
            Err(err) => {
                log::error!("failed to set startup command for session {session_id}: {err}");
            }
        }
    }

    fn run_startup_command_chatminal_session(&mut self, session_id: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        self.switch_chatminal_session_target(session_id, None);
        if let Err(err) = crate::chatminal_runtime::run_runtime_session_startup_command(session_id) {
            log::error!("failed to run startup command for session {session_id}: {err}");
        }
    }

    fn switch_chatminal_profile(&mut self, profile_id: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        let snapshot = self.chatminal_sidebar.snapshot();
        if snapshot.active_profile_id.as_deref() == Some(profile_id) {
            return;
        }

        match self.chatminal_sidebar.switch_profile(profile_id) {
            Ok(workspace) => {
                let _ = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID)
                    .swap_profile_layout(
                        snapshot.active_profile_id.as_deref(),
                        profile_id,
                        workspace.active_session_id.as_deref(),
                    );
                self.apply_chatminal_profile_workspace(workspace);
            }
            Err(err) => {
                log::error!("failed to switch sidebar profile {profile_id}: {err}");
            }
        }
    }

    fn toggle_chatminal_profile(&mut self, profile_id: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        self.chatminal_sidebar.toggle_profile_expanded(profile_id);
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    fn create_chatminal_profile(&mut self) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        match self.chatminal_sidebar.create_profile() {
            Ok(profile) => {
                self.switch_chatminal_profile(&profile.profile_id);
            }
            Err(err) => {
                log::error!("failed to create sidebar profile: {err}");
            }
        }
    }

    fn apply_chatminal_profile_workspace(&mut self, workspace: RuntimeWorkspace) {
        let next_session_id = workspace.active_session_id.clone();
        self.chatminal_sidebar.apply_workspace(workspace);
        if let Some(session_id) = next_session_id {
            self.switch_chatminal_session(&session_id);
        } else {
            self.create_chatminal_session();
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    fn request_quit_application(&mut self) -> anyhow::Result<()> {
        let config = &self.config;
        log::info!("QuitApplication over here (window)");

        let should_prompt = !self.chatminal_sidebar.is_enabled()
            && matches!(
                config.window_close_confirmation,
                WindowCloseConfirmation::AlwaysPrompt
            );

        if should_prompt {
            if !self.spawn_overlay_on_active_render_scope(move |_tab_id, term| {
                confirm_quit_program(term)
            }) {
                anyhow::bail!("no active tab!?");
            }
        } else {
            if self.chatminal_sidebar.is_enabled() {
                if let Some(window) = self.window.as_ref() {
                    front_end().forget_known_window(window);
                    window.close();
                }
            }
            let con = Connection::get().expect("call on gui thread");
            con.terminate_message_loop();
        }

        Ok(())
    }

    fn close_requested(&mut self, _window: &Window) {
        if let Err(err) = self.request_quit_application() {
            log::error!("failed to quit application from close request: {err:#}");
            let con = Connection::get().expect("call on gui thread");
            con.terminate_message_loop();
        }
    }

    fn focus_changed(&mut self, focused: bool, window: &Window) {
        log::trace!("Setting focus to {:?}", focused);
        self.focused = if focused { Some(Instant::now()) } else { None };
        self.quad_generation += 1;
        self.load_os_parameters();

        if self.focused.is_none() {
            self.last_mouse_click = None;
            self.current_mouse_buttons.clear();
            self.current_mouse_capture = None;
            self.is_click_to_focus_window = false;

            for state in self.terminal_ui_state_by_handle.borrow_mut().values_mut() {
                state.mouse_terminal_coords.take();
            }
        }

        // Reset the cursor blink phase
        self.prev_cursor.bump();

        // force cursor to be repainted
        window.invalidate();

        if let Some(pane) = self.active_terminal_instance_or_overlay() {
            pane.focus_changed(focused);
        }

        self.update_title();
        self.emit_window_event("window-focus-changed", None);
    }

    fn created(&mut self, ctx: RenderContext) -> anyhow::Result<()> {
        self.render_state = None;

        let render_info = ctx.renderer_info();
        self.opengl_info.replace(render_info.clone());

        match RenderState::new(ctx, &self.fonts, &self.render_metrics, ATLAS_SIZE) {
            Ok(render_state) => {
                log::debug!(
                    "OpenGL initialized! {} chatminal-desktop version: {}",
                    render_info,
                    config::engine_version(),
                );
                self.render_state.replace(render_state);
            }
            Err(err) => {
                log::error!("failed to create RenderState: {}", err);
            }
        }

        if self.render_state.is_none() {
            panic!("No OpenGL");
        }

        Ok(())
    }
}

impl TermWindow {
    pub async fn new_primary_window(
        primary_window_id: PrimaryGuiWindowId,
    ) -> anyhow::Result<()> {
        let primary_host_window_id = PrimaryHostWindowId::try_from(primary_window_id)
            .context(format!("invalid primary gui window id {primary_window_id}"))?;
        let config = configuration();
        let chatminal_sidebar = ChatminalSidebar::new();
        let dpi = config.dpi.unwrap_or_else(|| ::window::default_dpi()) as usize;
        let fontconfig = Rc::new(FontConfiguration::new(Some(config.clone()), dpi)?);

        let size = if chatminal_sidebar.is_enabled() {
            Default::default()
        } else {
            match crate::chatminal_runtime::active_host_runtime_entry_size() {
                Some(size) => size,
                None => {
                    log::debug!("new_window has no tabs... yet?");
                    Default::default()
                }
            }
        };
        let physical_rows = size.rows as usize;
        let physical_cols = size.cols as usize;

        let render_metrics = RenderMetrics::new(&fontconfig)?;
        log::trace!("using render_metrics {:#?}", render_metrics);

        // Initially we have only a single tab, so take that into account
        // for the tab bar state.
        let show_session_bar =
            Self::should_show_session_bar_for_count(&config, 1, chatminal_sidebar.is_enabled());
        let tab_bar_height = if show_session_bar {
            Self::tab_bar_pixel_height_impl(&config, &fontconfig, &render_metrics)? as usize
        } else {
            0
        };

        let terminal_size = TerminalSize {
            rows: physical_rows,
            cols: physical_cols,
            pixel_width: (render_metrics.cell_size.width as usize * physical_cols),
            pixel_height: (render_metrics.cell_size.height as usize * physical_rows),
            dpi: dpi as u32,
        };

        if terminal_size != size {
            // DPI is different from the default assumed DPI when the mux
            // created the pty. We need to inform the kernel of the revised
            // pixel geometry now
            log::trace!(
                "Initial geometry was {:?} but dpi-adjusted geometry \
                        is {:?}; update the kernel pixel geometry for the ptys!",
                size,
                terminal_size,
            );
            if !chatminal_sidebar.is_enabled() {
                crate::chatminal_runtime::resize_host_window_tabs(terminal_size);
            }
        }

        let h_context = DimensionContext {
            dpi: dpi as f32,
            pixel_max: terminal_size.pixel_width as f32,
            pixel_cell: render_metrics.cell_size.width as f32,
        };
        let padding_left = config.window_padding.left.evaluate_as_pixels(h_context) as usize
            + Self::chatminal_sidebar_width_for_dimensions(terminal_size.pixel_width, dpi);
        let padding_right = resize::effective_right_padding(&config, h_context) as usize;
        let v_context = DimensionContext {
            dpi: dpi as f32,
            pixel_max: terminal_size.pixel_height as f32,
            pixel_cell: render_metrics.cell_size.height as f32,
        };
        let padding_top = config.window_padding.top.evaluate_as_pixels(v_context) as usize
            + Self::chatminal_terminal_chrome_height_for_dimensions(terminal_size.pixel_width, dpi);
        let padding_bottom = config.window_padding.bottom.evaluate_as_pixels(v_context) as usize
            + Self::chatminal_terminal_footer_height_for_dimensions(
                terminal_size.pixel_width,
                dpi,
                true,
            );

        let mut dimensions = Dimensions {
            pixel_width: (terminal_size.pixel_width + padding_left + padding_right) as usize,
            pixel_height: ((terminal_size.rows * render_metrics.cell_size.height as usize)
                + padding_top
                + padding_bottom) as usize
                + tab_bar_height,
            dpi,
        };

        let border = Self::get_os_border_impl(&None, &config, &dimensions, &render_metrics);

        dimensions.pixel_height += (border.top + border.bottom).get() as usize;
        dimensions.pixel_width += (border.left + border.right).get() as usize;

        let window_background = load_background_image(&config, &dimensions, &render_metrics);

        log::trace!(
            "TermWindow::new_primary_window called with primary_window_id {} {:?} {:?}",
            primary_window_id,
            terminal_size,
            dimensions
        );

        let render_state = None;

        let connection_name = Connection::get().unwrap().name();

        let myself = Self {
            created: Instant::now(),
            connection_name,
            last_fps_check_time: Instant::now(),
            num_frames: 0,
            last_frame_duration: Duration::ZERO,
            fps: 0.,
            config_subscription: None,
            os_parameters: None,
            gl: None,
            webgpu: None,
            window: None,
            window_background,
            chatminal_sidebar_seen_version: chatminal_sidebar.version(),
            chatminal_sidebar_poll_started: false,
            sidebar_tree_cache: None,
            sidebar_header_cache: None,
            sidebar_footer_background_cache: None,
            chatminal_sidebar,
            system_metrics: crate::system_metrics::SystemMetricsHandle::start(),
            metrics_tick_started: false,
            config: config.clone(),
            config_overrides: engine_dynamic::Value::default(),
            palette: None,
            focused: None,
            primary_host_window_id,
            fonts: Rc::clone(&fontconfig),
            render_metrics,
            dimensions,
            window_state: WindowState::default(),
            resizes_pending: 0,
            is_repaint_pending: false,
            pending_scale_changes: LinkedList::new(),
            terminal_size,
            render_state,
            input_map: InputMap::new(&config),
            leader_is_down: None,
            dead_key_status: DeadKeyStatus::None,
            show_session_bar,
            show_terminal_footer: true,
            show_scroll_bar: config.enable_scroll_bar,
            tab_bar: SessionBarState::default(),
            fancy_tab_bar: None,
            right_status: String::new(),
            left_status: String::new(),
            last_mouse_coords: (0, -1),
            window_drag_position: None,
            current_mouse_event: None,
            current_modifier_and_leds: Default::default(),
            prev_cursor: PrevCursorPos::new(),
            last_scroll_info: RenderableDimensions::default(),
            runtime_ui_state: RefCell::new(HashMap::new()),
            terminal_ui_state_by_handle: RefCell::new(HashMap::new()),
            current_mouse_buttons: vec![],
            current_mouse_capture: None,
            last_mouse_click: None,
            current_highlight: None,
            quad_generation: 0,
            shape_generation: 0,
            shape_cache: RefCell::new(LfuCache::new(
                "shape_cache.hit.rate",
                "shape_cache.miss.rate",
                |config| config.shape_cache_size,
                &config,
            )),
            line_state_cache: RefCell::new(LfuCacheU64::new(
                "line_state_cache.hit.rate",
                "line_state_cache.miss.rate",
                |config| config.line_state_cache_size,
                &config,
            )),
            next_line_state_id: 0,
            line_quad_cache: RefCell::new(LfuCache::new(
                "line_quad_cache.hit.rate",
                "line_quad_cache.miss.rate",
                |config| config.line_quad_cache_size,
                &config,
            )),
            line_to_ele_shape_cache: RefCell::new(LfuCache::new(
                "line_to_ele_shape_cache.hit.rate",
                "line_to_ele_shape_cache.miss.rate",
                |config| config.line_to_ele_shape_cache_size,
                &config,
            )),
            last_status_call: Instant::now(),
            cursor_blink_state: RefCell::new(ColorEase::new(
                config.cursor_blink_rate,
                config.cursor_blink_ease_in,
                config.cursor_blink_rate,
                config.cursor_blink_ease_out,
                None,
            )),
            blink_state: RefCell::new(ColorEase::new(
                config.text_blink_rate,
                config.text_blink_ease_in,
                config.text_blink_rate,
                config.text_blink_ease_out,
                None,
            )),
            rapid_blink_state: RefCell::new(ColorEase::new(
                config.text_blink_rate_rapid,
                config.text_blink_rapid_ease_in,
                config.text_blink_rate_rapid,
                config.text_blink_rapid_ease_out,
                None,
            )),
            event_states: HashMap::new(),
            current_event: None,
            has_animation: RefCell::new(None),
            scheduled_animation: RefCell::new(None),
            allow_images: AllowImage::Yes,
            semantic_zones: HashMap::new(),
            ui_items: vec![],
            dragging: None,
            last_ui_item: None,
            is_click_to_focus_window: false,
            key_table_state: KeyTableState::default(),
            modal: RefCell::new(None),
            opengl_info: None,
        };

        let tw = Rc::new(RefCell::new(myself));
        let tw_event = Rc::clone(&tw);

        let mut x = None;
        let mut y = None;
        let mut origin = GeometryOrigin::default();

        if let Some(position) = crate::chatminal_runtime::host_window_initial_position()
            .or_else(|| POSITION.lock().unwrap().take())
        {
            x.replace(position.x);
            y.replace(position.y);
            origin = position.origin;
        }

        let geometry = RequestedWindowGeometry {
            width: Dimension::Pixels(dimensions.pixel_width as f32),
            height: Dimension::Pixels(dimensions.pixel_height as f32),
            x,
            y,
            origin,
        };
        log::trace!("{:?}", geometry);

        let window = Window::new_window(
            &get_window_class(),
            "chatminal",
            geometry,
            Some(&config),
            Rc::clone(&fontconfig),
            move |event, window| {
                let mut tw = tw_event.borrow_mut();
                if let Err(err) = tw.dispatch_window_event(event, window) {
                    log::error!("dispatch_window_event: {:#}", err);
                }
            },
        )
        .await?;
        tw.borrow_mut().window.replace(window.clone());

        Self::apply_icon(&window)?;

        let config_subscription = config::subscribe_to_config_reload({
            let window = window.clone();
            move || {
                window.notify(TermWindowNotif::Apply(Box::new(|tw| {
                    tw.config_was_reloaded()
                })));
                true
            }
        });

        let gl = match config.front_end {
            FrontEndSelection::WebGpu => None,
            _ => Some(window.enable_opengl().await?),
        };

        {
            let mut myself = tw.borrow_mut();
            let webgpu = match config.front_end {
                FrontEndSelection::WebGpu => Some(Rc::new(
                    WebGpuState::new(&window, dimensions, &config).await?,
                )),
                _ => None,
            };
            myself.config_subscription.replace(config_subscription);
            if config.use_resize_increments {
                window.set_resize_increments(
                    ResizeIncrementCalculator {
                        x: myself.render_metrics.cell_size.width as u16,
                        y: myself.render_metrics.cell_size.height as u16,
                        padding_left: padding_left,
                        padding_top: padding_top,
                        padding_right: padding_right,
                        padding_bottom: padding_bottom,
                        border: border,
                        tab_bar_height: tab_bar_height,
                    }
                    .into(),
                );
            }

            if let Some(gl) = gl {
                myself.gl.replace(Rc::clone(&gl));
                myself.created(RenderContext::Glium(Rc::clone(&gl)))?;
            }
            if let Some(webgpu) = webgpu {
                myself.webgpu.replace(Rc::clone(&webgpu));
                myself.created(RenderContext::WebGpu(Rc::clone(&webgpu)))?;
            }
            myself.load_os_parameters();
            window.show();
            myself.initialize_chatminal_sidebar();
            myself.initialize_metrics_tick();
            myself.subscribe_to_runtime_updates();
            myself.emit_window_event("window-config-reloaded", None);
            myself.emit_status_event();
        }

        // update checker disabled — desktop deprecated, WezTerm update endpoint removed
        front_end().record_primary_window_binding(window, primary_window_id);

        Ok(())
    }

    fn dispatch_window_event(
        &mut self,
        event: WindowEvent,
        window: &Window,
    ) -> anyhow::Result<bool> {
        log::debug!("{event:?}");
        match event {
            WindowEvent::Destroyed => {
                // Ensure that we cancel any overlays we had running, so
                // that the mux can empty out, otherwise the mux keeps
                // the TermWindow alive via the frontend even though
                // the window is gone and we'll linger forever.
                // upstream issue #3522
                self.clear_all_overlays();
                Ok(false)
            }
            WindowEvent::CloseRequested => {
                self.close_requested(window);
                Ok(true)
            }
            WindowEvent::AppearanceChanged(appearance) => {
                log::debug!("Appearance is now {:?}", appearance);
                // This is a bit fugly; we get per-window notifications
                // for appearance changes which successfully updates the
                // per-window config, but we need to explicitly tell the
                // global config to reload, otherwise things that acces
                // the config via config::configuration() will see the
                // prior version of the config.
                // What's fugly about this is that we'll reload the
                // global config here once per window, which could
                // be nasty for folks with a lot of windows.
                // upstream issue #2295
                config::reload();
                self.config_was_reloaded();
                Ok(true)
            }
            WindowEvent::PerformKeyAssignment(action) => {
                if let Some(pane) = self.active_terminal_instance_or_overlay() {
                    self.perform_key_assignment(&pane, &action)?;
                    window.invalidate();
                }
                Ok(true)
            }
            WindowEvent::FocusChanged(focused) => {
                self.focus_changed(focused, window);
                Ok(true)
            }
            WindowEvent::MouseEvent(event) => {
                self.mouse_event_impl(event, window);
                Ok(true)
            }
            WindowEvent::MouseLeave => {
                self.mouse_leave_impl(window);
                Ok(true)
            }
            WindowEvent::Resized {
                dimensions,
                window_state,
                live_resizing,
            } => {
                self.resize(dimensions, window_state, window, live_resizing);
                Ok(true)
            }
            WindowEvent::SetInnerSizeCompleted => {
                self.resizes_pending -= 1;
                if self.is_repaint_pending {
                    self.is_repaint_pending = false;
                    if self.webgpu.is_some() {
                        self.do_paint_webgpu()?;
                    } else {
                        self.do_paint(window);
                    }
                }
                self.apply_pending_scale_changes();
                Ok(true)
            }
            WindowEvent::AdviseModifiersLedStatus(modifiers, leds) => {
                self.current_modifier_and_leds = (modifiers, leds);
                self.update_title();
                window.invalidate();
                Ok(true)
            }
            WindowEvent::RawKeyEvent(event) => {
                self.raw_key_event_impl(event, window);
                Ok(true)
            }
            WindowEvent::KeyEvent(event) => {
                self.key_event_impl(event, window);
                Ok(true)
            }
            WindowEvent::AdviseDeadKeyStatus(status) => {
                if self.config.debug_key_events {
                    log::info!("DeadKeyStatus now: {:?}", status);
                } else {
                    log::trace!("DeadKeyStatus now: {:?}", status);
                }
                if let Some(modal) = self.get_modal() {
                    match modal.composition_status_changed(&status, self) {
                        Ok(true) => {}
                        Ok(false) => {}
                        Err(err) => {
                            log::error!("Error dispatching composition status to modal: {err:#}");
                        }
                    }
                }
                if let Some(pane) = self.active_terminal_instance_or_overlay() {
                    if let Some(copy_overlay) = pane.downcast_ref::<CopyOverlay>() {
                        copy_overlay.apply_composition_status(&status);
                    }
                }
                self.dead_key_status = status;
                self.update_title();
                // Ensure that we repaint so that any composing
                // text is updated
                window.invalidate();
                Ok(true)
            }
            WindowEvent::NeedRepaint => {
                if self.resizes_pending > 0 {
                    self.is_repaint_pending = true;
                    Ok(true)
                } else if self.webgpu.is_some() {
                    self.do_paint_webgpu()
                } else {
                    Ok(self.do_paint(window))
                }
            }
            WindowEvent::Notification(item) => {
                if let Ok(notif) = item.downcast::<TermWindowNotif>() {
                    self.dispatch_notif(*notif, window)
                        .context("dispatch_notif")?;
                }
                Ok(true)
            }
            WindowEvent::DroppedString(text) => {
                let pane = match self.active_terminal_instance_or_overlay() {
                    Some(pane) => pane,
                    None => return Ok(true),
                };
                pane.send_paste(text.as_str())?;
                Ok(true)
            }
            WindowEvent::DroppedUrl(urls) => {
                let pane = match self.active_terminal_instance_or_overlay() {
                    Some(pane) => pane,
                    None => return Ok(true),
                };
                let urls = urls
                    .iter()
                    .map(|url| self.config.quote_dropped_files.escape(&url.to_string()))
                    .collect::<Vec<_>>()
                    .join(" ")
                    + " ";
                pane.send_paste(urls.as_str())?;
                Ok(true)
            }
            WindowEvent::DroppedFile(paths) => {
                let pane = match self.active_terminal_instance_or_overlay() {
                    Some(pane) => pane,
                    None => return Ok(true),
                };
                let paths = paths
                    .iter()
                    .map(|path| {
                        self.config
                            .quote_dropped_files
                            .escape(&path.to_string_lossy())
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
                    + " ";
                pane.send_paste(&paths)?;
                Ok(true)
            }
            WindowEvent::DraggedFile(_) => Ok(true),
        }
    }

    fn do_paint(&mut self, window: &Window) -> bool {
        let gl = match self.gl.as_ref() {
            Some(gl) => gl,
            None => return false,
        };

        if gl.is_context_lost() {
            log::error!("opengl context was lost; should reinit");
            window.close();
            front_end().forget_known_window(window);
            return false;
        }

        let mut frame = glium::Frame::new(
            Rc::clone(&gl),
            (
                self.dimensions.pixel_width as u32,
                self.dimensions.pixel_height as u32,
            ),
        );
        self.paint_impl(&mut RenderFrame::Glium(&mut frame));
        window.finish_frame(frame).is_ok()
    }

    fn do_paint_webgpu(&mut self) -> anyhow::Result<bool> {
        self.webgpu.as_mut().unwrap().resize(self.dimensions);
        match self.do_paint_webgpu_impl() {
            Ok(ok) => Ok(ok),
            Err(err) => {
                match err.downcast_ref::<wgpu::SurfaceError>() {
                    Some(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.webgpu.as_mut().unwrap().resize(self.dimensions);
                        return self.do_paint_webgpu_impl();
                    }
                    _ => {}
                }
                Err(err)
            }
        }
    }

    fn do_paint_webgpu_impl(&mut self) -> anyhow::Result<bool> {
        self.paint_impl(&mut RenderFrame::WebGpu);
        Ok(true)
    }

    fn dispatch_notif(&mut self, notif: TermWindowNotif, window: &Window) -> anyhow::Result<()> {
        fn chan_err<T>(e: smol::channel::TrySendError<T>) -> anyhow::Error {
            anyhow::anyhow!("{}", e)
        }

        match notif {
            TermWindowNotif::InvalidateShapeCache => {
                self.shape_generation += 1;
                self.shape_cache.borrow_mut().clear();
                self.sidebar_tree_cache = None;
                self.sidebar_header_cache = None;
                self.sidebar_footer_background_cache = None;
                self.invalidate_modal();
                window.invalidate();
            }
            TermWindowNotif::PerformAssignmentForTerminalHandle {
                terminal_handle,
                assignment,
                tx,
            } => {
                let result = self
                    .resolve_public_terminal_instance(terminal_handle)
                    .ok_or_else(|| anyhow!("terminal handle {} is not valid", terminal_handle))
                    .and_then(|pane| {
                        self.perform_key_assignment(&pane, &assignment)
                            .context("perform_key_assignment")?;
                        Ok(())
                    });
                window.invalidate();
                if let Some(tx) = tx {
                    tx.try_send(result).ok();
                }
            }
            TermWindowNotif::SetRightStatus(status) => {
                if status != self.right_status {
                    self.right_status = status;
                    self.update_title_post_status();
                } else {
                    self.schedule_next_status_update();
                }
            }
            TermWindowNotif::SetLeftStatus(status) => {
                if status != self.left_status {
                    self.left_status = status;
                    self.update_title_post_status();
                } else {
                    self.schedule_next_status_update();
                }
            }
            TermWindowNotif::GetDimensions(tx) => {
                tx.try_send((self.dimensions, self.window_state))
                    .map_err(chan_err)
                    .context("send GetDimensions response")?;
            }
            TermWindowNotif::GetEffectiveConfig(tx) => {
                tx.try_send(self.config.clone())
                    .map_err(chan_err)
                    .context("send GetEffectiveConfig response")?;
            }
            TermWindowNotif::FinishWindowEvent { name, again } => {
                self.finish_window_event(&name, again);
            }
            TermWindowNotif::GetConfigOverrides(tx) => {
                tx.try_send(self.config_overrides.clone())
                    .map_err(chan_err)
                    .context("send GetConfigOverrides response")?;
            }
            TermWindowNotif::SetConfigOverrides(value) => {
                if value != self.config_overrides {
                    self.config_overrides = value;
                    self.config_was_reloaded();
                }
            }
            TermWindowNotif::CancelOverlayForTerminalHandle(terminal_handle) => {
                let pane_id = TerminalUiKey::try_from(terminal_handle)
                    .map_err(|_| anyhow!("invalid terminal handle {terminal_handle}"))?;
                self.cancel_overlay_for_terminal_handle(pane_id);
            }
            TermWindowNotif::CancelOverlayForRenderScope {
                render_target_id,
                pane_id,
            } => {
                let pane_id = pane_id
                    .map(|pane_id| {
                        TerminalUiKey::try_from(pane_id)
                            .map_err(|_| anyhow!("invalid pane id {pane_id}"))
                    })
                    .transpose()?;
                self.cancel_overlay_for_render_scope(render_target_id, pane_id);
            }
            TermWindowNotif::RuntimeNotification(n) => match n {
                RuntimeNotification::Alert {
                    alert: Alert::SetUserVar { name, value },
                    pane_id,
                } => {
                    self.emit_user_var_event(pane_id as u64, name, value);
                }
                RuntimeNotification::WindowTitleChanged { .. }
                | RuntimeNotification::Alert {
                    alert:
                        Alert::OutputSinceFocusLost
                        | Alert::CurrentWorkingDirectoryChanged
                        | Alert::WindowTitleChanged(_)
                        | Alert::TabTitleChanged(_)
                        | Alert::IconTitleChanged(_)
                        | Alert::Progress(_),
                    ..
                } => {
                    self.update_title();
                }
                RuntimeNotification::Alert {
                    alert: Alert::PaletteChanged,
                    pane_id,
                } => {
                    // Shape cache includes color information, so
                    // ensure that we invalidate that as part of
                    // this overall invalidation for the palette
                    self.dispatch_notif(TermWindowNotif::InvalidateShapeCache, window)?;
                    self.handle_pane_output_event(pane_id as u64);
                }
                RuntimeNotification::Alert {
                    alert: Alert::Bell,
                    pane_id,
                } => {
                    let pane_id = pane_id as u64;
                    if !self.window_contains_pane(pane_id) {
                        return Ok(());
                    }

                    match self.config.audible_bell {
                        AudibleBell::SystemBeep => {
                            Connection::get().expect("on main thread").beep();
                        }
                        AudibleBell::Disabled => {}
                    }

                    log::trace!("Ding! (this is the bell) in pane {}", pane_id);
                    self.emit_window_event("bell", Some(pane_id));

                    let mut per_pane = self.terminal_ui_state(pane_id);
                    per_pane.bell_start.replace(Instant::now());
                    window.invalidate();
                }
                RuntimeNotification::Alert {
                    alert: Alert::ToastNotification { .. },
                    ..
                } => {}
                RuntimeNotification::TabAddedToWindow { tab_id } => {
                    let mut size = self.terminal_size;
                    if let Some(tab_size) = self.render_scope_size(tab_id as u64) {
                        // If we attached to a remote target and loaded in
                        // a tab async, we need to fixup its size, either
                        // by resizing it or resizes ourselves.
                        // The strategy here is to adjust both by taking
                        // the maximal size in both horizontal and vertical
                        // dimensions and applying that. In practice that
                        // means that a new local client will resize larger
                        // to adjust to the size of an existing client.
                        size.rows = size.rows.max(tab_size.rows);
                        size.cols = size.cols.max(tab_size.cols);

                        if size.rows != self.terminal_size.rows
                            || size.cols != self.terminal_size.cols
                            || size.pixel_width != self.terminal_size.pixel_width
                            || size.pixel_height != self.terminal_size.pixel_height
                        {
                            self.set_window_size(size, window)?;
                        } else if tab_size.dpi == 0 {
                            log::debug!("fixup dpi in newly added tab");
                            let _ = self.resize_render_scope(tab_id as u64, self.terminal_size);
                        }
                    }
                }
                RuntimeNotification::PaneOutput(pane_id) => {
                    self.handle_pane_output_event(pane_id as u64);
                }
                RuntimeNotification::WindowInvalidated => {
                    window.invalidate();
                    self.update_title_post_status();
                }
                RuntimeNotification::AssignClipboard { .. } => {
                    // Handled by frontend
                }
                RuntimeNotification::SaveToDownloads { .. } => {
                    // Handled by frontend
                }
                RuntimeNotification::PaneFocused(_) => {
                    // Also handled by clientpane
                    self.update_title_post_status();
                }
                RuntimeNotification::TabResized(_) => {
                    // Also handled by engine-client
                    self.update_title_post_status();
                }
                RuntimeNotification::TabTitleChanged { .. } => {
                    self.update_title_post_status();
                }
                RuntimeNotification::PaneAdded(_)
                | RuntimeNotification::WorkspaceRenamed { .. }
                | RuntimeNotification::PaneRemoved(_)
                | RuntimeNotification::WindowWorkspaceChanged
                | RuntimeNotification::ActiveWorkspaceChanged(_)
                | RuntimeNotification::Empty => {}
            },
            TermWindowNotif::EmitStatusUpdate => {
                self.emit_status_event();
            }
            TermWindowNotif::Apply(func) => {
                func(self);
            }
            TermWindowNotif::SetInnerSize { width, height } => {
                self.set_inner_size(window, width, height);
            }
        }

        Ok(())
    }

    fn set_inner_size(&mut self, window: &Window, width: usize, height: usize) {
        self.resizes_pending += 1;
        window.set_inner_size(width, height);
    }

    /// Take care to remove our panes from the mux, otherwise
    /// we can leave the mux with no windows but some panes
    /// and it won't believe that we are empty.
    fn clear_all_overlays(&mut self) {
        let overlay_panes_to_cancel = self
            .terminal_ui_state_by_handle
            .borrow()
            .iter()
            .filter_map(|(_, state)| {
                state
                    .overlay
                    .as_ref()
                    .map(|overlay| overlay.pane.pane_id() as u64)
            })
            .collect::<Vec<_>>();

        for pane_id in overlay_panes_to_cancel {
            self.cancel_overlay_for_terminal_handle(pane_id);
        }

        let tab_overlays_to_cancel = self
            .runtime_ui_state
            .borrow()
            .iter()
            .filter_map(|(tab_id, state)| state.overlay.as_ref().map(|_| *tab_id))
            .collect::<Vec<_>>();

        for tab_id in tab_overlays_to_cancel {
            self.cancel_overlay_for_render_scope(tab_id as u64, None);
        }

        self.terminal_ui_state_by_handle.borrow_mut().clear();
        self.runtime_ui_state.borrow_mut().clear();
    }

    fn apply_icon(window: &Window) -> anyhow::Result<()> {
        let image = image::load_from_memory(ICON_DATA)?.into_rgba8();
        let (width, height) = image.dimensions();
        window.set_icon(Image::with_rgba32(
            width as usize,
            height as usize,
            width as usize * 4,
            image.as_raw(),
        ));
        Ok(())
    }

    fn schedule_status_update(&self) {
        if let Some(window) = self.window.as_ref() {
            window.notify(TermWindowNotif::EmitStatusUpdate);
        }
    }

    fn is_pane_visible(&mut self, pane_id: TerminalUiKey) -> bool {
        self.get_panes_to_render()
            .into_iter()
            .any(|pos| pos.pane.pane_id() as u64 == pane_id)
    }

    fn handle_pane_output_event(&mut self, pane_id: TerminalUiKey) {
        metrics::histogram!("mux.pane_output_event.rate").record(1.);
        if self.is_pane_visible(pane_id) {
            if let Some(ref win) = self.window {
                win.invalidate();
            }
        }
    }

    fn handle_runtime_notification_callback(
        n: RuntimeNotification,
        window: &Window,
        primary_host_window_id: PrimaryHostWindowId,
        dead: &Arc<AtomicBool>,
    ) -> bool {
        if dead.load(Ordering::Relaxed) {
            // Subscription cancelled asynchronously
            return false;
        }

        match n {
            RuntimeNotification::Alert {
                pane_id,
                alert:
                    Alert::OutputSinceFocusLost
                    | Alert::CurrentWorkingDirectoryChanged
                    | Alert::WindowTitleChanged(_)
                    | Alert::TabTitleChanged(_)
                    | Alert::IconTitleChanged(_)
                    | Alert::Progress(_)
                    | Alert::SetUserVar { .. }
                    | Alert::Bell,
            }
            | RuntimeNotification::PaneFocused(pane_id)
            | RuntimeNotification::PaneRemoved(pane_id)
            | RuntimeNotification::PaneOutput(pane_id) => {
                // Ideally we'd check to see if pane_id is part of this window,
                // but overlays may not be 100% associated with the window
                // in the mux and we don't want to lose the invalidation
                // signal for that case, so we just check window validity
                // here and propagate to the window event handler that
                // will then do the check with full context.
                if !Self::host_window_exists() {
                    // Something inconsistent: cancel subscription
                    log::debug!(
                        "PaneOutput: wanted primary_host_window_id={} from mux, but \
                         was not found, cancel mux subscription",
                        primary_host_window_id
                    );
                    return false;
                }
                let _ = pane_id;
            }
            RuntimeNotification::PaneAdded(_pane_id) => {
                // If some other client spawns a pane inside this window, this
                // gives us an opportunity to attach it to the clipboard.
                return Self::host_window_exists();
            }
            RuntimeNotification::TabAddedToWindow { .. }
            | RuntimeNotification::WindowTitleChanged { .. }
            | RuntimeNotification::WindowInvalidated => {
            }
            RuntimeNotification::TabResized(tab_id)
            | RuntimeNotification::TabTitleChanged { tab_id, .. } => {
                if Self::host_window_contains_render_scope(tab_id as u64) {
                    // fall through
                } else {
                    return true;
                }
            }
            RuntimeNotification::Alert {
                alert: Alert::ToastNotification { .. },
                ..
            }
            | RuntimeNotification::AssignClipboard { .. }
            | RuntimeNotification::SaveToDownloads { .. }
            | RuntimeNotification::ActiveWorkspaceChanged(_)
            | RuntimeNotification::WorkspaceRenamed { .. }
            | RuntimeNotification::Empty
            | RuntimeNotification::WindowWorkspaceChanged => return true,
            RuntimeNotification::Alert {
                alert: Alert::PaletteChanged { .. },
                ..
            } => {
                // fall through
            }
        }

        window.notify(TermWindowNotif::RuntimeNotification(n));

        true
    }

    fn subscribe_to_runtime_updates(&self) {
        let window = self.window.clone().expect("window to be valid on startup");
        let primary_host_window_id = self.primary_host_window_id;
        let dead = Arc::new(AtomicBool::new(false));
        crate::chatminal_runtime::subscribe_runtime_notifications(move |n| {
            if dead.load(Ordering::Relaxed) {
                return false;
            }
            let window = window.clone();
            let dead = dead.clone();
            promise::spawn::spawn_into_main_thread(async move {
                Self::handle_runtime_notification_callback(
                    n,
                    &window,
                    primary_host_window_id,
                    &dead,
                )
            })
            .detach();
            true
        });
    }

    fn positioned_pane_to_terminal_instance_info(
        &self,
        pos: &TerminalPaneLayout,
    ) -> TerminalInstanceInformation {
        let host_terminal_handle = pos.pane.pane_id() as u64;
        let terminal_instance_id = crate::chatminal_runtime::desktop_session_terminal_binding(
            crate::chatminal_runtime::SessionTerminalHandle::new(host_terminal_handle),
        )
        .map(|binding| binding.terminal_instance_id.as_u64())
        .or_else(|| {
            pane_metadata_terminal_instance_id(&*pos.pane)
                .map(|terminal_instance_id| terminal_instance_id.as_u64())
        })
        .unwrap_or(host_terminal_handle);
        TerminalInstanceInformation {
            host_terminal_handle,
            terminal_instance_id,
            terminal_index: pos.index,
            is_active: pos.is_active,
            is_zoomed: pos.is_zoomed,
            has_unseen_output: pos.pane.has_unseen_output(),
            left: pos.left,
            top: pos.top,
            width: pos.width,
            height: pos.height,
            pixel_width: pos.pixel_width,
            pixel_height: pos.pixel_height,
            title: pos.pane.get_title(),
            user_vars: pos.pane.copy_user_vars(),
            progress: pos.pane.get_progress(),
        }
    }

    fn get_session_entry_information(&mut self) -> Vec<SessionEntryInformation> {
        if self.chatminal_sidebar.is_enabled() {
            let entry_bindings = crate::chatminal_runtime::desktop_session_entry_bindings();
            let leaves_by_session: HashMap<String, Vec<TerminalInstanceInformation>> =
                entry_bindings
                    .iter()
                    .filter_map(|entry| {
                        let panes = self.positioned_panes_for_session(&entry.session_id);
                        if panes.is_empty() {
                            return None;
                        }
                        Some((
                            entry.session_id.clone(),
                            panes
                                .iter()
                                .map(|pane| self.positioned_pane_to_terminal_instance_info(pane))
                                .collect(),
                        ))
                    })
                    .collect();

            return entry_bindings
                .into_iter()
                .map(|entry| {
                    let terminal_instances = leaves_by_session
                        .get(&entry.session_id)
                        .cloned()
                        .unwrap_or_default();
                    let active_terminal_instance = terminal_instances
                        .iter()
                        .find(|leaf| leaf.is_active)
                        .cloned();

                    SessionEntryInformation {
                        entry_index: entry.entry_index,
                        render_target_id: entry.render_target_id.map(|id| id.as_u64()).unwrap_or(0),
                        is_active: entry.is_active,
                        is_last_active: entry.is_last_active,
                        entry_title: entry.title,
                        active_terminal_instance,
                        terminal_instances,
                        session_id: Some(entry.session_id),
                        view_id: entry.view_id,
                    }
                })
                .collect();
        }

        self.with_host_window(|window| {
            let tab_index = window.get_active_idx();
            let last_active_idx = window.get_last_active_idx();

            window
                .iter()
                .enumerate()
                .map(|(idx, tab)| {
                    let terminal_instances = self
                        .get_positioned_panes_for_render_scope(tab.tab_id() as u64)
                        .iter()
                        .map(|pane| self.positioned_pane_to_terminal_instance_info(pane))
                        .collect::<Vec<_>>();

                    SessionEntryInformation {
                        entry_index: idx,
                        render_target_id: tab.tab_id() as u64,
                        is_active: tab_index == idx,
                        is_last_active: last_active_idx
                            .map(|last_active| last_active == idx)
                            .unwrap_or(false),
                        entry_title: tab.get_title(),
                        active_terminal_instance: terminal_instances
                            .iter()
                            .find(|leaf| leaf.is_active)
                            .cloned(),
                        terminal_instances,
                        session_id: None,
                        view_id: None,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
    }

    fn get_terminal_instance_information(&self) -> Vec<TerminalInstanceInformation> {
        self.get_panes_to_render()
            .iter()
            .map(|pane| self.positioned_pane_to_terminal_instance_info(pane))
            .collect()
    }

    fn get_positioned_panes_for_render_scope(
        &self,
        render_target_id: u64,
    ) -> Vec<TerminalPaneLayout> {
        let Some(size) = self.render_scope_size(render_target_id) else {
            return vec![];
        };

        if let Some(pane) = self.render_target_overlay(render_target_id) {
            vec![TerminalPaneLayout {
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 0,
                top: 0,
                width: size.cols as _,
                height: size.rows as _,
                pixel_width: size.cols as usize * self.render_metrics.cell_size.width as usize,
                pixel_height: size.rows as usize * self.render_metrics.cell_size.height as usize,
                pane,
            }]
        } else {
            let mut panes =
                crate::desktop_host_runtime::host_overlay_pane_layouts_by_id(render_target_id);
            for p in &mut panes {
                if let Some(overlay) = self
                    .terminal_ui_state(p.pane.pane_id() as u64)
                    .overlay
                    .as_ref()
                {
                    p.pane = Arc::clone(&overlay.pane);
                }
            }
            panes
                .into_iter()
                .map(TerminalPaneLayout::from_mux)
                .collect()
        }
    }

    fn get_panes_to_render(&self) -> Vec<TerminalPaneLayout> {
        let layout_panes = self.layout_positioned_panes();
        if !layout_panes.is_empty() {
            return layout_panes;
        }
        if self.chatminal_sidebar.is_enabled() {
            if let Some(session_id) = self.active_session_id() {
                let panes = self.positioned_panes_for_session(&session_id);
                if !panes.is_empty() {
                    return panes;
                }
            }
        }
        self.active_render_target_positioned_panes()
    }

    /// If `host_terminal_handle` is `None`, removes any overlay for the specified host tab.
    /// Otherwise removes the overlay only if it belongs to the specified host leaf.
    fn resolve_search_pattern(
        &self,
        pattern: Pattern,
        pane: &Arc<dyn OverlayPane>,
    ) -> OverlayPattern {
        match pattern {
            Pattern::CaseSensitiveString(s) => OverlayPattern::CaseSensitiveString(s),
            Pattern::CaseInSensitiveString(s) => OverlayPattern::CaseInSensitiveString(s),
            Pattern::Regex(s) => OverlayPattern::Regex(s),
            Pattern::CurrentSelectionOrEmptyString => {
                let text = self.selection_text(pane);
                let first_line = text
                    .lines()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                OverlayPattern::CaseSensitiveString(first_line)
            }
        }
    }
}

include!("../desktop_termwindow_actions_items.rs");
include!("../desktop_termwindow_close_helpers.rs");
include!("../desktop_termwindow_event_helpers.rs");
include!("../desktop_termwindow_host_runtime_helpers.rs");
include!("../desktop_termwindow_overlay_helpers.rs");
include!("../desktop_termwindow_positioned_session_helpers.rs");
include!("../desktop_termwindow_session_close_helpers.rs");
include!("../desktop_termwindow_selection.rs");
include!("../desktop_termwindow_spawn.rs");
include!("../desktop_termwindow_state_helpers.rs");

impl Drop for TermWindow {
    fn drop(&mut self) {
        self.clear_all_overlays();
        if let Some(window) = self.window.take() {
            if let Some(fe) = try_front_end() {
                fe.forget_known_window(&window);
            }
        }
    }
}
