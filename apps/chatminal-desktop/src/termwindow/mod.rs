#![allow(clippy::range_plus_one)]
use super::renderstate::*;
use super::utilsprites::RenderMetrics;
use crate::chatminal_session_surface;
use crate::chatminal_sidebar::ChatminalSidebar;
use crate::colorease::ColorEase;
use crate::frontend::{front_end, try_front_end};
use crate::inputmap::InputMap;
use crate::overlay::{
    confirm_close_pane, confirm_close_tab, confirm_close_window, confirm_quit_program, launcher,
    start_overlay, start_overlay_pane, CopyModeParams, CopyOverlay, LauncherArgs, LauncherFlags,
    QuickSelectOverlay,
};
use crate::resize_increment_calculator::ResizeIncrementCalculator;
use crate::scripting::guiwin::DesktopWindowId;
use crate::scripting::guiwin::GuiWin;
use crate::scrollbar::*;
use crate::selection::Selection;
use crate::shapecache::*;
use crate::tabbar::{TabBarItem, TabBarState};
use crate::termwindow::background::{
    load_background_image, reload_background_image, LoadedBackgroundLayer,
};
use crate::termwindow::keyevent::{KeyTableArgs, KeyTableState};
use crate::termwindow::modal::Modal;
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
use chatminal_session_runtime::{LeafId, SurfaceId};
use config::keyassignment::{
    Confirmation, KeyAssignment, LauncherActionArgs, PaneDirection, Pattern, PromptInputLine,
    QuickSelectArguments, RotationDirection, SpawnCommand, SplitSize,
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
use mux::pane::{
    CachePolicy, CloseReason, Pane, PaneId, Pattern as MuxPattern, PerformAssignmentResult,
};
use mux::renderable::RenderableDimensions;
use mux::tab::{
    PositionedPane, PositionedSplit, SplitDirection, SplitRequest, SplitSize as MuxSplitSize, Tab,
    TabId,
};
use mux::window::WindowId as EngineWindowId;
use mux::{Mux, MuxNotification};
use smol::channel::Sender;
use smol::Timer;
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, LinkedList};
use std::convert::TryFrom;
use std::ops::Add;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use termwiz::hyperlink::Hyperlink;
use termwiz::surface::SequenceNo;
use termwiz_funcs::lines_to_escapes;

pub mod background;
pub mod box_model;
pub mod charselect;
pub mod clipboard;
pub mod keyevent;
pub mod modal;
mod mouseevent;
pub mod palette;
pub mod paneselect;
mod prevcursor;
pub mod render;
pub mod resize;
mod selection;
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

fn pane_metadata_u64(pane: &dyn Pane, key: &str) -> Option<u64> {
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

fn pane_metadata_surface_id(pane: &dyn Pane) -> Option<SurfaceId> {
    pane_metadata_u64(pane, "chatminal_surface_id").map(SurfaceId::new)
}

fn pane_metadata_leaf_id(pane: &dyn Pane) -> Option<LeafId> {
    pane_metadata_u64(pane, "chatminal_leaf_id").map(LeafId::new)
}

fn pane_matches_public_id(pane: &dyn Pane, public_id: u64) -> bool {
    pane.pane_id() as u64 == public_id
        || pane_metadata_leaf_id(pane)
            .map(|leaf_id| leaf_id.as_u64() == public_id)
            .unwrap_or(false)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MouseCapture {
    UI,
    TerminalPane(PaneId),
}

/// Type used together with Window::notify to do something in the
/// context of the window-specific event loop
pub enum TermWindowNotif {
    InvalidateShapeCache,
    PerformAssignmentForLeafId {
        leaf_id: u64,
        assignment: KeyAssignment,
        tx: Option<Sender<anyhow::Result<()>>>,
    },
    SetLeftStatus(String),
    SetRightStatus(String),
    GetDimensions(Sender<(Dimensions, WindowState)>),
    GetSelectionForLeafId {
        leaf_id: u64,
        tx: Sender<String>,
    },
    GetSelectionEscapesForLeafId {
        leaf_id: u64,
        tx: Sender<anyhow::Result<String>>,
    },
    GetEffectiveConfig(Sender<ConfigHandle>),
    FinishWindowEvent {
        name: String,
        again: bool,
    },
    GetConfigOverrides(Sender<engine_dynamic::Value>),
    SetConfigOverrides(engine_dynamic::Value),
    CancelOverlayForLeafId(u64),
    CancelOverlayForSurfaceId {
        surface_id: u64,
        pane_id: Option<u64>,
    },
    CancelOverlayForHostSurfaceId {
        host_surface_id: u64,
        pane_id: Option<u64>,
    },
    MuxNotification(MuxNotification),
    EmitStatusUpdate,
    Apply(Box<dyn FnOnce(&mut TermWindow) + Send + Sync>),
    SwitchToWindowId(DesktopWindowId),
    SetInnerSize {
        width: usize,
        height: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UIItemType {
    TabBar(TabBarItem),
    CloseTab(usize),
    CloseSession(String),
    AboveScrollThumb,
    ScrollThumb,
    BelowScrollThumb,
    Split(PositionedSplit),
    ChatminalSidebarBackground,
    ChatminalSidebarCreateProfile,
    ChatminalSidebarProfile(String),
    ChatminalSidebarCreateSession,
    ChatminalSidebarSession(String),
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
    pub pane: Arc<dyn Pane>,
    pub key_table_state: KeyTableState,
}

#[derive(Default)]
pub struct LeafUiState {
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
pub struct SurfaceInformation {
    pub host_surface_id: u64,
    pub surface_index: usize,
    pub is_active: bool,
    pub is_last_active: bool,
    pub active_leaf: Option<LeafInformation>,
    pub leaves: Vec<LeafInformation>,
    pub window_id: DesktopWindowId,
    pub surface_title: String,
    pub session_id: Option<String>,
    pub surface_id: Option<SurfaceId>,
    pub active_leaf_id: Option<LeafId>,
}

impl SurfaceInformation {
    fn resolved_window_title(&self) -> mlua::Result<String> {
        let mux = Mux::try_get().ok_or_else(|| mlua::Error::external("no mux"))?;
        let window_id = EngineWindowId::try_from(self.window_id)
            .map_err(|_| mlua::Error::external(format!("invalid window id {}", self.window_id)))?;
        let window = mux
            .get_window(window_id)
            .ok_or_else(|| mlua::Error::external(format!("window {} not found", self.window_id)))?;
        Ok(window.get_title().to_string())
    }
}

impl UserData for SurfaceInformation {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("host_surface_id", |_, this| Ok(this.host_surface_id));
        fields.add_field_method_get("surface_index", |_, this| Ok(this.surface_index));
        fields.add_field_method_get("is_active", |_, this| Ok(this.is_active));
        fields.add_field_method_get("is_last_active", |_, this| Ok(this.is_last_active));
        fields.add_field_method_get("active_leaf", |_, this| Ok(this.active_leaf.clone()));
        fields.add_field_method_get("leaves", |_, this| Ok(this.leaves.clone()));
        fields.add_field_method_get("window_id", |_, this| Ok(this.window_id));
        fields.add_field_method_get("surface_title", |_, this| Ok(this.surface_title.clone()));
        fields.add_field_method_get("session_id", |_, this| Ok(this.session_id.clone()));
        fields.add_field_method_get("surface_id", |_, this| {
            Ok(this.surface_id.map(|value| value.as_u64()))
        });
        fields.add_field_method_get("active_leaf_id", |_, this| {
            Ok(this.active_leaf_id.map(|value| value.as_u64()))
        });
        fields.add_field_method_get("window_title", |_, this| this.resolved_window_title());
    }
}

/// Data used when synchronously formatting pane and window titles
#[derive(Debug, Clone)]
pub struct LeafInformation {
    pub host_leaf_id: u64,
    pub leaf_id: u64,
    pub leaf_index: usize,
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

impl LeafInformation {
    fn resolved_pane(&self) -> Option<Arc<dyn Pane>> {
        let mux = Mux::try_get()?;
        PaneId::try_from(self.host_leaf_id)
            .ok()
            .and_then(|pane_id| mux.get_pane(pane_id))
            .or_else(|| {
                PaneId::try_from(self.leaf_id)
                    .ok()
                    .and_then(|pane_id| mux.get_pane(pane_id))
            })
            .or_else(|| {
                mux.iter_panes()
                    .into_iter()
                    .find(|pane| pane_matches_public_id(&**pane, self.leaf_id))
            })
    }
}

impl UserData for LeafInformation {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("host_leaf_id", |_, this| Ok(this.host_leaf_id));
        fields.add_field_method_get("leaf_id", |_, this| Ok(this.leaf_id));
        fields.add_field_method_get("leaf_index", |_, this| Ok(this.leaf_index));
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
        fields.add_field_method_get("domain_name", |_, this| {
            if let Some(mux) = Mux::try_get() {
                if let Some(pane) = this.resolved_pane() {
                    let domain_id = pane.domain_id();
                    let name = mux
                        .get_domain(domain_id)
                        .map(|dom| dom.domain_name().to_string());
                    return Ok(name.unwrap_or_default());
                }
            }
            Ok(String::new())
        });
    }
}

#[derive(Default)]
pub struct SurfaceUiState {
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
    InProgressWithQueued(Option<PaneId>),
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
    pub window_id: EngineWindowId,
    pub window_id_for_subscriptions: Arc<Mutex<EngineWindowId>>,
    pub render_metrics: RenderMetrics,
    render_state: Option<RenderState>,
    input_map: InputMap,
    /// If is_some, the LEADER modifier is active until the specified instant.
    leader_is_down: Option<std::time::Instant>,
    dead_key_status: DeadKeyStatus,
    key_table_state: KeyTableState,
    show_tab_bar: bool,
    show_scroll_bar: bool,
    tab_bar: TabBarState,
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

    surface_state: RefCell<HashMap<TabId, SurfaceUiState>>,
    leaf_state: RefCell<HashMap<PaneId, LeafUiState>>,
    semantic_zones: HashMap<PaneId, SemanticZoneCache>,

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
    system_metrics: crate::system_metrics::SystemMetricsHandle,
    metrics_tick_started: bool,
}

impl TermWindow {
    fn chatminal_sidebar_width_for_dimensions(pixel_width: usize, dpi: usize) -> usize {
        ChatminalSidebar::width_pixels(pixel_width, dpi)
    }

    pub(crate) fn chatminal_sidebar_width(&self) -> usize {
        Self::chatminal_sidebar_width_for_dimensions(
            self.dimensions.pixel_width,
            self.dimensions.dpi,
        )
    }

    fn chatminal_shell_enabled_for_dimensions(pixel_width: usize, dpi: usize) -> bool {
        Self::chatminal_sidebar_width_for_dimensions(pixel_width, dpi) > 0
    }

    fn chatminal_terminal_chrome_height_for_dimensions(_pixel_width: usize, _dpi: usize) -> usize {
        0
    }

    fn chatminal_terminal_footer_height_for_dimensions(pixel_width: usize, dpi: usize) -> usize {
        if Self::chatminal_shell_enabled_for_dimensions(pixel_width, dpi) {
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
        ) as f32
    }

    fn should_show_tab_bar_for_count(config: &ConfigHandle, num_tabs: usize) -> bool {
        if num_tabs <= 1 {
            config.enable_tab_bar && !config.hide_tab_bar_if_only_one_tab
        } else {
            config.enable_tab_bar
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
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }
        self.schedule_chatminal_sidebar_tick();
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

    fn activate_chatminal_session_index(&mut self, session_idx: isize) -> anyhow::Result<()> {
        let snapshot = self.chatminal_sidebar.snapshot();
        let max = snapshot.sessions.len();
        ensure!(max > 0, "no more sessions");

        let session_idx = if session_idx < 0 {
            max.saturating_sub(session_idx.unsigned_abs())
        } else {
            session_idx as usize
        };

        if let Some(session) = snapshot.sessions.get(session_idx) {
            self.switch_chatminal_session(&session.session_id);
        }
        Ok(())
    }

    fn activate_chatminal_session_relative(
        &mut self,
        delta: isize,
        wrap: bool,
    ) -> anyhow::Result<()> {
        let snapshot = self.chatminal_sidebar.snapshot();
        let max = snapshot.sessions.len();
        ensure!(max > 0, "no more sessions");

        let active = snapshot
            .active_session_id
            .as_deref()
            .and_then(|session_id| {
                snapshot
                    .sessions
                    .iter()
                    .position(|session| session.session_id == session_id)
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
        let lookup = chatminal_session_surface::collect_session_surface_lookup(
            self.window_id as DesktopWindowId,
        );
        let Some(session_id) = lookup.last_active_session_id else {
            return Ok(());
        };
        self.switch_chatminal_session(&session_id);
        Ok(())
    }

    fn switch_chatminal_session_target(
        &mut self,
        session_id: &str,
        preferred_surface_id: Option<SurfaceId>,
    ) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        if let Err(err) = self.chatminal_sidebar.activate_session(
            session_id,
            self.terminal_size.cols.max(20),
            self.terminal_size.rows.max(5),
        ) {
            log::error!("failed to activate sidebar session {session_id}: {err}");
            return;
        }

        self.focus_or_spawn_chatminal_session_surface(session_id, preferred_surface_id);
    }

    fn focus_or_spawn_chatminal_session_surface(
        &mut self,
        session_id: &str,
        preferred_surface_id: Option<SurfaceId>,
    ) {
        if let Some(surface_state) = preferred_surface_id
            .and_then(|surface_id| {
                chatminal_session_surface::focus_surface_state(
                    self.window_id as DesktopWindowId,
                    surface_id,
                )
            })
            .or_else(|| {
                chatminal_session_surface::focus_session_surface_state(
                    self.window_id as DesktopWindowId,
                    session_id,
                )
            })
        {
            if let Ok(client) = crate::chatminal_runtime::runtime_client() {
                if let Err(err) = client
                    .notify_session_surface_focused(session_id, surface_state.snapshot.surface_id)
                {
                    log::error!("failed to notify runtime bridge about surface focus: {err}");
                }
            }
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
            return;
        }

        let mux = Mux::get();
        let current_host_leaf_id = self
            .active_host_surface()
            .and_then(|tab| tab.get_active_pane().map(|pane| pane.pane_id()));
        let window_id = self.window_id;
        let size = self.terminal_size;
        let workspace = mux.active_workspace().clone();
        let session_id = session_id.to_string();
        let window = self.window.clone();

        chatminal_session_surface::spawn_session_surface(
            window_id as DesktopWindowId,
            session_id,
            size,
            current_host_leaf_id,
            workspace,
            window,
        );
    }

    fn active_host_surface(&self) -> Option<Arc<Tab>> {
        let mux = Mux::get();
        if self.chatminal_sidebar.is_enabled() {
            if let Some(session_id) = self.active_session_id() {
                if let Some(tab) = chatminal_session_surface::host_surface_for_session(
                    self.window_id as DesktopWindowId,
                    &session_id,
                ) {
                    return Some(tab);
                }
            }
        }
        mux.get_active_tab_for_window(self.window_id)
    }

    fn host_surface_for_public_surface(&self, surface_id: SurfaceId) -> Option<Arc<Tab>> {
        if self.chatminal_sidebar.is_enabled() {
            return chatminal_session_surface::host_surface_for_public_surface(
                self.window_id as DesktopWindowId,
                surface_id,
            );
        }
        self.host_surface_id_for_surface(surface_id)
            .and_then(|tab_id| Mux::get().get_tab(tab_id))
    }

    pub(crate) fn active_surface_id(&self) -> Option<SurfaceId> {
        if !self.chatminal_sidebar.is_enabled() {
            return None;
        }
        if let Some(surface_id) = self
            .active_host_leaf()
            .and_then(|pane| pane_metadata_surface_id(&*pane))
        {
            return Some(surface_id);
        }
        let session_id = self.active_session_id()?;
        chatminal_session_surface::surface_id_for_session(
            self.window_id as DesktopWindowId,
            &session_id,
        )
    }

    pub(crate) fn active_session_id(&self) -> Option<String> {
        if !self.chatminal_sidebar.is_enabled() {
            return None;
        }
        chatminal_session_surface::active_session_id(self.window_id as DesktopWindowId)
            .or_else(|| self.chatminal_sidebar.snapshot().active_session_id)
    }

    pub(crate) fn active_workspace_name(&self) -> String {
        Mux::get().active_workspace().to_string()
    }

    pub(crate) fn active_leaf_id(&self) -> Option<LeafId> {
        if self.chatminal_sidebar.is_enabled() {
            if let Some(session_id) = self.active_session_id() {
                if let Some(leaf_id) = chatminal_session_surface::active_leaf_id(
                    self.window_id as DesktopWindowId,
                    &session_id,
                ) {
                    return Some(leaf_id);
                }
            }
        }

        self.active_host_surface()
            .and_then(|tab| tab.get_active_pane())
            .map(|pane| LeafId::new(pane.pane_id() as u64))
    }

    pub(crate) fn active_public_leaf_id(&self) -> Option<u64> {
        if self.chatminal_sidebar.is_enabled() {
            if let Some(leaf_id) = self.active_leaf_id() {
                return Some(leaf_id.as_u64());
            }
        }

        self.active_host_surface()
            .and_then(|tab| tab.get_active_pane())
            .map(|pane| pane.pane_id() as u64)
    }

    fn leaf_id_for_pane(&self, pane: &Arc<dyn Pane>) -> LeafId {
        pane_metadata_leaf_id(&**pane).unwrap_or_else(|| LeafId::new(pane.pane_id() as u64))
    }

    fn focus_active_session_leaf(&self, pane: &Arc<dyn Pane>) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let Some(session_id) = self.active_session_id() else {
            return false;
        };
        chatminal_session_surface::focus_session_leaf(
            self.window_id as DesktopWindowId,
            &session_id,
            self.leaf_id_for_pane(pane),
        )
        .is_some()
    }

    fn swap_active_with_session_leaf(&self, pane: &Arc<dyn Pane>, keep_focus: bool) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let Some(session_id) = self.active_session_id() else {
            return false;
        };
        chatminal_session_surface::swap_active_with_session_leaf(
            self.window_id as DesktopWindowId,
            &session_id,
            self.leaf_id_for_pane(pane),
            keep_focus,
        )
    }

    fn resolve_public_leaf(&self, public_id: u64) -> Option<Arc<dyn Pane>> {
        if let Some(pane) = self
            .get_active_leaf_or_overlay()
            .filter(|pane| pane_matches_public_id(&**pane, public_id))
        {
            return Some(pane);
        }

        let mux = Mux::get();
        if let Ok(pane_id) = PaneId::try_from(public_id) {
            if let Some(pane) = mux.get_pane(pane_id) {
                return Some(pane);
            }
        }

        let window = mux.get_window(self.window_id)?;
        for tab in window.iter() {
            for pos in tab.iter_panes_ignoring_zoom() {
                if pane_matches_public_id(&*pos.pane, public_id) {
                    return Some(pos.pane.clone());
                }
            }
        }
        None
    }

    pub(crate) fn is_session_ui_mode(&self) -> bool {
        self.chatminal_sidebar.is_enabled()
    }

    fn host_surface_overlay(&self, tab_id: TabId) -> Option<Arc<dyn Pane>> {
        self.surface_ui_state(tab_id)
            .overlay
            .as_ref()
            .map(|overlay| overlay.pane.clone())
    }

    fn surface_overlay(&self, surface_id: SurfaceId) -> Option<Arc<dyn Pane>> {
        let tab_id = self.host_surface_id_for_surface(surface_id)?;
        self.host_surface_overlay(tab_id)
    }

    fn active_surface_overlay(&self) -> Option<Arc<dyn Pane>> {
        let surface_id = self.active_surface_id()?;
        self.surface_overlay(surface_id)
    }

    fn active_surface_has_overlay(&self) -> bool {
        self.active_surface_overlay().is_some()
    }

    fn host_surface_id_for_surface(&self, surface_id: SurfaceId) -> Option<TabId> {
        if self.chatminal_sidebar.is_enabled() {
            return chatminal_session_surface::host_surface_id_for_public_surface(
                self.window_id as DesktopWindowId,
                surface_id,
            );
        }
        usize::try_from(surface_id.as_u64()).ok()
    }

    fn assign_overlay_for_target_surface(
        &mut self,
        surface_id: Option<SurfaceId>,
        fallback_tab_id: TabId,
        overlay: Arc<dyn Pane>,
    ) {
        if let Some(surface_id) = surface_id {
            self.assign_overlay_for_surface(surface_id, overlay);
            return;
        }
        self.assign_overlay_for_host_surface(fallback_tab_id, overlay);
    }

    fn sync_active_chatminal_session_from_mux(&mut self) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        let lookup = chatminal_session_surface::collect_session_surface_lookup(
            self.window_id as DesktopWindowId,
        );
        let client = match crate::chatminal_runtime::runtime_client() {
            Ok(client) => client,
            Err(err) => {
                log::error!("failed to create runtime client for session reconcile: {err}");
                return;
            }
        };
        let action = match client.reconcile_session_surface_lookup(&lookup) {
            Ok(action) => action,
            Err(err) => {
                log::error!("failed to reconcile session surface lookup: {err}");
                return;
            }
        };
        let chatminal_session_runtime::SessionBridgeAction::FocusSurface { session_id } = action
        else {
            return;
        };
        self.focus_or_spawn_chatminal_session_surface(&session_id, None);
    }

    fn close_chatminal_session_for_tab(&mut self, tab: &Arc<Tab>) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let Some(session_id) = chatminal_session_surface::session_id_for_host_surface(
            self.window_id as DesktopWindowId,
            tab.tab_id(),
        ) else {
            return false;
        };
        self.close_chatminal_session_by_id(&session_id)
    }

    fn close_chatminal_session_by_id(&mut self, session_id: &str) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let surface_id = chatminal_session_surface::surface_id_for_session(
            self.window_id as DesktopWindowId,
            session_id,
        );
        if let Err(err) = self.chatminal_sidebar.close_session(&session_id) {
            log::error!("failed to close synced session {session_id}: {err}");
            return false;
        }
        chatminal_session_surface::remove_session_surface(
            self.window_id as DesktopWindowId,
            session_id,
        );
        if let (Some(surface_id), Ok(client)) =
            (surface_id, crate::chatminal_runtime::runtime_client())
        {
            let lookup_after_close = chatminal_session_surface::collect_session_surface_lookup(
                self.window_id as DesktopWindowId,
            );
            if let Err(err) =
                client.notify_session_surface_closed(session_id, surface_id, &lookup_after_close)
            {
                log::error!("failed to notify runtime bridge about surface close: {err}");
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
        self.sync_active_chatminal_session_from_mux();
        true
    }

    fn create_chatminal_session(&mut self) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        match self.chatminal_sidebar.create_session(
            self.terminal_size.cols.max(20),
            self.terminal_size.rows.max(5),
        ) {
            Ok(created) => {
                if let Some(window) = self.window.as_ref() {
                    window.invalidate();
                }
                self.switch_chatminal_session(&created.session_id);
            }
            Err(err) => {
                log::error!("failed to create sidebar session: {err}");
            }
        }
    }

    fn switch_chatminal_profile(&mut self, profile_id: &str) {
        if !self.chatminal_sidebar.is_enabled() {
            return;
        }
        if self
            .chatminal_sidebar
            .snapshot()
            .active_profile_id
            .as_deref()
            == Some(profile_id)
        {
            return;
        }

        match self.chatminal_sidebar.switch_profile(profile_id) {
            Ok(workspace) => {
                self.apply_chatminal_profile_workspace(workspace);
            }
            Err(err) => {
                log::error!("failed to switch sidebar profile {profile_id}: {err}");
            }
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

    fn close_requested(&mut self, window: &Window) {
        let mux = Mux::get();
        match self.config.window_close_confirmation {
            WindowCloseConfirmation::NeverPrompt => {
                // Immediately kill the tabs and allow the window to close
                mux.kill_window(self.window_id);
                window.close();
                front_end().forget_known_window(window);
            }
            WindowCloseConfirmation::AlwaysPrompt => {
                let tab = match self.active_host_surface() {
                    Some(tab) => tab,
                    None => {
                        mux.kill_window(self.window_id);
                        window.close();
                        front_end().forget_known_window(window);
                        return;
                    }
                };

                let engine_window_id = self.window_id;

                let can_close = mux
                    .get_window(engine_window_id)
                    .map_or(false, |w| w.can_close_without_prompting());
                if can_close {
                    mux.kill_window(self.window_id);
                    window.close();
                    front_end().forget_known_window(window);
                    return;
                }
                let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
                    confirm_close_window(term, engine_window_id as u64)
                });
                self.assign_overlay_for_target_surface(
                    self.active_surface_id(),
                    tab.tab_id(),
                    overlay,
                );
                promise::spawn::spawn(future).detach();

                // Don't close right now; let the close happen from
                // the confirmation overlay
            }
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

            for state in self.leaf_state.borrow_mut().values_mut() {
                state.mouse_terminal_coords.take();
            }
        }

        // Reset the cursor blink phase
        self.prev_cursor.bump();

        // force cursor to be repainted
        window.invalidate();

        if let Some(pane) = self.get_active_leaf_or_overlay() {
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
    pub async fn new_window(window_id: DesktopWindowId) -> anyhow::Result<()> {
        let engine_window_id = EngineWindowId::try_from(window_id)
            .context(format!("invalid desktop window id {window_id}"))?;
        let config = configuration();
        let chatminal_sidebar = ChatminalSidebar::from_env();
        let dpi = config.dpi.unwrap_or_else(|| ::window::default_dpi()) as usize;
        let fontconfig = Rc::new(FontConfiguration::new(Some(config.clone()), dpi)?);

        let mux = Mux::get();
        let size = match mux.get_active_tab_for_window(engine_window_id) {
            Some(tab) => tab.get_size(),
            None => {
                log::debug!("new_window has no tabs... yet?");
                Default::default()
            }
        };
        let physical_rows = size.rows as usize;
        let physical_cols = size.cols as usize;

        let render_metrics = RenderMetrics::new(&fontconfig)?;
        log::trace!("using render_metrics {:#?}", render_metrics);

        // Initially we have only a single tab, so take that into account
        // for the tab bar state.
        let show_tab_bar = Self::should_show_tab_bar_for_count(&config, 1);
        let tab_bar_height = if show_tab_bar {
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
            if let Some(window) = mux.get_window(engine_window_id) {
                for tab in window.iter() {
                    tab.resize(terminal_size);
                }
            };
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
            + Self::chatminal_terminal_footer_height_for_dimensions(terminal_size.pixel_width, dpi);

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
            "TermWindow::new_window called with window_id {} {:?} {:?}",
            window_id,
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
            chatminal_sidebar,
            system_metrics: crate::system_metrics::SystemMetricsHandle::start(),
            metrics_tick_started: false,
            config: config.clone(),
            config_overrides: engine_dynamic::Value::default(),
            palette: None,
            focused: None,
            window_id: engine_window_id,
            window_id_for_subscriptions: Arc::new(Mutex::new(engine_window_id)),
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
            show_tab_bar,
            show_scroll_bar: config.enable_scroll_bar,
            tab_bar: TabBarState::default(),
            fancy_tab_bar: None,
            right_status: String::new(),
            left_status: String::new(),
            last_mouse_coords: (0, -1),
            window_drag_position: None,
            current_mouse_event: None,
            current_modifier_and_leds: Default::default(),
            prev_cursor: PrevCursorPos::new(),
            last_scroll_info: RenderableDimensions::default(),
            surface_state: RefCell::new(HashMap::new()),
            leaf_state: RefCell::new(HashMap::new()),
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

        if let Some(position) = mux
            .get_window(engine_window_id)
            .and_then(|window| window.get_initial_position().clone())
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
            myself.subscribe_to_pane_updates();
            myself.emit_window_event("window-config-reloaded", None);
            myself.emit_status_event();
        }

        crate::update::start_update_checker();
        front_end().record_window_binding(window, window_id);

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
                if let Some(pane) = self.get_active_leaf_or_overlay() {
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
                let pane = match self.get_active_leaf_or_overlay() {
                    Some(pane) => pane,
                    None => return Ok(true),
                };
                pane.send_paste(text.as_str())?;
                Ok(true)
            }
            WindowEvent::DroppedUrl(urls) => {
                let pane = match self.get_active_leaf_or_overlay() {
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
                let pane = match self.get_active_leaf_or_overlay() {
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
                self.invalidate_modal();
                window.invalidate();
            }
            TermWindowNotif::PerformAssignmentForLeafId {
                leaf_id,
                assignment,
                tx,
            } => {
                let result = self
                    .resolve_public_leaf(leaf_id)
                    .ok_or_else(|| anyhow!("leaf id {} is not valid", leaf_id))
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
            TermWindowNotif::CancelOverlayForLeafId(leaf_id) => {
                let pane_id =
                    PaneId::try_from(leaf_id).map_err(|_| anyhow!("invalid leaf id {leaf_id}"))?;
                self.cancel_overlay_for_leaf(pane_id);
            }
            TermWindowNotif::CancelOverlayForSurfaceId {
                surface_id,
                pane_id,
            } => {
                let surface_id = SurfaceId::new(surface_id);
                let pane_id = pane_id
                    .map(|pane_id| {
                        PaneId::try_from(pane_id).map_err(|_| anyhow!("invalid pane id {pane_id}"))
                    })
                    .transpose()?;
                self.cancel_overlay_for_surface(surface_id, pane_id);
            }
            TermWindowNotif::CancelOverlayForHostSurfaceId {
                host_surface_id,
                pane_id,
            } => {
                let host_surface_id = TabId::try_from(host_surface_id)
                    .map_err(|_| anyhow!("invalid host surface id {host_surface_id}"))?;
                let pane_id = pane_id
                    .map(|pane_id| {
                        PaneId::try_from(pane_id).map_err(|_| anyhow!("invalid pane id {pane_id}"))
                    })
                    .transpose()?;
                self.cancel_overlay_for_host_surface(host_surface_id, pane_id);
            }
            TermWindowNotif::MuxNotification(n) => match n {
                MuxNotification::Alert {
                    alert: Alert::SetUserVar { name, value },
                    pane_id,
                } => {
                    self.emit_user_var_event(pane_id, name, value);
                }
                MuxNotification::WindowTitleChanged { .. }
                | MuxNotification::Alert {
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
                MuxNotification::Alert {
                    alert: Alert::PaletteChanged,
                    pane_id,
                } => {
                    // Shape cache includes color information, so
                    // ensure that we invalidate that as part of
                    // this overall invalidation for the palette
                    self.dispatch_notif(TermWindowNotif::InvalidateShapeCache, window)?;
                    self.handle_pane_output_event(pane_id);
                }
                MuxNotification::Alert {
                    alert: Alert::Bell,
                    pane_id,
                } => {
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

                    let mut per_pane = self.leaf_ui_state(pane_id);
                    per_pane.bell_start.replace(Instant::now());
                    window.invalidate();
                }
                MuxNotification::Alert {
                    alert: Alert::ToastNotification { .. },
                    ..
                } => {}
                MuxNotification::TabAddedToWindow {
                    window_id: _,
                    tab_id,
                } => {
                    let mux = Mux::get();
                    let mut size = self.terminal_size;
                    if let Some(tab) = mux.get_tab(tab_id) {
                        // If we attached to a remote domain and loaded in
                        // a tab async, we need to fixup its size, either
                        // by resizing it or resizes ourselves.
                        // The strategy here is to adjust both by taking
                        // the maximal size in both horizontal and vertical
                        // dimensions and applying that. In practice that
                        // means that a new local client will resize larger
                        // to adjust to the size of an existing client.
                        let tab_size = tab.get_size();
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
                            tab.resize(self.terminal_size);
                        }
                    }
                }
                MuxNotification::PaneOutput(pane_id) => {
                    self.handle_pane_output_event(pane_id);
                }
                MuxNotification::WindowInvalidated(_) => {
                    window.invalidate();
                    self.update_title_post_status();
                }
                MuxNotification::WindowRemoved(_window_id) => {
                    // Handled by frontend
                }
                MuxNotification::AssignClipboard { .. } => {
                    // Handled by frontend
                }
                MuxNotification::SaveToDownloads { .. } => {
                    // Handled by frontend
                }
                MuxNotification::PaneFocused(_) => {
                    // Also handled by clientpane
                    self.update_title_post_status();
                }
                MuxNotification::TabResized(_) => {
                    // Also handled by engine-client
                    self.update_title_post_status();
                }
                MuxNotification::TabTitleChanged { .. } => {
                    self.update_title_post_status();
                }
                MuxNotification::PaneAdded(_)
                | MuxNotification::WorkspaceRenamed { .. }
                | MuxNotification::PaneRemoved(_)
                | MuxNotification::WindowWorkspaceChanged(_)
                | MuxNotification::ActiveWorkspaceChanged(_)
                | MuxNotification::Empty
                | MuxNotification::WindowCreated(_) => {}
            },
            TermWindowNotif::EmitStatusUpdate => {
                self.emit_status_event();
            }
            TermWindowNotif::GetSelectionForLeafId { leaf_id, tx } => {
                let pane = self
                    .resolve_public_leaf(leaf_id)
                    .ok_or_else(|| anyhow!("leaf id {} is not valid", leaf_id))?;

                tx.try_send(self.selection_text(&pane))
                    .map_err(chan_err)
                    .context("send GetSelectionForLeafId response")?;
            }
            TermWindowNotif::GetSelectionEscapesForLeafId { leaf_id, tx } => {
                let result = self
                    .resolve_public_leaf(leaf_id)
                    .ok_or_else(|| anyhow!("leaf id {} is not valid", leaf_id))
                    .and_then(|pane| {
                        let lines = self.selection_lines(&pane);
                        lines_to_escapes(lines)
                    });

                tx.try_send(result)
                    .map_err(chan_err)
                    .context("send GetSelectionEscapesForLeafId response")?;
            }
            TermWindowNotif::Apply(func) => {
                func(self);
            }
            TermWindowNotif::SwitchToWindowId(window_id) => {
                let engine_window_id = EngineWindowId::try_from(window_id)
                    .context(format!("invalid desktop window id {window_id}"))?;
                self.window_id = engine_window_id;
                *self.window_id_for_subscriptions.lock().unwrap() = engine_window_id;

                self.clear_all_overlays();
                self.current_highlight.take();
                self.invalidate_fancy_tab_bar();
                self.invalidate_modal();

                let mux = Mux::get();
                if let Some(window) = mux.get_window(self.window_id) {
                    for tab in window.iter() {
                        tab.resize(self.terminal_size);
                    }
                };
                self.update_title();
                window.invalidate();
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
            .leaf_state
            .borrow()
            .iter()
            .filter_map(|(_, state)| state.overlay.as_ref().map(|overlay| overlay.pane.pane_id()))
            .collect::<Vec<_>>();

        for pane_id in overlay_panes_to_cancel {
            self.cancel_overlay_for_leaf(pane_id);
        }

        let tab_overlays_to_cancel = self
            .surface_state
            .borrow()
            .iter()
            .filter_map(|(tab_id, state)| state.overlay.as_ref().map(|_| *tab_id))
            .collect::<Vec<_>>();

        for tab_id in tab_overlays_to_cancel {
            self.cancel_overlay_for_host_surface(tab_id, None);
        }

        self.leaf_state.borrow_mut().clear();
        self.surface_state.borrow_mut().clear();
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

    fn is_pane_visible(&mut self, pane_id: PaneId) -> bool {
        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return false,
        };

        let tab_id = tab.tab_id();
        if let Some(tab_overlay) = self.host_surface_overlay(tab_id) {
            return tab_overlay.pane_id() == pane_id;
        }

        tab.contains_pane(pane_id)
    }

    fn handle_pane_output_event(&mut self, pane_id: PaneId) {
        metrics::histogram!("mux.pane_output_event.rate").record(1.);
        if self.is_pane_visible(pane_id) {
            if let Some(ref win) = self.window {
                win.invalidate();
            }
        }
    }

    fn handle_pane_output_event_callback(
        n: MuxNotification,
        window: &Window,
        window_id: EngineWindowId,
        dead: &Arc<AtomicBool>,
    ) -> bool {
        if dead.load(Ordering::Relaxed) {
            // Subscription cancelled asynchronously
            return false;
        }

        match n {
            MuxNotification::Alert {
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
            | MuxNotification::PaneFocused(pane_id)
            | MuxNotification::PaneRemoved(pane_id)
            | MuxNotification::PaneOutput(pane_id) => {
                // Ideally we'd check to see if pane_id is part of this window,
                // but overlays may not be 100% associated with the window
                // in the mux and we don't want to lose the invalidation
                // signal for that case, so we just check window validity
                // here and propagate to the window event handler that
                // will then do the check with full context.
                let mux = Mux::get();
                if mux.get_window(window_id).is_none() {
                    // Something inconsistent: cancel subscription
                    log::debug!(
                        "PaneOutput: wanted window_id={} from mux, but \
                         was not found, cancel mux subscription",
                        window_id
                    );
                    return false;
                }
                let _ = pane_id;
            }
            MuxNotification::PaneAdded(_pane_id) => {
                // If some other client spawns a pane inside this window, this
                // gives us an opportunity to attach it to the clipboard.
                let mux = Mux::get();
                return mux.get_window(window_id).is_some();
            }
            MuxNotification::TabAddedToWindow {
                window_id: notification_window_id,
                ..
            }
            | MuxNotification::WindowTitleChanged {
                window_id: notification_window_id,
                ..
            }
            | MuxNotification::WindowInvalidated(notification_window_id) => {
                if notification_window_id != window_id {
                    return true;
                }
            }
            MuxNotification::WindowRemoved(notification_window_id) => {
                if notification_window_id != window_id {
                    return true;
                }
                // Set the window as dead to unsubscribe from further notifications
                dead.store(true, Ordering::Relaxed);
                return false;
            }
            MuxNotification::TabResized(tab_id)
            | MuxNotification::TabTitleChanged { tab_id, .. } => {
                let mux = Mux::get();
                if mux.window_containing_tab(tab_id) == Some(window_id) {
                    // fall through
                } else {
                    return true;
                }
            }
            MuxNotification::Alert {
                alert: Alert::ToastNotification { .. },
                ..
            }
            | MuxNotification::AssignClipboard { .. }
            | MuxNotification::SaveToDownloads { .. }
            | MuxNotification::WindowCreated(_)
            | MuxNotification::ActiveWorkspaceChanged(_)
            | MuxNotification::WorkspaceRenamed { .. }
            | MuxNotification::Empty
            | MuxNotification::WindowWorkspaceChanged(_) => return true,
            MuxNotification::Alert {
                alert: Alert::PaletteChanged { .. },
                ..
            } => {
                // fall through
            }
        }

        window.notify(TermWindowNotif::MuxNotification(n));

        true
    }

    fn subscribe_to_pane_updates(&self) {
        let window = self.window.clone().expect("window to be valid on startup");
        let window_id = Arc::clone(&self.window_id_for_subscriptions);
        let mux = Mux::get();
        let dead = Arc::new(AtomicBool::new(false));
        mux.subscribe(move |n| {
            if dead.load(Ordering::Relaxed) {
                return false;
            }
            let window_id = *window_id.lock().unwrap();
            let window = window.clone();
            let dead = dead.clone();
            promise::spawn::spawn_into_main_thread(async move {
                Self::handle_pane_output_event_callback(n, &window, window_id, &dead)
            })
            .detach();
            true
        });
    }

    fn emit_status_event(&mut self) {
        self.emit_window_event("update-right-status", None);
        self.emit_window_event("update-status", None);
    }

    fn schedule_window_event(&mut self, name: &str, pane_id: Option<PaneId>) {
        let window = GuiWin::new(self);
        let pane = match pane_id {
            Some(pane_id) => Mux::get().get_pane(pane_id),
            None => None,
        };
        let pane = match pane {
            Some(pane) => pane,
            None => match self.get_active_leaf_or_overlay() {
                Some(pane) => pane,
                None => return,
            },
        };
        let pane_id = pane.pane_id() as u64;
        let name = name.to_string();

        async fn do_event(
            lua: Option<Rc<mlua::Lua>>,
            name: String,
            window: GuiWin,
            pane_id: u64,
        ) -> anyhow::Result<()> {
            let again = if let Some(lua) = lua {
                let args = lua.pack_multi((window.clone(), pane_id))?;

                if let Err(err) = config::lua::emit_event(&lua, (name.clone(), args)).await {
                    log::error!("while processing {} event: {:#}", name, err);
                }
                true
            } else {
                false
            };

            window
                .window
                .notify(TermWindowNotif::FinishWindowEvent { name, again });

            Ok(())
        }

        promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
            do_event(lua, name, window, pane_id)
        }))
        .detach();
    }

    /// Called as part of finishing up a callout to lua.
    /// If again==false it means that there isn't a lua config
    /// to execute against, so we should just mark as done.
    /// Otherwise, if there is a queued item, schedule it now.
    fn finish_window_event(&mut self, name: &str, again: bool) {
        let state = self
            .event_states
            .entry(name.to_string())
            .or_insert(EventState::None);
        if again {
            match state {
                EventState::InProgress => {
                    *state = EventState::None;
                }
                EventState::InProgressWithQueued(pane) => {
                    let pane = *pane;
                    *state = EventState::InProgress;
                    self.schedule_window_event(name, pane);
                }
                EventState::None => {}
            }
        } else {
            *state = EventState::None;
        }
    }

    pub fn emit_window_event(&mut self, name: &str, pane_id: Option<PaneId>) {
        if self.get_active_leaf_or_overlay().is_none() || self.window.is_none() {
            return;
        }

        let state = self
            .event_states
            .entry(name.to_string())
            .or_insert(EventState::None);
        match state {
            EventState::InProgress => {
                // Flag that we want to run again when the currently
                // executing event calls finish_window_event().
                *state = EventState::InProgressWithQueued(pane_id);
                return;
            }
            EventState::InProgressWithQueued(other_pane) => {
                // We've already got one copy executing and another
                // pending dispatch, so don't queue another.
                if pane_id != *other_pane {
                    log::warn!(
                        "Cannot queue {} event for pane {:?}, as \
                         there is already an event queued for pane {:?} \
                         in the same window",
                        name,
                        pane_id,
                        other_pane
                    );
                }
                return;
            }
            EventState::None => {
                // Nothing pending, so schedule a call now
                *state = EventState::InProgress;
                self.schedule_window_event(name, pane_id);
            }
        }
    }

    fn check_for_dirty_lines_and_invalidate_selection(&mut self, pane: &Arc<dyn Pane>) {
        let dims = pane.get_dimensions();
        let viewport = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top);
        let visible_range = viewport..viewport + dims.viewport_rows as StableRowIndex;
        let seqno = self.selection(pane.pane_id()).seqno;
        let dirty = pane.get_changed_since(visible_range, seqno);

        if dirty.is_empty() {
            return;
        }
        if pane.downcast_ref::<CopyOverlay>().is_none()
            && pane.downcast_ref::<QuickSelectOverlay>().is_none()
        {
            // If any of the changed lines intersect with the
            // selection, then we need to clear the selection, but not
            // when the search overlay is active; the search overlay
            // marks lines as dirty to force invalidate them for
            // highlighting purpose but also manipulates the selection
            // and we want to allow it to retain the selection it made!

            let clear_selection =
                if let Some(selection_range) = self.selection(pane.pane_id()).range.as_ref() {
                    let selection_rows = selection_range.rows();
                    selection_rows.into_iter().any(|row| dirty.contains(row))
                } else {
                    false
                };

            if clear_selection {
                self.selection(pane.pane_id()).range.take();
                self.selection(pane.pane_id()).origin.take();
                self.selection(pane.pane_id()).seqno = pane.get_current_seqno();
            }
        }
    }
}

impl TermWindow {
    fn palette(&mut self) -> &ColorPalette {
        if self.palette.is_none() {
            self.palette
                .replace(config::TermConfig::new().color_palette());
        }
        self.palette.as_ref().unwrap()
    }

    pub fn config_was_reloaded(&mut self) {
        log::debug!(
            "config was reloaded, overrides: {:?}",
            self.config_overrides
        );
        self.key_table_state.clear_stack();
        self.connection_name = Connection::get().unwrap().name();
        let config = match config::overridden_config(&self.config_overrides) {
            Ok(config) => config,
            Err(err) => {
                log::error!(
                    "Failed to apply config overrides to window: {:#}: {:?}",
                    err,
                    self.config_overrides
                );
                configuration()
            }
        };
        self.config = config.clone();
        self.palette.take();

        let mux = Mux::get();
        let window = match mux.get_window(self.window_id) {
            Some(window) => window,
            _ => return,
        };
        self.show_tab_bar = Self::should_show_tab_bar_for_count(&config, window.len());
        *self.cursor_blink_state.borrow_mut() = ColorEase::new(
            config.cursor_blink_rate,
            config.cursor_blink_ease_in,
            config.cursor_blink_rate,
            config.cursor_blink_ease_out,
            None,
        );
        *self.blink_state.borrow_mut() = ColorEase::new(
            config.text_blink_rate,
            config.text_blink_ease_in,
            config.text_blink_rate,
            config.text_blink_ease_out,
            None,
        );
        *self.rapid_blink_state.borrow_mut() = ColorEase::new(
            config.text_blink_rate_rapid,
            config.text_blink_rapid_ease_in,
            config.text_blink_rate_rapid,
            config.text_blink_rapid_ease_out,
            None,
        );

        self.show_scroll_bar = config.enable_scroll_bar;
        self.shape_generation += 1;
        {
            let mut shape_cache = self.shape_cache.borrow_mut();
            shape_cache.update_config(&config);
            shape_cache.clear();
        }
        self.line_state_cache.borrow_mut().update_config(&config);
        self.line_quad_cache.borrow_mut().update_config(&config);
        self.line_to_ele_shape_cache
            .borrow_mut()
            .update_config(&config);
        self.fancy_tab_bar.take();
        self.invalidate_fancy_tab_bar();
        self.invalidate_modal();
        self.input_map = InputMap::new(&config);
        self.leader_is_down = None;
        self.render_state.as_mut().map(|rs| rs.config_changed());
        let dimensions = self.dimensions;

        if let Err(err) = self.fonts.config_changed(&config) {
            log::error!("Failed to load font configuration: {:#}", err);
        }

        if let Some(window) = mux.get_window(self.window_id) {
            let term_config: Arc<dyn TerminalConfiguration> =
                Arc::new(TermConfig::with_config(config.clone()));
            for tab in window.iter() {
                for pane in tab.iter_panes_ignoring_zoom() {
                    pane.pane.set_config(Arc::clone(&term_config));
                }
            }
            for state in self.leaf_state.borrow().values() {
                if let Some(overlay) = &state.overlay {
                    overlay.pane.set_config(Arc::clone(&term_config));
                }
            }
            for state in self.surface_state.borrow().values() {
                if let Some(overlay) = &state.overlay {
                    overlay.pane.set_config(Arc::clone(&term_config));
                }
            }
        }

        if let Some(window) = self.window.as_ref().map(|w| w.clone()) {
            self.load_os_parameters();
            self.apply_scale_change(&dimensions, self.fonts.get_font_scale());
            self.apply_dimensions(&dimensions, None, &window);
            window.config_did_change(&config);
            window.invalidate();
        }

        // Do this after we've potentially adjusted scaling based on config/padding
        // and window size
        self.window_background = reload_background_image(
            &config,
            &self.window_background,
            &self.dimensions,
            &self.render_metrics,
        );

        self.invalidate_modal();
        self.emit_window_event("window-config-reloaded", None);
    }

    fn invalidate_modal(&mut self) {
        if let Some(modal) = self.get_modal() {
            modal.reconfigure(self);
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }
    }

    pub fn cancel_modal(&self) {
        self.modal.borrow_mut().take();
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn set_modal(&self, modal: Rc<dyn Modal>) {
        self.modal.borrow_mut().replace(modal);
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    fn get_modal(&self) -> Option<Rc<dyn Modal>> {
        self.modal.borrow().as_ref().map(|m| Rc::clone(&m))
    }

    fn update_scrollbar(&mut self) {
        if !self.show_scroll_bar {
            return;
        }

        let tab = match self.get_active_leaf_or_overlay() {
            Some(tab) => tab,
            None => return,
        };

        let render_dims = tab.get_dimensions();
        if render_dims == self.last_scroll_info {
            return;
        }

        self.last_scroll_info = render_dims;

        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    /// Called by various bits of code to update the title bar.
    /// Let's also trigger the status event so that it can choose
    /// to update the right-status.
    fn update_title(&mut self) {
        self.schedule_status_update();
        self.update_title_impl();
    }

    fn window_contains_pane(&mut self, pane_id: PaneId) -> bool {
        let mux = Mux::get();

        let (_domain, window_id, _tab_id) = match mux.resolve_pane_id(pane_id) {
            Some(tuple) => tuple,
            None => return false,
        };

        return window_id == self.window_id;
    }

    fn emit_user_var_event(&mut self, pane_id: PaneId, name: String, value: String) {
        if !self.window_contains_pane(pane_id) {
            return;
        }

        let mux = Mux::get();
        let window = GuiWin::new(self);
        let pane = match mux.get_pane(pane_id) {
            Some(pane) => pane.pane_id() as u64,
            None => return,
        };

        async fn do_event(
            lua: Option<Rc<mlua::Lua>>,
            name: String,
            value: String,
            window: GuiWin,
            pane_id: u64,
        ) -> anyhow::Result<()> {
            if let Some(lua) = lua {
                let args = lua.pack_multi((window.clone(), pane_id, name, value))?;
                if let Err(err) =
                    config::lua::emit_event(&lua, ("user-var-changed".to_string(), args)).await
                {
                    log::error!("while processing user-var-changed event: {:#}", err);
                }
            }

            window
                .window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    term_window.update_title();
                })));

            Ok(())
        }

        promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
            do_event(lua, name, value, window, pane)
        }))
        .detach();
    }

    /// Called by window:set_right_status after the status has
    /// been updated; let's update the bar
    pub fn update_title_post_status(&mut self) {
        self.update_title_impl();
    }

    fn update_title_impl(&mut self) {
        let mux = Mux::get();
        let window = match mux.get_window(self.window_id) {
            Some(window) => window,
            _ => return,
        };
        let surfaces = self.get_surface_information();
        let leaves = self.get_leaf_information();
        let active_surface = surfaces.iter().find(|surface| surface.is_active).cloned();
        let active_leaf = leaves.iter().find(|leaf| leaf.is_active).cloned();

        let border = self.get_os_border();
        let tab_bar_height = self.tab_bar_pixel_height().unwrap_or(0.);
        let tab_bar_y = if self.config.tab_bar_at_bottom {
            ((self.dimensions.pixel_height as f32) - (tab_bar_height + border.bottom.get() as f32))
                .max(0.)
        } else {
            border.top.get() as f32
        };

        let tab_bar_height = self.tab_bar_pixel_height().unwrap_or(0.);

        let tab_bar_x = self.terminal_tab_bar_left();
        let tab_bar_width = self.terminal_tab_bar_width();
        let hovering_in_tab_bar = match &self.current_mouse_event {
            Some(event) => {
                let mouse_x = event.coords.x as f32;
                let mouse_y = event.coords.y as f32;
                mouse_x >= tab_bar_x
                    && mouse_x < tab_bar_x + tab_bar_width
                    && mouse_y >= tab_bar_y as f32
                    && mouse_y < tab_bar_y as f32 + tab_bar_height
            }
            None => false,
        };

        let new_tab_bar = TabBarState::new(
            self.terminal_tab_bar_cols(),
            if hovering_in_tab_bar {
                self.current_mouse_event.as_ref().map(|event| {
                    ((event.coords.x as f32 - tab_bar_x).max(0.0)
                        / self.render_metrics.cell_size.width as f32)
                        .floor() as usize
                })
            } else {
                None
            },
            &surfaces,
            &leaves,
            self.config.resolved_palette.tab_bar.as_ref(),
            &self.config,
            &self.left_status,
            &self.right_status,
        );
        if new_tab_bar != self.tab_bar {
            self.tab_bar = new_tab_bar;
            self.invalidate_fancy_tab_bar();
            self.invalidate_modal();
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }

        let num_tabs = window.len();
        if num_tabs == 0 {
            return;
        }
        drop(window);

        let title = match config::run_immediate_with_lua_config(|lua| {
            if let Some(lua) = lua {
                let surfaces = lua.create_sequence_from(surfaces.clone().into_iter())?;
                let leaves = lua.create_sequence_from(leaves.clone().into_iter())?;

                let v = config::lua::emit_sync_callback(
                    &*lua,
                    (
                        "format-window-title".to_string(),
                        (
                            active_surface.clone(),
                            active_leaf.clone(),
                            surfaces,
                            leaves,
                            (*self.config).clone(),
                        ),
                    ),
                )?;
                match &v {
                    mlua::Value::Nil => Ok(None),
                    _ => Ok(Some(String::from_lua(v, &*lua)?)),
                }
            } else {
                Ok(None)
            }
        }) {
            Ok(s) => s,
            Err(err) => {
                log::warn!("format-window-title: {}", err);
                None
            }
        };

        let title = match title {
            Some(title) => title,
            None => {
                if let (Some(pos), Some(surface)) = (active_leaf, active_surface) {
                    if num_tabs == 1 {
                        format!("{}{}", if pos.is_zoomed { "[Z] " } else { "" }, pos.title)
                    } else {
                        format!(
                            "{}[{}/{}] {}",
                            if pos.is_zoomed { "[Z] " } else { "" },
                            surface.surface_index + 1,
                            num_tabs,
                            pos.title
                        )
                    }
                } else {
                    "".to_string()
                }
            }
        };

        if let Some(window) = self.window.as_ref() {
            window.set_title(&title);

            let show_tab_bar = Self::should_show_tab_bar_for_count(&self.config, num_tabs);

            // If the number of tabs changed and caused the tab bar to
            // hide/show, then we'll need to resize things.  It is simplest
            // to piggy back on the config reloading code for that, so that
            // is what we're doing.
            if show_tab_bar != self.show_tab_bar {
                self.config_was_reloaded();
            }
        }
        self.schedule_next_status_update();
    }

    fn schedule_next_status_update(&mut self) {
        if let Some(window) = self.window.as_ref() {
            let now = Instant::now();
            if self.last_status_call <= now {
                let interval = Duration::from_millis(self.config.status_update_interval);
                let target = now + interval;
                self.last_status_call = target;

                let window = window.clone();
                promise::spawn::spawn(async move {
                    Timer::at(target).await;
                    window.notify(TermWindowNotif::EmitStatusUpdate);
                })
                .detach();
            }
        }
    }

    fn update_text_cursor(&mut self, pos: &PositionedPane) {
        if let Some(win) = self.window.as_ref() {
            let cursor = pos.pane.get_cursor_position();
            let top = pos.pane.get_dimensions().physical_top;
            let tab_bar_height = if self.show_tab_bar && !self.config.tab_bar_at_bottom {
                self.tab_bar_pixel_height().unwrap()
            } else {
                0.0
            };
            let (padding_left, padding_top) = self.padding_left_top();

            let r = Rect::new(
                Point::new(
                    (((cursor.x + pos.left) as isize).max(0) * self.render_metrics.cell_size.width)
                        .add(padding_left as isize),
                    ((cursor.y + pos.top as isize - top).max(0)
                        * self.render_metrics.cell_size.height)
                        .add(tab_bar_height as isize)
                        .add(padding_top as isize),
                ),
                self.render_metrics.cell_size,
            );
            win.set_text_cursor_position(r);
        }
    }

    fn activate_window(&mut self, window_idx: usize) -> anyhow::Result<()> {
        let windows = front_end().gui_windows();
        if let Some(win) = windows.get(window_idx) {
            win.window.focus();
        }
        Ok(())
    }

    fn activate_window_relative(&mut self, delta: isize, wrap: bool) -> anyhow::Result<()> {
        let windows = front_end().gui_windows();
        let my_idx = windows
            .iter()
            .position(|w| Some(&w.window) == self.window.as_ref())
            .ok_or_else(|| anyhow!("I'm not in the window list!?"))?;

        let idx = my_idx as isize + delta;

        let idx = if wrap {
            let idx = if idx < 0 {
                windows.len() as isize + idx
            } else {
                idx
            };
            idx as usize % windows.len()
        } else {
            if idx < 0 {
                0
            } else if idx >= windows.len() as isize {
                windows.len().saturating_sub(1)
            } else {
                idx as usize
            }
        };

        if let Some(win) = windows.get(idx) {
            win.window.focus();
        }

        Ok(())
    }

    fn activate_surface_index(&mut self, surface_idx: isize) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return self.activate_chatminal_session_index(surface_idx);
        }
        let mux = Mux::get();
        let mut window = mux
            .get_window_mut(self.window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        // This logic is coupled with the CliSubCommand::ActivateTab
        // logic in the desktop entrypoint. If you update this, update that!
        let max = window.len();

        let surface_idx = if surface_idx < 0 {
            max.saturating_sub(surface_idx.abs() as usize)
        } else {
            surface_idx as usize
        };

        if surface_idx < max {
            window.save_and_then_set_active(surface_idx);

            drop(window);

            if let Some(tab) = self.get_active_leaf_or_overlay() {
                tab.focus_changed(true);
            }

            self.update_title();
            self.update_scrollbar();
            self.sync_active_chatminal_session_from_mux();
        }
        Ok(())
    }

    fn activate_surface_relative(&mut self, delta: isize, wrap: bool) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return self.activate_chatminal_session_relative(delta, wrap);
        }
        let mux = Mux::get();
        let window = mux
            .get_window(self.window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        let max = window.len();
        ensure!(max > 0, "no more tabs");

        // This logic is coupled with the CliSubCommand::ActivateTab
        // logic in the desktop entrypoint. If you update this, update that!
        let active = window.get_active_idx() as isize;
        let surface_idx = active + delta;
        let surface_idx = if wrap {
            let surface_idx = if surface_idx < 0 {
                max as isize + surface_idx
            } else {
                surface_idx
            };
            (surface_idx as usize % max) as isize
        } else {
            if surface_idx < 0 {
                0
            } else if surface_idx >= max as isize {
                max as isize - 1
            } else {
                surface_idx
            }
        };
        drop(window);
        self.activate_surface_index(surface_idx)
    }

    fn activate_last_surface(&mut self) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return self.activate_last_chatminal_session();
        }
        let mux = Mux::get();
        let window = mux
            .get_window(self.window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        let last_idx = window.get_last_active_idx();
        drop(window);
        match last_idx {
            Some(idx) => self.activate_surface_index(idx as isize),
            None => Ok(()),
        }
    }

    fn move_surface(&mut self, surface_idx: usize) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return Ok(());
        }
        let mux = Mux::get();
        let mut window = mux
            .get_window_mut(self.window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        let max = window.len();
        ensure!(max > 0, "no more tabs");

        let active = window.get_active_idx();

        ensure!(surface_idx < max, "cannot move a surface out of range");

        let surface = window.remove_by_idx(active);
        window.insert(surface_idx, &surface);
        window.set_active_without_saving(surface_idx);

        drop(window);
        self.update_title();
        self.update_scrollbar();

        Ok(())
    }

    fn show_input_selector(&mut self, args: &config::keyassignment::InputSelector) {
        let target_surface_id = self.active_surface_id();
        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return,
        };

        // Ignore any current overlay: we're going to cancel it out below
        // and we don't want this new one to reference that cancelled pane
        let pane = match self.get_active_leaf_no_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane_id = pane.pane_id() as u64;

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::selector::selector(term, args, gui_win, pane_id)
        });
        self.assign_overlay_for_target_surface(target_surface_id, tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    fn show_prompt_input_line(&mut self, args: &PromptInputLine) {
        let target_surface_id = self.active_surface_id();
        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return,
        };

        let pane = match self.get_active_leaf_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane_id = pane.pane_id() as u64;

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::prompt::show_line_prompt_overlay(term, args, gui_win, pane_id)
        });
        self.assign_overlay_for_target_surface(target_surface_id, tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    fn show_confirmation(&mut self, args: &Confirmation) {
        let target_surface_id = self.active_surface_id();
        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return,
        };

        let pane = match self.get_active_leaf_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane_id = pane.pane_id() as u64;

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::confirm::show_confirmation_overlay(term, args, gui_win, pane_id)
        });
        self.assign_overlay_for_target_surface(target_surface_id, tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    fn show_debug_overlay(&mut self) {
        let target_surface_id = self.active_surface_id();
        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return,
        };

        let gui_win = GuiWin::new(self);

        let opengl_info = self.opengl_info.as_deref().unwrap_or("Unknown").to_string();
        let connection_info = self.connection_name.clone();

        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
            crate::overlay::show_debug_overlay(term, gui_win, opengl_info, connection_info)
        });
        self.assign_overlay_for_target_surface(target_surface_id, tab.tab_id(), overlay);
        promise::spawn::spawn(future).detach();
    }

    fn show_surface_navigator(&mut self) {
        if self.is_session_ui_mode() {
            return;
        }
        let mux = Mux::get();
        let active_surface_idx = match mux.get_window(self.window_id) {
            Some(window) => window.get_active_idx(),
            None => return,
        };
        let title = "Tab Navigator".to_string();
        let args = LauncherActionArgs {
            title: Some(title),
            flags: LauncherFlags::TABS,
            help_text: None,
            fuzzy_help_text: None,
            alphabet: None,
        };
        self.show_launcher_impl(args, active_surface_idx);
    }

    fn show_launcher(&mut self) {
        let title = "Launcher".to_string();
        let args = LauncherActionArgs {
            title: Some(title),
            flags: LauncherFlags::LAUNCH_MENU_ITEMS
                | LauncherFlags::WORKSPACES
                | LauncherFlags::DOMAINS
                | LauncherFlags::KEY_ASSIGNMENTS
                | LauncherFlags::COMMANDS,
            help_text: None,
            fuzzy_help_text: None,
            alphabet: None,
        };
        self.show_launcher_impl(args, 0);
    }

    fn show_launcher_impl(&mut self, args: LauncherActionArgs, initial_choice_idx: usize) {
        let engine_window_id = self.window_id;
        let window = self.window.as_ref().unwrap().clone();
        let target_surface_id = self.active_surface_id();

        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return,
        };

        let pane = match self.get_active_leaf_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let domain_id_of_current_pane = tab
            .get_active_pane()
            .expect("tab has no panes!")
            .domain_id();
        let pane_id = pane.pane_id();
        let tab_id = tab.tab_id();
        let title = args.title.unwrap();
        let flags = args.flags;
        let help_text = args.help_text.unwrap_or(
            "Select an item and press Enter=launch  \
             Esc=cancel  /=filter"
                .to_string(),
        );
        let fuzzy_help_text = args
            .fuzzy_help_text
            .unwrap_or("Fuzzy matching: ".to_string());

        let config = &self.config;
        let alphabet = args.alphabet.unwrap_or(config.launcher_alphabet.clone());

        promise::spawn::spawn(async move {
            let args = LauncherArgs::new(
                &title,
                flags,
                engine_window_id,
                pane_id,
                domain_id_of_current_pane,
                &help_text,
                &fuzzy_help_text,
                &alphabet,
            )
            .await;

            let win = window.clone();
            win.notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                let tab = target_surface_id
                    .and_then(|surface_id| term_window.host_surface_for_public_surface(surface_id))
                    .or_else(|| Mux::get().get_tab(tab_id));
                if let Some(tab) = tab {
                    let window = window.clone();
                    let (overlay, future) =
                        start_overlay(term_window, &tab, move |_tab_id, term| {
                            launcher(args, term, window, initial_choice_idx)
                        });

                    term_window.assign_overlay_for_target_surface(
                        target_surface_id,
                        tab_id,
                        overlay,
                    );
                    promise::spawn::spawn(future).detach();
                }
            })));
        })
        .detach();
    }

    /// Returns the Prompt semantic zones
    fn get_semantic_prompt_zones(&mut self, pane: &Arc<dyn Pane>) -> &[StableRowIndex] {
        let cache = self
            .semantic_zones
            .entry(pane.pane_id())
            .or_insert_with(SemanticZoneCache::default);

        let seqno = pane.get_current_seqno();
        if cache.seqno != seqno {
            let zones = pane.get_semantic_zones().unwrap_or_else(|_| vec![]);
            let mut zones: Vec<StableRowIndex> = zones
                .into_iter()
                .filter_map(|zone| {
                    if zone.semantic_type == engine_term::SemanticType::Prompt {
                        Some(zone.start_y)
                    } else {
                        None
                    }
                })
                .collect();
            // dedup to avoid issues where both left and right prompts are
            // defined: we only care if there were 1+ prompts on a line,
            // not about how many prompts are on a line.
            // upstream issue #1121
            zones.dedup();
            cache.zones = zones;
            cache.seqno = seqno;
        }
        &cache.zones
    }

    fn scroll_to_prompt(&mut self, amount: isize, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top);
        let zone = {
            let zones = self.get_semantic_prompt_zones(&pane);
            let idx = match zones.binary_search(&position) {
                Ok(idx) | Err(idx) => idx,
            };
            let idx = ((idx as isize) + amount).max(0) as usize;
            zones.get(idx).cloned()
        };
        if let Some(zone) = zone {
            self.set_viewport(pane.pane_id(), Some(zone), dims);
        }

        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn scroll_by_page(&mut self, amount: f64, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top) as f64
            + (amount * dims.viewport_rows as f64);
        self.set_viewport(pane.pane_id(), Some(position as isize), dims);
        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn scroll_by_current_event_wheel_delta(&mut self, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        if let Some(event) = &self.current_mouse_event {
            let amount = match event.kind {
                MouseEventKind::VertWheel(amount) => -amount,
                _ => return Ok(()),
            };
            self.scroll_by_line(amount.into(), pane)?;
        }
        Ok(())
    }

    fn scroll_by_line(&mut self, amount: isize, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id())
            .unwrap_or(dims.physical_top)
            .saturating_add(amount);
        self.set_viewport(pane.pane_id(), Some(position), dims);
        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn move_surface_relative(&mut self, delta: isize) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return Ok(());
        }
        let mux = Mux::get();
        let window = mux
            .get_window(self.window_id)
            .ok_or_else(|| anyhow!("no such window"))?;

        let max = window.len();
        ensure!(max > 0, "no more tabs");

        let active = window.get_active_idx();
        let surface_idx = active as isize + delta;
        let surface_idx = if surface_idx < 0 {
            0usize
        } else if surface_idx >= max as isize {
            max - 1
        } else {
            surface_idx as usize
        };

        drop(window);
        self.move_surface(surface_idx)
    }

    pub fn perform_key_assignment(
        &mut self,
        pane: &Arc<dyn Pane>,
        assignment: &KeyAssignment,
    ) -> anyhow::Result<PerformAssignmentResult> {
        use KeyAssignment::*;

        if let Some(modal) = self.get_modal() {
            if modal.perform_assignment(assignment, self) {
                return Ok(PerformAssignmentResult::Handled);
            }
        }

        match pane.perform_assignment(assignment) {
            PerformAssignmentResult::Unhandled => {}
            result => return Ok(result),
        }

        let window = self.window.as_ref().map(|w| w.clone());

        match assignment {
            ActivateKeyTable {
                name,
                timeout_milliseconds,
                replace_current,
                one_shot,
                until_unknown,
                prevent_fallback,
            } => {
                anyhow::ensure!(
                    self.input_map.has_table(name),
                    "ActivateKeyTable: no key_table named {}",
                    name
                );
                self.key_table_state.activate(KeyTableArgs {
                    name,
                    timeout_milliseconds: *timeout_milliseconds,
                    replace_current: *replace_current,
                    one_shot: *one_shot,
                    until_unknown: *until_unknown,
                    prevent_fallback: *prevent_fallback,
                });
                self.update_title();
            }
            PopKeyTable => {
                self.key_table_state.pop();
                self.update_title();
            }
            ClearKeyTableStack => {
                self.key_table_state.clear_stack();
                self.update_title();
            }
            Multiple(actions) => {
                for a in actions {
                    self.perform_key_assignment(pane, a)?;
                }
            }
            SpawnTab(spawn_where) => {
                self.spawn_surface(spawn_where);
            }
            SpawnWindow => {
                self.spawn_command(&SpawnCommand::default(), SpawnWhere::NewWindow);
            }
            SpawnCommandInNewTab(spawn) => {
                self.spawn_command(spawn, SpawnWhere::NewTab);
            }
            SpawnCommandInNewWindow(spawn) => {
                self.spawn_command(spawn, SpawnWhere::NewWindow);
            }
            SplitHorizontal(spawn) => {
                log::trace!("SplitHorizontal {:?}", spawn);
                self.spawn_command(
                    spawn,
                    SpawnWhere::SplitPane(SplitRequest {
                        direction: SplitDirection::Horizontal,
                        target_is_second: true,
                        size: MuxSplitSize::Percent(50),
                        top_level: false,
                    }),
                );
            }
            SplitVertical(spawn) => {
                log::trace!("SplitVertical {:?}", spawn);
                self.spawn_command(
                    spawn,
                    SpawnWhere::SplitPane(SplitRequest {
                        direction: SplitDirection::Vertical,
                        target_is_second: true,
                        size: MuxSplitSize::Percent(50),
                        top_level: false,
                    }),
                );
            }
            ToggleFullScreen => {
                self.window.as_ref().unwrap().toggle_fullscreen();
            }
            ToggleAlwaysOnTop => {
                let window = self.window.clone().unwrap();
                let current_level = self.window_state.as_window_level();

                match current_level {
                    WindowLevel::AlwaysOnTop => {
                        window.set_window_level(WindowLevel::Normal);
                    }
                    WindowLevel::AlwaysOnBottom | WindowLevel::Normal => {
                        window.set_window_level(WindowLevel::AlwaysOnTop);
                    }
                }
            }
            ToggleAlwaysOnBottom => {
                let window = self.window.clone().unwrap();
                let current_level = self.window_state.as_window_level();

                match current_level {
                    WindowLevel::AlwaysOnBottom => {
                        window.set_window_level(WindowLevel::Normal);
                    }
                    WindowLevel::AlwaysOnTop | WindowLevel::Normal => {
                        window.set_window_level(WindowLevel::AlwaysOnBottom);
                    }
                }
            }
            SetWindowLevel(level) => {
                let window = self.window.clone().unwrap();
                window.set_window_level(level.clone());
            }
            CopyTo(dest) => {
                let text = self.selection_text(pane);
                self.copy_to_clipboard(*dest, text);
            }
            CopyTextTo { text, destination } => {
                self.copy_to_clipboard(*destination, text.clone());
            }
            PasteFrom(source) => {
                self.paste_from_clipboard(pane, *source);
            }
            ActivateTabRelative(n) => {
                self.activate_surface_relative(*n, true)?;
            }
            ActivateTabRelativeNoWrap(n) => {
                self.activate_surface_relative(*n, false)?;
            }
            ActivateLastTab => self.activate_last_surface()?,
            DecreaseFontSize => self.decrease_font_size(),
            IncreaseFontSize => self.increase_font_size(),
            ResetFontSize => self.reset_font_size(),
            ResetFontAndWindowSize => {
                if let Some(w) = window.as_ref() {
                    self.reset_font_and_window_size(&w)?
                }
            }
            ActivateTab(n) => {
                self.activate_surface_index(*n)?;
            }
            ActivateWindow(n) => {
                self.activate_window(*n)?;
            }
            ActivateWindowRelative(n) => {
                self.activate_window_relative(*n, true)?;
            }
            ActivateWindowRelativeNoWrap(n) => {
                self.activate_window_relative(*n, false)?;
            }
            SendString(s) => pane.writer().write_all(s.as_bytes())?,
            SendKey(key) => {
                use keyevent::Key;
                let mods = key.mods;
                if let Key::Code(key) = self.win_key_code_to_termwiz_key_code(
                    &key.key.resolve(self.config.key_map_preference),
                ) {
                    pane.key_down(key, mods)?;
                }
            }
            Hide => {
                if let Some(w) = window.as_ref() {
                    w.hide();
                }
            }
            Show => {
                if let Some(w) = window.as_ref() {
                    w.show();
                }
            }
            CloseCurrentTab { confirm } => self.close_current_surface(*confirm),
            CloseCurrentPane { confirm } => self.close_current_pane(*confirm),
            Nop | DisableDefaultAssignment => {}
            ReloadConfiguration => config::reload(),
            MoveTab(n) => self.move_surface(*n)?,
            MoveTabRelative(n) => self.move_surface_relative(*n)?,
            ScrollByPage(n) => self.scroll_by_page(**n, pane)?,
            ScrollByLine(n) => self.scroll_by_line(*n, pane)?,
            ScrollByCurrentEventWheelDelta => self.scroll_by_current_event_wheel_delta(pane)?,
            ScrollToPrompt(n) => self.scroll_to_prompt(*n, pane)?,
            ScrollToTop => self.scroll_to_top(pane),
            ScrollToBottom => self.scroll_to_bottom(pane),
            ShowTabNavigator => self.show_surface_navigator(),
            ShowDebugOverlay => self.show_debug_overlay(),
            ShowLauncher => self.show_launcher(),
            ShowLauncherArgs(args) => {
                let title = args.title.clone().unwrap_or("Launcher".to_string());
                let args = LauncherActionArgs {
                    title: Some(title),
                    flags: args.flags,
                    help_text: args.help_text.clone(),
                    fuzzy_help_text: args.fuzzy_help_text.clone(),
                    alphabet: args.alphabet.clone(),
                };
                self.show_launcher_impl(args, 0);
            }
            HideApplication => {
                let con = Connection::get().expect("call on gui thread");
                con.hide_application();
            }
            QuitApplication => {
                let config = &self.config;
                log::info!("QuitApplication over here (window)");

                match config.window_close_confirmation {
                    WindowCloseConfirmation::NeverPrompt => {
                        let con = Connection::get().expect("call on gui thread");
                        con.terminate_message_loop();
                    }
                    WindowCloseConfirmation::AlwaysPrompt => {
                        let target_surface_id = self.active_surface_id();
                        let tab = match self.active_host_surface() {
                            Some(tab) => tab,
                            None => anyhow::bail!("no active tab!?"),
                        };

                        let (overlay, future) = start_overlay(self, &tab, move |_tab_id, term| {
                            confirm_quit_program(term)
                        });
                        self.assign_overlay_for_target_surface(
                            target_surface_id,
                            tab.tab_id(),
                            overlay,
                        );
                        promise::spawn::spawn(future).detach();
                    }
                }
            }
            SelectTextAtMouseCursor(mode) => self.select_text_at_mouse_cursor(*mode, pane),
            ExtendSelectionToMouseCursor(mode) => {
                self.extend_selection_at_mouse_cursor(*mode, pane)
            }
            ClearSelection => {
                self.clear_selection(pane);
            }
            StartWindowDrag => {
                self.window_drag_position = self.current_mouse_event.clone();
            }
            OpenLinkAtMouseCursor => {
                self.do_open_link_at_mouse_cursor(pane);
            }
            EmitEvent(name) => {
                self.emit_window_event(name, None);
            }
            CompleteSelectionOrOpenLinkAtMouseCursor(dest) => {
                let text = self.selection_text(pane);
                if !text.is_empty() {
                    self.copy_to_clipboard(*dest, text);
                    let window = self.window.as_ref().unwrap();
                    window.invalidate();
                } else {
                    self.do_open_link_at_mouse_cursor(pane);
                }
            }
            CompleteSelection(dest) => {
                let text = self.selection_text(pane);
                if !text.is_empty() {
                    self.copy_to_clipboard(*dest, text);
                    let window = self.window.as_ref().unwrap();
                    window.invalidate();
                }
            }
            ClearScrollback(erase_mode) => {
                pane.erase_scrollback(*erase_mode);
                let window = self.window.as_ref().unwrap();
                window.invalidate();
            }
            Search(pattern) => {
                if let Some(pane) = self.get_active_leaf_or_overlay() {
                    let mut replace_current = false;
                    if let Some(existing) = pane.downcast_ref::<CopyOverlay>() {
                        let mut params = existing.get_params();
                        params.editing_search = true;
                        if !pattern.is_empty() {
                            params.pattern = self.resolve_search_pattern(pattern.clone(), &pane);
                        }
                        existing.apply_params(params);
                        replace_current = true;
                    } else {
                        let search = CopyOverlay::with_pane(
                            self,
                            &pane,
                            CopyModeParams {
                                pattern: self.resolve_search_pattern(pattern.clone(), &pane),
                                editing_search: true,
                            },
                        )?;
                        self.assign_overlay_for_leaf(pane.pane_id(), search);
                    }
                    self.leaf_ui_state(pane.pane_id())
                        .overlay
                        .as_mut()
                        .map(|overlay| {
                            overlay.key_table_state.activate(KeyTableArgs {
                                name: "search_mode",
                                timeout_milliseconds: None,
                                replace_current,
                                one_shot: false,
                                until_unknown: false,
                                prevent_fallback: false,
                            });
                        });
                }
            }
            QuickSelect => {
                if let Some(pane) = self.get_active_leaf_no_overlay() {
                    let qa = QuickSelectOverlay::with_pane(
                        self,
                        &pane,
                        &QuickSelectArguments::default(),
                    );
                    self.assign_overlay_for_leaf(pane.pane_id(), qa);
                }
            }
            QuickSelectArgs(args) => {
                if let Some(pane) = self.get_active_leaf_no_overlay() {
                    let qa = QuickSelectOverlay::with_pane(self, &pane, args);
                    self.assign_overlay_for_leaf(pane.pane_id(), qa);
                }
            }
            ActivateCopyMode => {
                if let Some(pane) = self.get_active_leaf_or_overlay() {
                    let mut replace_current = false;
                    if let Some(existing) = pane.downcast_ref::<CopyOverlay>() {
                        let mut params = existing.get_params();
                        params.editing_search = false;
                        existing.apply_params(params);
                        replace_current = true;
                    } else {
                        let copy = CopyOverlay::with_pane(
                            self,
                            &pane,
                            CopyModeParams {
                                pattern: MuxPattern::default(),
                                editing_search: false,
                            },
                        )?;
                        self.assign_overlay_for_leaf(pane.pane_id(), copy);
                    }
                    self.leaf_ui_state(pane.pane_id())
                        .overlay
                        .as_mut()
                        .map(|overlay| {
                            overlay.key_table_state.activate(KeyTableArgs {
                                name: "copy_mode",
                                timeout_milliseconds: None,
                                replace_current,
                                one_shot: false,
                                until_unknown: false,
                                prevent_fallback: false,
                            });
                        });
                }
            }
            AdjustPaneSize(direction, amount) => {
                let tab = match self.active_host_surface() {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };

                if !self.active_surface_has_overlay() {
                    tab.adjust_pane_size(*direction, *amount);
                }
            }
            ActivatePaneByIndex(index) => {
                let tab = match self.active_host_surface() {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };

                if !self.active_surface_has_overlay() {
                    let panes = tab.iter_panes();
                    let focused_leaf = self.chatminal_sidebar.is_enabled()
                        && panes
                            .iter()
                            .find(|p| p.index == *index)
                            .map(|pos| self.focus_active_session_leaf(&pos.pane))
                            .is_some();
                    if !focused_leaf && panes.iter().position(|p| p.index == *index).is_some() {
                        tab.set_active_idx(*index);
                    }
                }
            }
            ActivatePaneDirection(direction) => {
                let tab = match self.active_host_surface() {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };

                if !self.active_surface_has_overlay() {
                    let active_session_id = self.active_session_id();
                    let focused_leaf = self.chatminal_sidebar.is_enabled()
                        && active_session_id
                            .and_then(|session_id| {
                                chatminal_session_surface::activate_session_leaf_direction(
                                    self.window_id as DesktopWindowId,
                                    &session_id,
                                    *direction,
                                )
                            })
                            .is_some();
                    if !focused_leaf {
                        tab.activate_pane_direction(*direction);
                    }
                }
            }
            TogglePaneZoomState => {
                let tab = match self.active_host_surface() {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };
                tab.toggle_zoom();
            }
            SetPaneZoomState(zoomed) => {
                let tab = match self.active_host_surface() {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };
                tab.set_zoomed(*zoomed);
            }
            SwitchWorkspaceRelative(delta) => {
                let mux = Mux::get();
                let workspace = mux.active_workspace();
                let workspaces = mux.iter_workspaces();
                let idx = workspaces.iter().position(|w| *w == workspace).unwrap_or(0);
                let new_idx = idx as isize + delta;
                let new_idx = if new_idx < 0 {
                    workspaces.len() as isize + new_idx
                } else {
                    new_idx
                };
                let new_idx = new_idx as usize % workspaces.len();
                if let Some(w) = workspaces.get(new_idx) {
                    front_end().switch_workspace(w);
                }
            }
            SwitchToWorkspace { name, spawn } => {
                let activity = crate::Activity::new();
                let mux = Mux::get();
                let name = name
                    .as_ref()
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| mux.generate_workspace_name());
                let switcher = crate::frontend::WorkspaceSwitcher::new(&name);
                mux.set_active_workspace(&name);

                if mux.iter_windows_in_workspace(&name).is_empty() {
                    let spawn = spawn.as_ref().map(|s| s.clone()).unwrap_or_default();
                    let size = self.terminal_size;
                    let term_config = Arc::new(TermConfig::with_config(self.config.clone()));
                    let src_window_id = self.window_id;

                    promise::spawn::spawn(async move {
                        if let Err(err) = crate::spawn::spawn_command_internal(
                            spawn,
                            SpawnWhere::NewWindow,
                            size,
                            Some(src_window_id as DesktopWindowId),
                            term_config,
                        )
                        .await
                        {
                            log::error!("Failed to spawn: {:#}", err);
                        }
                        switcher.do_switch();
                        drop(activity);
                    })
                    .detach();
                } else {
                    switcher.do_switch();
                }
            }
            DetachDomain(domain) => {
                let domain = Mux::get().resolve_spawn_tab_domain(Some(pane.pane_id()), domain)?;
                domain.detach()?;
            }
            AttachDomain(domain) => {
                let window = self.window_id;
                let domain = domain.to_string();
                let dpi = self.dimensions.dpi as u32;

                promise::spawn::spawn(async move {
                    let mux = Mux::get();
                    let domain = mux
                        .get_domain_by_name(&domain)
                        .ok_or_else(|| anyhow!("{} is not a valid domain name", domain))?;
                    domain.attach(Some(window)).await?;

                    let have_panes_in_domain = mux
                        .iter_panes()
                        .iter()
                        .any(|p| p.domain_id() == domain.domain_id());

                    if !have_panes_in_domain {
                        let config = config::configuration();
                        let _tab = domain
                            .spawn(
                                config.initial_size(
                                    dpi,
                                    Some(crate::cell_pixel_dims(&config, dpi as f64)?),
                                ),
                                None,
                                None,
                                window,
                            )
                            .await?;
                    }

                    Result::<(), anyhow::Error>::Ok(())
                })
                .detach();
            }
            CopyMode(_) => {
                // NOP here; handled by the overlay directly
            }
            RotatePanes(direction) => {
                let tab = match self.active_host_surface() {
                    Some(tab) => tab,
                    None => return Ok(PerformAssignmentResult::Handled),
                };
                match direction {
                    RotationDirection::Clockwise => tab.rotate_clockwise(),
                    RotationDirection::CounterClockwise => tab.rotate_counter_clockwise(),
                }
            }
            SplitPane(split) => {
                log::trace!("SplitPane {:?}", split);
                self.spawn_command(
                    &split.command,
                    SpawnWhere::SplitPane(SplitRequest {
                        direction: match split.direction {
                            PaneDirection::Down | PaneDirection::Up => SplitDirection::Vertical,
                            PaneDirection::Left | PaneDirection::Right => {
                                SplitDirection::Horizontal
                            }
                            PaneDirection::Next | PaneDirection::Prev => {
                                log::error!(
                                    "Invalid direction {:?} for SplitPane",
                                    split.direction
                                );
                                return Ok(PerformAssignmentResult::Handled);
                            }
                        },
                        target_is_second: match split.direction {
                            PaneDirection::Down | PaneDirection::Right => true,
                            PaneDirection::Up | PaneDirection::Left => false,
                            PaneDirection::Next | PaneDirection::Prev => unreachable!(),
                        },
                        size: match split.size {
                            SplitSize::Percent(n) => MuxSplitSize::Percent(n),
                            SplitSize::Cells(n) => MuxSplitSize::Cells(n),
                        },
                        top_level: split.top_level,
                    }),
                );
            }
            PaneSelect(args) => {
                let modal = crate::termwindow::paneselect::PaneSelector::new(self, args);
                self.set_modal(Rc::new(modal));
            }
            CharSelect(args) => {
                let modal = crate::termwindow::charselect::CharSelector::new(self, args);
                self.set_modal(Rc::new(modal));
            }
            ResetTerminal => {
                pane.perform_actions(vec![termwiz::escape::Action::Esc(
                    termwiz::escape::Esc::Code(termwiz::escape::EscCode::FullReset),
                )]);
            }
            OpenUri(link) => {
                engine_open_url::open_url(link);
            }
            ActivateCommandPalette => {
                let modal = crate::termwindow::palette::CommandPalette::new(self);
                self.set_modal(Rc::new(modal));
            }
            PromptInputLine(args) => self.show_prompt_input_line(args),
            InputSelector(args) => self.show_input_selector(args),
            Confirmation(args) => self.show_confirmation(args),
        };
        Ok(PerformAssignmentResult::Handled)
    }

    fn do_open_link_at_mouse_cursor(&self, pane: &Arc<dyn Pane>) {
        // They clicked on a link, so let's open it!
        // We need to ensure that we spawn the `open` call outside of the context
        // of our window loop; on Windows it can cause a panic due to
        // triggering our WndProc recursively.
        // We get that assurance for free as part of the async dispatch that we
        // perform below; here we allow the user to define an `open-uri` event
        // handler that can bypass the normal `open_url` functionality.
        if let Some(link) = self.current_highlight.as_ref().cloned() {
            let window = GuiWin::new(self);
            let pane_id = pane.pane_id() as u64;

            async fn open_uri(
                lua: Option<Rc<mlua::Lua>>,
                window: GuiWin,
                pane_id: u64,
                link: String,
            ) -> anyhow::Result<()> {
                let default_click = match lua {
                    Some(lua) => {
                        let args = lua.pack_multi((window, pane_id, link.clone()))?;
                        config::lua::emit_event(&lua, ("open-uri".to_string(), args))
                            .await
                            .map_err(|e| {
                                log::error!("while processing open-uri event: {:#}", e);
                                e
                            })?
                    }
                    None => true,
                };
                if default_click {
                    log::info!("clicking {}", link);
                    engine_open_url::open_url(&link);
                }
                Ok(())
            }

            promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
                open_uri(lua, window, pane_id, link.uri().to_string())
            }))
            .detach();
        }
    }
    fn close_current_pane(&mut self, confirm: bool) {
        let mux = Mux::get();
        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return,
        };
        let pane = match tab.get_active_pane() {
            Some(p) => p,
            None => return,
        };

        let pane_id = pane.pane_id();
        if confirm && !pane.can_close_without_prompting(CloseReason::Pane) {
            let window = self.window.clone().unwrap();
            let (overlay, future) = start_overlay_pane(self, &pane, move |pane_id, term| {
                confirm_close_pane(pane_id, term, window)
            });
            self.assign_overlay_for_leaf(pane_id, overlay);
            promise::spawn::spawn(future).detach();
        } else {
            mux.remove_pane(pane_id);
        }
    }

    fn close_specific_surface(&mut self, surface_idx: usize, confirm: bool) {
        let mux = Mux::get();
        let engine_window_id = self.window_id;
        let window = match mux.get_window(engine_window_id) {
            Some(w) => w,
            None => return,
        };

        let host_surface = match window.get_by_idx(surface_idx) {
            Some(surface) => Arc::clone(surface),
            None => return,
        };
        drop(window);

        let host_surface_id = host_surface.tab_id();
        if self.close_chatminal_session_for_tab(&host_surface) {
            return;
        }
        if confirm && !host_surface.can_close_without_prompting(CloseReason::Tab) {
            if self.activate_surface_index(surface_idx as isize).is_err() {
                return;
            }

            let (overlay, future) = start_overlay(self, &host_surface, move |tab_id, term| {
                confirm_close_tab(tab_id, term)
            });
            self.assign_overlay_for_host_surface(host_surface_id, overlay);
            promise::spawn::spawn(future).detach();
        } else {
            mux.remove_tab(host_surface_id);
            self.sync_active_chatminal_session_from_mux();
        }
    }

    fn close_current_surface(&mut self, confirm: bool) {
        let mux = Mux::get();
        let host_surface = match self.active_host_surface() {
            Some(surface) => surface,
            None => return,
        };
        let host_surface_id = host_surface.tab_id();
        if self.close_chatminal_session_for_tab(&host_surface) {
            return;
        }
        if confirm && !host_surface.can_close_without_prompting(CloseReason::Tab) {
            let target_surface_id = self.active_surface_id();
            let (overlay, future) = start_overlay(self, &host_surface, move |tab_id, term| {
                confirm_close_tab(tab_id, term)
            });
            self.assign_overlay_for_target_surface(target_surface_id, host_surface_id, overlay);
            promise::spawn::spawn(future).detach();
        } else {
            mux.remove_tab(host_surface_id);
            self.sync_active_chatminal_session_from_mux();
        }
    }

    pub fn leaf_ui_state(&self, pane_id: PaneId) -> RefMut<'_, LeafUiState> {
        RefMut::map(self.leaf_state.borrow_mut(), |state| {
            state.entry(pane_id).or_insert_with(LeafUiState::default)
        })
    }

    pub fn surface_ui_state(&self, tab_id: TabId) -> RefMut<'_, SurfaceUiState> {
        RefMut::map(self.surface_state.borrow_mut(), |state| {
            state.entry(tab_id).or_insert_with(SurfaceUiState::default)
        })
    }

    /// Resize overlays to match their corresponding tab/pane dimensions
    pub fn resize_overlays(&self) {
        let mux = Mux::get();
        for (_, state) in self.surface_state.borrow().iter() {
            if let Some(overlay) = state.overlay.as_ref().map(|o| &o.pane) {
                overlay.resize(self.terminal_size).ok();
            }
        }
        for (pane_id, state) in self.leaf_state.borrow().iter() {
            if let Some(overlay) = state.overlay.as_ref().map(|o| &o.pane) {
                if let Some(pane) = mux.get_pane(*pane_id) {
                    let dims = pane.get_dimensions();
                    overlay
                        .resize(TerminalSize {
                            cols: dims.cols,
                            rows: dims.viewport_rows,
                            dpi: self.terminal_size.dpi,
                            pixel_height: (self.terminal_size.pixel_height
                                / self.terminal_size.rows)
                                * dims.viewport_rows,
                            pixel_width: (self.terminal_size.pixel_width / self.terminal_size.cols)
                                * dims.cols,
                        })
                        .ok();
                }
            }
        }
    }

    pub fn get_viewport(&self, pane_id: PaneId) -> Option<StableRowIndex> {
        self.leaf_ui_state(pane_id).viewport
    }

    pub fn set_viewport(
        &mut self,
        pane_id: PaneId,
        position: Option<StableRowIndex>,
        dims: RenderableDimensions,
    ) {
        let pos = match position {
            Some(pos) => {
                // Drop out of scrolling mode if we're off the bottom
                if pos >= dims.physical_top {
                    None
                } else {
                    Some(pos.max(dims.scrollback_top))
                }
            }
            None => None,
        };

        let mut state = self.leaf_ui_state(pane_id);
        if pos != state.viewport {
            state.viewport = pos;

            // This is a bit gross.  If we add other overlays that need this information,
            // this should get extracted out into a trait
            if let Some(overlay) = state.overlay.as_ref() {
                if let Some(copy) = overlay.pane.downcast_ref::<CopyOverlay>() {
                    copy.viewport_changed(pos);
                } else if let Some(qs) = overlay.pane.downcast_ref::<QuickSelectOverlay>() {
                    qs.viewport_changed(pos);
                }
            }
        }
        self.window.as_ref().unwrap().invalidate();
    }

    fn maybe_scroll_to_bottom_for_input(&mut self, pane: &Arc<dyn Pane>) {
        if self.config.scroll_to_bottom_on_input {
            self.scroll_to_bottom(pane);
        }
    }

    fn scroll_to_top(&mut self, pane: &Arc<dyn Pane>) {
        let dims = pane.get_dimensions();
        self.set_viewport(pane.pane_id(), Some(dims.scrollback_top), dims);
    }

    fn scroll_to_bottom(&mut self, pane: &Arc<dyn Pane>) {
        self.leaf_ui_state(pane.pane_id()).viewport = None;
    }

    fn active_host_leaf(&self) -> Option<Arc<dyn Pane>> {
        if self.chatminal_sidebar.is_enabled() {
            if let Some(leaf_id) = self.active_public_leaf_id() {
                if let Some(pane) = self.resolve_public_leaf(leaf_id) {
                    return Some(pane);
                }
            }
        }

        self.active_host_surface().and_then(|tab| tab.get_active_pane())
    }

    fn get_active_leaf_no_overlay(&self) -> Option<Arc<dyn Pane>> {
        self.active_host_leaf()
    }

    /// Returns a leaf we can interact with; this will typically be
    /// the active host surface leaf for the window, but if the window has a surface-wide
    /// overlay (such as the launcher / tab navigator),
    /// then that will be returned instead. Otherwise, if the leaf has
    /// an active overlay (such as search or copy mode) then that will
    /// be returned.
    pub fn get_active_leaf_or_overlay(&self) -> Option<Arc<dyn Pane>> {
        if let Some(tab_overlay) = self
            .active_surface_id()
            .and_then(|surface_id| self.surface_overlay(surface_id))
        {
            Some(tab_overlay)
        } else {
            let pane = self.active_host_leaf()?;
            let pane_id = pane.pane_id();
            self.leaf_ui_state(pane_id)
                .overlay
                .as_ref()
                .map(|overlay| overlay.pane.clone())
                .or_else(|| Some(pane))
        }
    }

    fn get_splits(&mut self) -> Vec<PositionedSplit> {
        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return vec![],
        };

        let tab_id = tab.tab_id();

        if self.host_surface_overlay(tab_id).is_some() {
            vec![]
        } else {
            tab.iter_splits()
        }
    }

    fn positioned_leaf_to_leaf_info(pos: &PositionedPane) -> LeafInformation {
        let host_leaf_id = pos.pane.pane_id() as u64;
        let leaf_id = pane_metadata_leaf_id(&*pos.pane)
            .map(|leaf_id| leaf_id.as_u64())
            .unwrap_or(host_leaf_id);
        LeafInformation {
            host_leaf_id,
            leaf_id,
            leaf_index: pos.index,
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

    fn get_surface_information(&mut self) -> Vec<SurfaceInformation> {
        let mux = Mux::get();
        let window = match mux.get_window(self.window_id) {
            Some(window) => window,
            _ => return vec![],
        };

        if self.chatminal_sidebar.is_enabled() {
            let snapshot = self.chatminal_sidebar.snapshot();
            let lookup = chatminal_session_surface::collect_session_surface_lookup(
                self.window_id as DesktopWindowId,
            );
            let leaves_by_session: HashMap<String, Vec<LeafInformation>> = snapshot
                .sessions
                .iter()
                .filter_map(|session| {
                    let tab = chatminal_session_surface::host_surface_for_session(
                        self.window_id as DesktopWindowId,
                        &session.session_id,
                    )?;
                    Some((
                        session.session_id.clone(),
                        self.get_pos_leaves_for_host_surface(&tab)
                            .iter()
                            .map(Self::positioned_leaf_to_leaf_info)
                            .collect(),
                    ))
                })
                .collect();

            return snapshot
                .sessions
                .into_iter()
                .enumerate()
                .map(|(idx, session)| {
                    let host_surface = chatminal_session_surface::host_surface_for_session(
                        self.window_id as DesktopWindowId,
                        &session.session_id,
                    );
                    let surface_id = host_surface
                        .as_ref()
                        .and_then(|tab| tab.get_active_pane())
                        .and_then(|pane| pane_metadata_surface_id(&*pane))
                        .or_else(|| {
                            lookup
                                .surface_ids_by_session
                                .get(&session.session_id)
                                .copied()
                        });
                    let host_surface_id = surface_id
                        .map(|surface_id| surface_id.as_u64())
                        .or_else(|| host_surface.as_ref().map(|tab| tab.tab_id() as u64))
                        .unwrap_or(0);
                    let leaves = leaves_by_session
                        .get(&session.session_id)
                        .cloned()
                        .unwrap_or_default();
                    let active_leaf = leaves.iter().find(|leaf| leaf.is_active).cloned();
                    let active_leaf_id = chatminal_session_surface::active_leaf_id(
                        self.window_id as DesktopWindowId,
                        &session.session_id,
                    );

                    SurfaceInformation {
                        surface_index: idx,
                        host_surface_id,
                        is_active: lookup.active_session_id.as_deref()
                            == Some(session.session_id.as_str()),
                        is_last_active: lookup.last_active_session_id.as_deref()
                            == Some(session.session_id.as_str()),
                        window_id: self.window_id as DesktopWindowId,
                        surface_title: session.name,
                        active_leaf,
                        leaves,
                        session_id: Some(session.session_id),
                        surface_id,
                        active_leaf_id,
                    }
                })
                .collect();
        }

        let tab_index = window.get_active_idx();

        window
            .iter()
            .enumerate()
            .map(|(idx, tab)| {
                let leaves = self
                    .get_pos_leaves_for_host_surface(tab)
                    .iter()
                    .map(Self::positioned_leaf_to_leaf_info)
                    .collect::<Vec<_>>();

                SurfaceInformation {
                    surface_index: idx,
                    host_surface_id: tab.tab_id() as u64,
                    is_active: tab_index == idx,
                    is_last_active: window
                        .get_last_active_idx()
                        .map(|last_active| last_active == idx)
                        .unwrap_or(false),
                    window_id: self.window_id as DesktopWindowId,
                    surface_title: tab.get_title(),
                    active_leaf: leaves.iter().find(|leaf| leaf.is_active).cloned(),
                    leaves,
                    session_id: None,
                    surface_id: None,
                    active_leaf_id: None,
                }
            })
            .collect()
    }

    fn get_leaf_information(&self) -> Vec<LeafInformation> {
        self.get_panes_to_render()
            .iter()
            .map(Self::positioned_leaf_to_leaf_info)
            .collect()
    }

    fn get_pos_leaves_for_host_surface(&self, tab: &Arc<Tab>) -> Vec<PositionedPane> {
        let tab_id = tab.tab_id();

        if let Some(pane) = self.host_surface_overlay(tab_id) {
            let size = tab.get_size();
            vec![PositionedPane {
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
            let mut panes = tab.iter_panes();
            for p in &mut panes {
                if let Some(overlay) = self.leaf_ui_state(p.pane.pane_id()).overlay.as_ref() {
                    p.pane = Arc::clone(&overlay.pane);
                }
            }
            panes
        }
    }

    fn get_panes_to_render(&self) -> Vec<PositionedPane> {
        let tab = match self.active_host_surface() {
            Some(tab) => tab,
            None => return vec![],
        };

        self.get_pos_leaves_for_host_surface(&tab)
    }

    /// If `host_leaf_id` is `None`, removes any overlay for the specified host surface.
    /// Otherwise removes the overlay only if it belongs to the specified host leaf.
    fn cancel_overlay_for_host_surface(
        &mut self,
        host_surface_id: TabId,
        host_leaf_id: Option<PaneId>,
    ) {
        if host_leaf_id.is_some() {
            let current = self
                .host_surface_overlay(host_surface_id)
                .map(|overlay| overlay.pane_id());
            if current != host_leaf_id {
                return;
            }
        }
        if let Some(overlay) = self.surface_ui_state(host_surface_id).overlay.take() {
            Mux::get().remove_pane(overlay.pane.pane_id());
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn schedule_cancel_overlay_for_host_surface(
        window: Window,
        host_surface_id: u64,
        host_leaf_id: Option<u64>,
    ) {
        window.notify(TermWindowNotif::CancelOverlayForHostSurfaceId {
            host_surface_id,
            pane_id: host_leaf_id,
        });
    }

    fn cancel_overlay_for_surface(&mut self, surface_id: SurfaceId, host_leaf_id: Option<PaneId>) {
        let Some(host_surface_id) = self.host_surface_id_for_surface(surface_id) else {
            log::error!("surface id {surface_id} out of range for overlay cancel");
            return;
        };
        self.cancel_overlay_for_host_surface(host_surface_id, host_leaf_id);
    }

    pub fn schedule_cancel_overlay_for_surface(
        window: Window,
        surface_id: SurfaceId,
        pane_id: Option<u64>,
    ) {
        window.notify(TermWindowNotif::CancelOverlayForSurfaceId {
            surface_id: surface_id.as_u64(),
            pane_id,
        });
    }

    fn cancel_overlay_for_leaf(&mut self, host_leaf_id: PaneId) {
        if let Some(overlay) = self.leaf_ui_state(host_leaf_id).overlay.take() {
            // Ungh, when I built the CopyOverlay, its pane doesn't get
            // added to the mux and instead it reports the overlaid
            // pane id.  Take care to avoid killing ourselves off
            // when closing the CopyOverlay
            if host_leaf_id != overlay.pane.pane_id() {
                Mux::get().remove_pane(overlay.pane.pane_id());
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn schedule_cancel_overlay_for_leaf(window: Window, leaf_id: u64) {
        window.notify(TermWindowNotif::CancelOverlayForLeafId(leaf_id));
    }

    pub fn assign_overlay_for_leaf(&mut self, host_leaf_id: PaneId, pane: Arc<dyn Pane>) {
        self.cancel_overlay_for_leaf(host_leaf_id);
        self.leaf_ui_state(host_leaf_id).overlay.replace(OverlayState {
            pane,
            key_table_state: KeyTableState::default(),
        });
        self.update_title();
    }

    pub fn assign_overlay_for_host_surface(&mut self, host_surface_id: TabId, overlay: Arc<dyn Pane>) {
        self.cancel_overlay_for_host_surface(host_surface_id, None);
        self.surface_ui_state(host_surface_id).overlay.replace(OverlayState {
            pane: overlay,
            key_table_state: KeyTableState::default(),
        });
        self.update_title();
    }

    pub fn assign_overlay_for_surface(&mut self, surface_id: SurfaceId, overlay: Arc<dyn Pane>) {
        let Some(host_surface_id) = self.host_surface_id_for_surface(surface_id) else {
            log::error!("surface id {surface_id} out of range for overlay assignment");
            return;
        };
        self.cancel_overlay_for_surface(surface_id, None);
        self.surface_ui_state(host_surface_id).overlay.replace(OverlayState {
            pane: overlay,
            key_table_state: KeyTableState::default(),
        });
        self.update_title();
    }

    fn resolve_search_pattern(&self, pattern: Pattern, pane: &Arc<dyn Pane>) -> MuxPattern {
        match pattern {
            Pattern::CaseSensitiveString(s) => MuxPattern::CaseSensitiveString(s),
            Pattern::CaseInSensitiveString(s) => MuxPattern::CaseInSensitiveString(s),
            Pattern::Regex(s) => MuxPattern::Regex(s),
            Pattern::CurrentSelectionOrEmptyString => {
                let text = self.selection_text(pane);
                let first_line = text
                    .lines()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                MuxPattern::CaseSensitiveString(first_line)
            }
        }
    }
}

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
