use std::collections::HashMap;
use std::future::Future;
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;

use runtime::{
    CachePolicy, LogicalLine, RenderableDimensions, SessionTerminalHandle, SplitRequest,
    StableCursorPosition,
};
use config::keyassignment::SessionDirection;
use dynamic::Value;
use terminal_emulator::{Progress, SemanticZone, StableRowIndex, TerminalSize};
use procinfo::LocalProcessInfo;
use termwiz::surface::Line;
use url::Url;

pub type LuaWindowId = u64;
pub type BackendFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;

#[derive(Clone, Debug)]
pub enum LuaSplitSource {
    Spawn {
        command: Option<portable_pty::CommandBuilder>,
        command_dir: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct LuaSessionRecord {
    pub session_id: Option<String>,
    pub title: String,
    pub size: TerminalSize,
    pub active_terminal: Option<SessionTerminalHandle>,
    pub active_terminal_instance_id: Option<u64>,
    pub is_active: bool,
}

#[derive(Clone, Debug)]
pub struct LuaSessionTerminalRecord {
    pub index: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub pixel_width: usize,
    pub height: usize,
    pub pixel_height: usize,
    pub terminal: SessionTerminalHandle,
    pub session_id: Option<String>,
    pub terminal_instance_id: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct LuaSpawnContext {
    pub size: TerminalSize,
    pub sibling: Option<SessionTerminalHandle>,
}

#[derive(Clone, Debug)]
pub struct LuaSpawnResult {
    pub session_id: String,
    pub terminal: SessionTerminalHandle,
}

pub trait LuaPane: Send + Sync {
    fn terminal_handle(&self) -> SessionTerminalHandle;
    fn get_metadata(&self) -> Value;
    fn send_paste(&self, text: &str) -> anyhow::Result<()>;
    fn send_text(&self, text: &str) -> anyhow::Result<()>;
    fn get_title(&self) -> String;
    fn get_progress(&self) -> Progress;
    fn get_current_working_dir(&self, policy: CachePolicy) -> Option<Url>;
    fn get_foreground_process_name(&self, policy: CachePolicy) -> Option<String>;
    fn get_foreground_process_info(&self, policy: CachePolicy) -> Option<LocalProcessInfo>;
    fn get_cursor_position(&self) -> StableCursorPosition;
    fn get_dimensions(&self) -> RenderableDimensions;
    fn copy_user_vars(&self) -> HashMap<String, String>;
    fn has_unseen_output(&self) -> bool;
    fn is_alt_screen_active(&self) -> bool;
    fn get_lines(&self, lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>);
    fn get_logical_lines(&self, lines: Range<StableRowIndex>) -> Vec<LogicalLine>;
    fn perform_actions(&self, actions: Vec<termwiz::escape::Action>);
    fn get_semantic_zones(&self) -> anyhow::Result<Vec<SemanticZone>>;
    fn tty_name(&self) -> Option<String>;
}

pub trait LuaBridgeBackend: Send + Sync {
    fn is_runtime_available(&self) -> bool;
    fn root_window_id(&self) -> LuaWindowId;
    fn pane(&self, terminal_handle: SessionTerminalHandle) -> Option<Arc<dyn LuaPane>>;
    fn all_panes(&self) -> Vec<Arc<dyn LuaPane>>;
    fn root_sessions(&self) -> Vec<LuaSessionRecord>;
    fn session(&self, session_id: &str) -> Option<LuaSessionRecord>;
    fn spawn_root_session(
        &self,
        cmd_builder: Option<portable_pty::CommandBuilder>,
        cwd: Option<String>,
        size: TerminalSize,
        pane: Option<SessionTerminalHandle>,
    ) -> BackendFuture<anyhow::Result<LuaSpawnResult>>;
    fn split_terminal(
        &self,
        terminal_handle: SessionTerminalHandle,
        request: SplitRequest,
        source: LuaSplitSource,
    ) -> BackendFuture<anyhow::Result<SessionTerminalHandle>>;
    fn active_workspace(&self) -> anyhow::Result<String>;
    fn root_workspace(&self) -> anyhow::Result<String>;
    fn set_root_workspace(&self, workspace: &str) -> anyhow::Result<()>;
    fn workspace_names(&self) -> anyhow::Result<Vec<String>>;
    fn set_active_workspace(&self, workspace: &str) -> anyhow::Result<()>;
    fn rename_workspace(&self, old_workspace: &str, new_workspace: &str) -> anyhow::Result<()>;
    fn root_title(&self) -> anyhow::Result<String>;
    fn set_root_title(&self, title: &str) -> anyhow::Result<()>;
    fn root_spawn_context(&self) -> anyhow::Result<LuaSpawnContext>;
    fn set_session_title(&self, session_id: &str, title: &str) -> anyhow::Result<()>;
    fn session_terminals(&self, session_id: &str) -> anyhow::Result<Vec<SessionTerminalHandle>>;
    fn terminal_in_direction(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> anyhow::Result<Option<SessionTerminalHandle>>;
    fn set_session_zoomed(&self, session_id: &str, zoomed: bool) -> anyhow::Result<bool>;
    fn session_terminal_infos(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Vec<LuaSessionTerminalRecord>>;
    fn rotate_session_counter_clockwise(&self, session_id: &str) -> anyhow::Result<()>;
    fn rotate_session_clockwise(&self, session_id: &str) -> anyhow::Result<()>;
    fn activate_terminal(&self, terminal_handle: SessionTerminalHandle) -> anyhow::Result<()>;
    fn activate_session(&self, session_id: &str) -> anyhow::Result<()>;
}

static BACKEND: OnceLock<Arc<dyn LuaBridgeBackend>> = OnceLock::new();

pub fn install_backend(
    backend: Arc<dyn LuaBridgeBackend>,
) -> Result<(), Arc<dyn LuaBridgeBackend>> {
    BACKEND.set(backend)
}

pub fn installed_backend() -> Option<Arc<dyn LuaBridgeBackend>> {
    BACKEND.get().map(Arc::clone)
}
