use runtime::{SessionTerminalHandle, SplitDirection, SplitRequest, SplitSize};
use config::keyassignment::SessionDirection;
use config::lua::mlua::{self, Lua, UserData, UserDataMethods, Value as LuaValue};
use config::lua::{get_or_create_module, get_or_create_sub_module};
use dynamic::{FromDynamic, ToDynamic, Value};
use luahelper::impl_lua_conversion_dynamic;
use portable_pty::CommandBuilder;
use std::collections::HashMap;
use std::sync::Arc;

pub mod backend;
mod leaf;
mod session;
mod window;

pub use backend::install_backend;
use backend::{
    installed_backend, LuaBridgeBackend, LuaPane, LuaSessionRecord, LuaSpawnContext, LuaSplitSource,
};
pub use leaf::TerminalRef;
pub use session::SessionRef;
pub use window::WindowRef;
pub(crate) type PaneCachePolicy = runtime::CachePolicy;

pub(crate) fn root_window_id() -> u64 {
    installed_backend()
        .map(|backend| backend.root_window_id())
        .unwrap_or_default()
}

#[derive(Clone, Debug)]
struct SpawnedSessionHandle {
    session: SessionRef,
    terminal: TerminalRef,
}

#[derive(Clone)]
pub(crate) struct LuaBridgeHost(Arc<dyn LuaBridgeBackend>);

impl std::fmt::Debug for LuaBridgeHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuaBridgeHost").finish_non_exhaustive()
    }
}

impl LuaBridgeHost {
    fn global() -> mlua::Result<Self> {
        let Some(backend) = installed_backend() else {
            return Err(mlua::Error::external("lua bridge backend is not installed"));
        };
        Ok(Self(backend))
    }

    fn ensure_runtime_available(&self) -> mlua::Result<()> {
        if self.0.is_runtime_available() {
            Ok(())
        } else {
            Err(mlua::Error::external("root window not available"))
        }
    }

    fn terminal_by_handle(
        &self,
        terminal_handle: SessionTerminalHandle,
    ) -> Option<Arc<dyn LuaPane>> {
        self.0.pane(terminal_handle)
    }

    async fn spawn_root_session(
        &self,
        cmd_builder: Option<CommandBuilder>,
        cwd: Option<String>,
        size: terminal_emulator::TerminalSize,
        pane: Option<SessionTerminalHandle>,
    ) -> anyhow::Result<SpawnedSessionHandle> {
        let spawned = self
            .0
            .spawn_root_session(cmd_builder, cwd, size, pane)
            .await?;
        let terminal = TerminalRef::from_terminal_handle(spawned.terminal);
        let session = SessionRef::new(spawned.session_id);
        Ok(SpawnedSessionHandle { session, terminal })
    }

    async fn split_terminal_handle(
        &self,
        terminal: TerminalRef,
        request: SplitRequest,
        source: LuaSplitSource,
    ) -> anyhow::Result<TerminalRef> {
        let terminal_handle = self
            .0
            .split_terminal(terminal.terminal_handle(), request, source)
            .await?;
        Ok(TerminalRef::from_terminal_handle(terminal_handle))
    }

    pub(crate) fn active_workspace(&self) -> mlua::Result<String> {
        self.0.active_workspace().map_err(mlua::Error::external)
    }

    pub(crate) fn root_workspace(&self) -> mlua::Result<String> {
        self.0.root_workspace().map_err(mlua::Error::external)
    }

    pub(crate) fn set_root_workspace(&self, workspace: &str) -> mlua::Result<()> {
        self.0
            .set_root_workspace(workspace)
            .map_err(mlua::Error::external)
    }

    pub(crate) fn workspace_names(&self) -> mlua::Result<Vec<String>> {
        self.0.workspace_names().map_err(mlua::Error::external)
    }

    pub(crate) fn set_active_workspace(&self, workspace: &str) -> mlua::Result<()> {
        self.0
            .set_active_workspace(workspace)
            .map_err(mlua::Error::external)?;
        let active_workspace = self.active_workspace()?;
        if active_workspace == workspace {
            Ok(())
        } else {
            Err(mlua::Error::external(format!(
                "failed to activate workspace '{workspace}': active workspace is '{active_workspace}'"
            )))
        }
    }

    pub(crate) fn rename_workspace(
        &self,
        old_workspace: &str,
        new_workspace: &str,
    ) -> mlua::Result<()> {
        if old_workspace == new_workspace {
            return Ok(());
        }
        self.0
            .rename_workspace(old_workspace, new_workspace)
            .map_err(mlua::Error::external)
    }

    pub(crate) fn root_sessions_with_info(&self) -> mlua::Result<Vec<LuaSessionRecord>> {
        Ok(self.0.root_sessions())
    }

    pub(crate) fn root_sessions(&self) -> mlua::Result<Vec<SessionRef>> {
        Ok(self
            .root_sessions_with_info()?
            .into_iter()
            .filter_map(|session| session.session_id.map(SessionRef::new))
            .collect())
    }

    pub(crate) fn root_title(&self) -> mlua::Result<String> {
        self.0.root_title().map_err(mlua::Error::external)
    }

    pub(crate) fn set_root_title(&self, title: &str) -> mlua::Result<()> {
        self.0.set_root_title(title).map_err(mlua::Error::external)
    }

    pub(crate) fn root_active_session(&self) -> mlua::Result<Option<SessionRef>> {
        Ok(self
            .root_sessions_with_info()?
            .into_iter()
            .find(|session| session.is_active)
            .and_then(|session| session.session_id.map(SessionRef::new)))
    }

    pub(crate) fn root_active_session_id(&self) -> mlua::Result<Option<String>> {
        Ok(self
            .root_sessions_with_info()?
            .into_iter()
            .find(|session| session.is_active)
            .and_then(|session| session.session_id))
    }

    pub(crate) fn root_active_terminal(&self) -> mlua::Result<Option<TerminalRef>> {
        Ok(self
            .root_sessions_with_info()?
            .into_iter()
            .find(|session| session.is_active)
            .and_then(|session| session.active_terminal)
            .map(TerminalRef::from_terminal_handle))
    }

    pub(crate) fn panes(&self) -> mlua::Result<Vec<Arc<dyn LuaPane>>> {
        self.ensure_runtime_available()?;
        Ok(self.0.all_panes())
    }

    pub(crate) fn pane(&self, terminal: TerminalRef) -> mlua::Result<Arc<dyn LuaPane>> {
        self.terminal_by_handle(terminal.terminal_handle())
            .ok_or_else(|| {
                mlua::Error::external(format!(
                    "terminal handle {} not found in runtime",
                    terminal.terminal_handle_value()
                ))
            })
    }

    pub(crate) fn with_pane<R>(
        &self,
        terminal: TerminalRef,
        func: impl FnOnce(&Arc<dyn LuaPane>) -> R,
    ) -> mlua::Result<R> {
        let pane = self.pane(terminal)?;
        Ok(func(&pane))
    }

    pub(crate) fn with_pane_result<R>(
        &self,
        terminal: TerminalRef,
        func: impl FnOnce(&Arc<dyn LuaPane>) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        let pane = self.pane(terminal)?;
        func(&pane)
    }

    fn session_record(&self, session_id: &str) -> mlua::Result<LuaSessionRecord> {
        self.0.session(session_id).ok_or_else(|| {
            mlua::Error::external(format!("session '{}' not found in runtime", session_id))
        })
    }

    pub(crate) fn session_active_terminal_instance_id(
        &self,
        session_id: &str,
    ) -> mlua::Result<Option<u64>> {
        self.session_record(session_id)
            .map(|session| session.active_terminal_instance_id)
    }

    pub(crate) fn session_window(&self, session_id: &str) -> mlua::Result<Option<WindowRef>> {
        let session = self.session_record(session_id)?;
        Ok(session
            .active_terminal
            .and_then(|handle| self.terminal_by_handle(handle))
            .map(|_| WindowRef::root()))
    }

    pub(crate) fn session_title(&self, session_id: &str) -> mlua::Result<String> {
        self.session_record(session_id).map(|session| session.title)
    }

    pub(crate) fn set_session_title(&self, session_id: &str, title: &str) -> mlua::Result<()> {
        self.0
            .set_session_title(session_id, title)
            .map_err(mlua::Error::external)
    }

    pub(crate) fn active_terminal_for_session(
        &self,
        session_id: &str,
    ) -> mlua::Result<Option<TerminalRef>> {
        self.session_record(session_id).map(|session| {
            session
                .active_terminal
                .map(TerminalRef::from_terminal_handle)
        })
    }

    pub(crate) fn terminals_for_session(&self, session_id: &str) -> mlua::Result<Vec<TerminalRef>> {
        Ok(self
            .0
            .session_terminals(session_id)
            .map_err(mlua::Error::external)?
            .into_iter()
            .map(TerminalRef::from_terminal_handle)
            .collect())
    }

    pub(crate) fn terminal_direction_for_session(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> mlua::Result<Option<TerminalRef>> {
        Ok(self
            .0
            .terminal_in_direction(session_id, direction)
            .map_err(mlua::Error::external)?
            .map(TerminalRef::from_terminal_handle))
    }

    pub(crate) fn set_session_zoomed(&self, session_id: &str, zoomed: bool) -> mlua::Result<bool> {
        self.0
            .set_session_zoomed(session_id, zoomed)
            .map_err(mlua::Error::external)
    }

    pub(crate) fn session_terminals_with_info(
        &self,
        session_id: &str,
    ) -> mlua::Result<Vec<SessionTerminalInfo>> {
        Ok(self
            .0
            .session_terminal_infos(session_id)
            .map_err(mlua::Error::external)?
            .into_iter()
            .map(|info| SessionTerminalInfo {
                leaf: LeafInfo {
                    index: info.index,
                    is_active: info.is_active,
                    is_zoomed: info.is_zoomed,
                    left: info.left,
                    top: info.top,
                    width: info.width,
                    pixel_width: info.pixel_width,
                    height: info.height,
                    pixel_height: info.pixel_height,
                },
                terminal: TerminalRef::from_terminal_handle(info.terminal),
                session_id: info.session_id,
                terminal_instance_id: info.terminal_instance_id,
            })
            .collect())
    }

    pub(crate) fn rotate_session_counter_clockwise(&self, session_id: &str) -> mlua::Result<()> {
        self.0
            .rotate_session_counter_clockwise(session_id)
            .map_err(mlua::Error::external)
    }

    pub(crate) fn rotate_session_clockwise(&self, session_id: &str) -> mlua::Result<()> {
        self.0
            .rotate_session_clockwise(session_id)
            .map_err(mlua::Error::external)
    }

    pub(crate) fn session_size(&self, session_id: &str) -> mlua::Result<terminal_emulator::TerminalSize> {
        self.session_record(session_id).map(|session| session.size)
    }

    pub(crate) fn terminal_window(&self, terminal: TerminalRef) -> Option<WindowRef> {
        self.terminal_by_handle(terminal.terminal_handle())
            .map(|_| WindowRef::root())
    }

    pub(crate) fn terminal_session(
        &self,
        terminal: TerminalRef,
    ) -> mlua::Result<Option<SessionRef>> {
        self.with_pane(terminal, |pane| pane_session_id(pane).map(SessionRef::new))
    }

    pub(crate) fn activate_terminal(&self, terminal: TerminalRef) -> mlua::Result<()> {
        self.0
            .activate_terminal(terminal.terminal_handle())
            .map_err(mlua::Error::external)
    }

    pub(crate) fn activate_session(&self, session: &SessionRef) -> mlua::Result<()> {
        self.0
            .activate_session(session.as_str())
            .map_err(mlua::Error::external)
    }

    pub(crate) fn root_window_spawn_context(
        &self,
    ) -> mlua::Result<(terminal_emulator::TerminalSize, Option<TerminalRef>)> {
        let LuaSpawnContext { size, sibling } =
            self.0.root_spawn_context().map_err(mlua::Error::external)?;
        Ok((size, sibling.map(TerminalRef::from_terminal_handle)))
    }

    pub(crate) async fn spawn_session_from_root_window(
        &self,
        cmd_builder: Option<CommandBuilder>,
        cwd: Option<String>,
    ) -> mlua::Result<(SessionRef, TerminalRef)> {
        let (size, pane) = self.root_window_spawn_context()?;
        let spawned = self
            .spawn_root_session(
                cmd_builder,
                cwd,
                size,
                pane.map(TerminalRef::terminal_handle),
            )
            .await
            .map_err(|e| mlua::Error::external(format!("{:#?}", e)))?;
        Ok((spawned.session, spawned.terminal))
    }

    pub(crate) async fn split_terminal(
        &self,
        terminal: TerminalRef,
        request: SplitRequest,
        source: LuaSplitSource,
    ) -> mlua::Result<TerminalRef> {
        self.split_terminal_handle(terminal, request, source)
            .await
            .map_err(|e| mlua::Error::external(format!("{:#?}", e)))
    }
}

fn get_host() -> mlua::Result<LuaBridgeHost> {
    LuaBridgeHost::global()
}

fn bind_host_to_lua(lua: &Lua) {
    if lua.app_data_ref::<LuaBridgeHost>().is_none() {
        if let Ok(host) = get_host() {
            let _ = lua.set_app_data(host);
        }
    }
}

fn get_host_for_lua(lua: &Lua) -> mlua::Result<LuaBridgeHost> {
    if let Some(host) = lua.app_data_ref::<LuaBridgeHost>() {
        return Ok((*host).clone());
    }
    get_host()
}

pub(crate) fn pane_metadata_string(pane: &Arc<dyn LuaPane>, key: &str) -> Option<String> {
    match pane.get_metadata() {
        Value::Object(obj) => {
            obj.get(&Value::String(key.to_string()))
                .and_then(|value| match value {
                    Value::String(value) => Some(value.clone()),
                    _ => None,
                })
        }
        _ => None,
    }
}

pub(crate) fn pane_metadata_u64(pane: &Arc<dyn LuaPane>, key: &str) -> Option<u64> {
    match pane.get_metadata() {
        Value::Object(obj) => {
            obj.get(&Value::String(key.to_string()))
                .and_then(|value| match value {
                    Value::U64(value) => Some(*value),
                    Value::I64(value) => (*value).try_into().ok(),
                    _ => None,
                })
        }
        _ => None,
    }
}

pub(crate) fn pane_session_id(pane: &Arc<dyn LuaPane>) -> Option<String> {
    pane_metadata_string(pane, "chatminal_session_id")
}

pub(crate) fn pane_terminal_instance_id(pane: &Arc<dyn LuaPane>) -> Option<u64> {
    pane_metadata_u64(pane, "chatminal_terminal_instance_id")
}

pub fn register(lua: &Lua) -> anyhow::Result<()> {
    bind_host_to_lua(lua);
    let session_module = get_or_create_sub_module(lua, "session")?;

    session_module.set(
        "get_active_workspace",
        lua.create_function(|lua, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.active_workspace()
        })?,
    )?;

    session_module.set(
        "get_workspace_names",
        lua.create_function(|lua, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.workspace_names()
        })?,
    )?;

    session_module.set(
        "set_active_workspace",
        lua.create_function(|lua, workspace: String| {
            let host = get_host_for_lua(lua)?;
            let workspaces = host.workspace_names()?;
            if workspaces.contains(&workspace) {
                host.set_active_workspace(&workspace)
            } else {
                Err(mlua::Error::external(format!(
                    "{:?} is not an existing workspace",
                    workspace
                )))
            }
        })?,
    )?;

    session_module.set(
        "rename_workspace",
        lua.create_function(|lua, (old_workspace, new_workspace): (String, String)| {
            let host = get_host_for_lua(lua)?;
            host.rename_workspace(&old_workspace, &new_workspace)
        })?,
    )?;

    session_module.set(
        "get_window",
        lua.create_function(|_, _: ()| Ok(WindowRef::root()))?,
    )?;

    session_module.set(
        "all_sessions",
        lua.create_function(|lua, _: ()| get_host_for_lua(lua)?.root_sessions())?,
    )?;

    session_module.set(
        "list_sessions",
        lua.create_function(|lua, _: ()| get_host_for_lua(lua)?.root_sessions())?,
    )?;

    session_module.set(
        "all_terminals",
        lua.create_function(|lua, _: ()| {
            let host = get_host_for_lua(lua)?;
            Ok(host
                .panes()?
                .into_iter()
                .map(|pane| TerminalRef::from_terminal_handle(pane.terminal_handle()))
                .collect::<Vec<TerminalRef>>())
        })?,
    )?;

    Ok(())
}

#[derive(Debug, Default, FromDynamic, ToDynamic)]
struct CommandBuilderFrag {
    args: Option<Vec<String>>,
    cwd: Option<String>,
    #[dynamic(default)]
    set_environment_variables: HashMap<String, String>,
}

impl CommandBuilderFrag {
    fn to_command_builder(&self) -> (Option<CommandBuilder>, Option<String>) {
        if let Some(args) = &self.args {
            let mut builder = CommandBuilder::from_argv(args.iter().map(Into::into).collect());
            for (k, v) in self.set_environment_variables.iter() {
                builder.env(k, v);
            }
            if let Some(cwd) = self.cwd.clone() {
                builder.cwd(cwd);
            }
            (Some(builder), None)
        } else {
            (None, self.cwd.clone())
        }
    }
}

#[derive(Debug, FromDynamic, ToDynamic)]
enum SessionSplitDirection {
    Left,
    Right,
    Top,
    Bottom,
}
impl_lua_conversion_dynamic!(SessionSplitDirection);

impl Default for SessionSplitDirection {
    fn default() -> Self {
        Self::Right
    }
}

#[derive(Debug, FromDynamic, ToDynamic)]
struct SpawnSession {
    #[dynamic(flatten)]
    cmd_builder: CommandBuilderFrag,
}
impl_lua_conversion_dynamic!(SpawnSession);

impl SpawnSession {
    async fn spawn(
        self,
        lua: &Lua,
        window: &WindowRef,
    ) -> mlua::Result<(SessionRef, TerminalRef, WindowRef)> {
        let host = get_host_for_lua(lua)?;
        let (cmd_builder, cwd) = self.cmd_builder.to_command_builder();
        let (session_ref, terminal_ref) = host
            .spawn_session_from_root_window(cmd_builder, cwd)
            .await?;
        Ok((session_ref, terminal_ref, *window))
    }
}

#[derive(Clone, FromDynamic, ToDynamic)]
struct SessionInfo {
    pub index: usize,
    pub is_active: bool,
}
impl_lua_conversion_dynamic!(SessionInfo);

#[derive(Clone, Debug, FromDynamic, ToDynamic)]
struct LeafInfo {
    pub index: usize,
    pub is_active: bool,
    pub is_zoomed: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub pixel_width: usize,
    pub height: usize,
    pub pixel_height: usize,
}
impl_lua_conversion_dynamic!(LeafInfo);

#[derive(Clone, Debug)]
pub(crate) struct SessionTerminalInfo {
    pub leaf: LeafInfo,
    pub terminal: TerminalRef,
    pub session_id: Option<String>,
    pub terminal_instance_id: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;
    use std::sync::Arc;

    use crate::backend::{
        BackendFuture, LuaBridgeBackend, LuaPane, LuaSessionRecord, LuaSessionTerminalRecord,
        LuaSpawnContext, LuaSpawnResult,
    };
    use runtime::{CachePolicy, LogicalLine};
    use terminal_emulator::{Progress, SemanticZone, StableRowIndex, TerminalSize};
    use procinfo::LocalProcessInfo;
    use termwiz::surface::Line;
    use url::Url;

    struct MockPane;

    impl LuaPane for MockPane {
        fn terminal_handle(&self) -> SessionTerminalHandle {
            SessionTerminalHandle::new(1)
        }
        fn get_metadata(&self) -> Value {
            Value::Null
        }
        fn send_paste(&self, _text: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn send_text(&self, _text: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_title(&self) -> String {
            String::new()
        }
        fn get_progress(&self) -> Progress {
            Progress::None
        }
        fn get_current_working_dir(&self, _policy: CachePolicy) -> Option<Url> {
            None
        }
        fn get_foreground_process_name(&self, _policy: CachePolicy) -> Option<String> {
            None
        }
        fn get_foreground_process_info(&self, _policy: CachePolicy) -> Option<LocalProcessInfo> {
            None
        }
        fn get_cursor_position(&self) -> runtime::StableCursorPosition {
            Default::default()
        }
        fn get_dimensions(&self) -> runtime::RenderableDimensions {
            Default::default()
        }
        fn copy_user_vars(&self) -> HashMap<String, String> {
            HashMap::new()
        }
        fn has_unseen_output(&self) -> bool {
            false
        }
        fn is_alt_screen_active(&self) -> bool {
            false
        }
        fn get_lines(&self, _lines: Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>) {
            (0, vec![])
        }
        fn get_logical_lines(&self, _lines: Range<StableRowIndex>) -> Vec<LogicalLine> {
            vec![]
        }
        fn perform_actions(&self, _actions: Vec<termwiz::escape::Action>) {}
        fn get_semantic_zones(&self) -> anyhow::Result<Vec<SemanticZone>> {
            Ok(vec![])
        }
        fn tty_name(&self) -> Option<String> {
            None
        }
    }

    struct MockBackend;

    impl LuaBridgeBackend for MockBackend {
        fn is_runtime_available(&self) -> bool {
            true
        }
        fn root_window_id(&self) -> u64 {
            1
        }
        fn pane(&self, _terminal_handle: SessionTerminalHandle) -> Option<Arc<dyn LuaPane>> {
            Some(Arc::new(MockPane))
        }
        fn all_panes(&self) -> Vec<Arc<dyn LuaPane>> {
            vec![Arc::new(MockPane)]
        }
        fn root_sessions(&self) -> Vec<LuaSessionRecord> {
            vec![LuaSessionRecord {
                session_id: Some("session-1".to_string()),
                title: "Session 1".to_string(),
                size: TerminalSize::default(),
                active_terminal: Some(SessionTerminalHandle::new(1)),
                active_terminal_instance_id: Some(11),
                is_active: true,
            }]
        }
        fn session(&self, session_id: &str) -> Option<LuaSessionRecord> {
            (session_id == "session-1").then(|| LuaSessionRecord {
                session_id: Some("session-1".to_string()),
                title: "Session 1".to_string(),
                size: TerminalSize::default(),
                active_terminal: Some(SessionTerminalHandle::new(1)),
                active_terminal_instance_id: Some(11),
                is_active: true,
            })
        }
        fn spawn_root_session(
            &self,
            _cmd_builder: Option<CommandBuilder>,
            _cwd: Option<String>,
            _size: TerminalSize,
            _pane: Option<SessionTerminalHandle>,
        ) -> BackendFuture<anyhow::Result<LuaSpawnResult>> {
            Box::pin(async {
                Ok(LuaSpawnResult {
                    session_id: "session-1".to_string(),
                    terminal: SessionTerminalHandle::new(1),
                })
            })
        }
        fn split_terminal(
            &self,
            _terminal_handle: SessionTerminalHandle,
            _request: SplitRequest,
            _source: LuaSplitSource,
        ) -> BackendFuture<anyhow::Result<SessionTerminalHandle>> {
            Box::pin(async { Ok(SessionTerminalHandle::new(2)) })
        }
        fn active_workspace(&self) -> anyhow::Result<String> {
            Ok("default".to_string())
        }
        fn root_workspace(&self) -> anyhow::Result<String> {
            Ok("default".to_string())
        }
        fn set_root_workspace(&self, _workspace: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn workspace_names(&self) -> anyhow::Result<Vec<String>> {
            Ok(vec!["default".to_string()])
        }
        fn set_active_workspace(&self, _workspace: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn rename_workspace(
            &self,
            _old_workspace: &str,
            _new_workspace: &str,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn root_title(&self) -> anyhow::Result<String> {
            Ok("Chatminal".to_string())
        }
        fn set_root_title(&self, _title: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn root_spawn_context(&self) -> anyhow::Result<LuaSpawnContext> {
            Ok(LuaSpawnContext {
                size: TerminalSize::default(),
                sibling: None,
            })
        }
        fn set_session_title(&self, _session_id: &str, _title: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn session_terminals(
            &self,
            session_id: &str,
        ) -> anyhow::Result<Vec<SessionTerminalHandle>> {
            if session_id == "session-1" {
                Ok(vec![SessionTerminalHandle::new(1)])
            } else {
                Err(anyhow::anyhow!("missing session"))
            }
        }
        fn terminal_in_direction(
            &self,
            _session_id: &str,
            _direction: SessionDirection,
        ) -> anyhow::Result<Option<SessionTerminalHandle>> {
            Ok(None)
        }
        fn set_session_zoomed(&self, _session_id: &str, _zoomed: bool) -> anyhow::Result<bool> {
            Ok(false)
        }
        fn session_terminal_infos(
            &self,
            session_id: &str,
        ) -> anyhow::Result<Vec<LuaSessionTerminalRecord>> {
            if session_id != "session-1" {
                return Err(anyhow::anyhow!("missing session"));
            }
            Ok(vec![LuaSessionTerminalRecord {
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 0,
                top: 0,
                width: 80,
                pixel_width: 800,
                height: 24,
                pixel_height: 480,
                terminal: SessionTerminalHandle::new(1),
                session_id: Some("session-1".to_string()),
                terminal_instance_id: Some(11),
            }])
        }
        fn rotate_session_counter_clockwise(&self, _session_id: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn rotate_session_clockwise(&self, _session_id: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn activate_terminal(&self, _terminal_handle: SessionTerminalHandle) -> anyhow::Result<()> {
            Ok(())
        }
        fn activate_session(&self, _session_id: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn install_test_backend() {
        let _ = install_backend(Arc::new(MockBackend));
    }

    #[test]
    fn bind_host_to_lua_registers_vm_local_host() {
        let lua = Lua::new();
        install_test_backend();

        assert!(lua.app_data_ref::<LuaBridgeHost>().is_none());

        bind_host_to_lua(&lua);

        assert!(lua.app_data_ref::<LuaBridgeHost>().is_some());
    }

    #[test]
    fn get_host_for_lua_prefers_bound_vm_host() {
        let lua = Lua::new();
        install_test_backend();
        bind_host_to_lua(&lua);

        assert!(get_host_for_lua(&lua).is_ok());
    }
}
