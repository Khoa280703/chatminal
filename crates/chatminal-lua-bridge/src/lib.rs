use chatminal_runtime::{RuntimeId, SessionTerminalHandle};
use config::keyassignment::SessionDirection;
use config::lua::mlua::{self, Lua, UserData, UserDataMethods, Value as LuaValue};
use config::lua::{get_or_create_module, get_or_create_sub_module};
use engine_dynamic::{FromDynamic, ToDynamic, Value};
use host_runtime::pane::Pane;
use host_runtime::spawn_target::SplitSource;
use host_runtime::tab::{SplitDirection, SplitRequest, SplitSize};
use host_runtime::RuntimeEntryInfo;
use luahelper::impl_lua_conversion_dynamic;
use portable_pty::CommandBuilder;
use std::collections::HashMap;
use std::sync::Arc;

mod leaf;
mod session;
mod window;

pub use leaf::TerminalRef;
pub use session::SessionRef;
pub use window::WindowRef;
pub(crate) type PaneCachePolicy = host_runtime::pane::CachePolicy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RootTabRef(RuntimeId);

impl RootTabRef {
    const fn from_runtime_id(runtime_id: RuntimeId) -> Self {
        Self(runtime_id)
    }

    const fn runtime_id(self) -> RuntimeId {
        self.0
    }
}

pub(crate) fn root_window_id() -> host_runtime::window::WindowId {
    host_runtime::window::ROOT_WINDOW_ID
}

pub(crate) fn terminal_handle_for_pane(pane: &dyn Pane) -> SessionTerminalHandle {
    host_runtime::terminal_handle_for_pane(pane)
}

#[derive(Clone, Debug)]
struct SpawnedSessionHandle {
    session: SessionRef,
    terminal: TerminalRef,
}

#[derive(Clone)]
pub(crate) struct LuaBridgeHost;

impl std::fmt::Debug for LuaBridgeHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuaBridgeHost").finish_non_exhaustive()
    }
}

impl LuaBridgeHost {
    fn global() -> mlua::Result<Self> {
        if !host_runtime::is_host_runtime_available() {
            return Err(mlua::Error::external("host runtime is not available"));
        }
        Ok(Self)
    }

    fn ensure_runtime_available(&self) -> mlua::Result<()> {
        if host_runtime::is_host_runtime_available() {
            Ok(())
        } else {
            Err(mlua::Error::external("root window not available"))
        }
    }

    fn terminal_by_handle(&self, terminal_handle: SessionTerminalHandle) -> Option<Arc<dyn Pane>> {
        host_runtime::terminal_by_handle(terminal_handle)
    }

    fn runtime_entry_info_by_runtime_id(&self, runtime_id: RuntimeId) -> Option<RuntimeEntryInfo> {
        host_runtime::runtime_entry_info_by_runtime_id(runtime_id)
    }

    fn runtime_entry_info_by_session_id(&self, session_id: &str) -> Option<RuntimeEntryInfo> {
        host_runtime::runtime_entry_info_by_session_id(session_id)
    }

    fn runtime_id_for_terminal(&self, terminal: TerminalRef) -> Option<RuntimeId> {
        host_runtime::resolve_runtime_id_for_terminal_handle(terminal.terminal_handle())
    }

    fn focus_root_runtime_id(&self, runtime_id: RuntimeId) -> bool {
        host_runtime::focus_root_runtime_entry(runtime_id)
    }

    fn root_window_spawn_state(
        &self,
    ) -> (engine_term::TerminalSize, Option<SessionTerminalHandle>) {
        host_runtime::root_window_spawn_context_state()
    }

    async fn spawn_root_session(
        &self,
        cmd_builder: Option<CommandBuilder>,
        cwd: Option<String>,
        size: engine_term::TerminalSize,
        pane: Option<SessionTerminalHandle>,
    ) -> anyhow::Result<SpawnedSessionHandle> {
        let (_tab, pane) = host_runtime::spawn_tab(cmd_builder, cwd, size, pane).await?;
        let terminal = terminal_ref_for_pane(pane.as_ref());
        let session = SessionRef::new(
            pane_session_id(&pane)
                .ok_or_else(|| anyhow::anyhow!("spawned session has no chatminal session_id"))?,
        );
        Ok(SpawnedSessionHandle { session, terminal })
    }

    async fn split_terminal_handle(
        &self,
        terminal: TerminalRef,
        request: SplitRequest,
        source: SplitSource,
    ) -> anyhow::Result<TerminalRef> {
        let (pane, _size) =
            host_runtime::split_pane(terminal.terminal_handle(), request, source).await?;
        Ok(terminal_ref_for_pane(pane.as_ref()))
    }

    pub(crate) fn active_workspace(&self) -> mlua::Result<String> {
        host_runtime::active_identity()
            .and_then(|client_id| host_runtime::active_workspace_for_client(&client_id))
            .or_else(host_runtime::root_window_workspace_name)
            .or_else(host_runtime::active_workspace_name)
            .ok_or_else(|| mlua::Error::external("root window not available"))
    }

    pub(crate) fn root_workspace(&self) -> mlua::Result<String> {
        host_runtime::root_window_workspace_name()
            .ok_or_else(|| mlua::Error::external("root window not available"))
    }

    pub(crate) fn set_root_workspace(&self, workspace: &str) -> mlua::Result<()> {
        if host_runtime::set_root_window_workspace_name(workspace) {
            Ok(())
        } else {
            Err(mlua::Error::external("root window not available"))
        }
    }

    pub(crate) fn workspace_names(&self) -> mlua::Result<Vec<String>> {
        self.ensure_runtime_available()?;
        Ok(host_runtime::iter_workspaces())
    }

    pub(crate) fn set_active_workspace(&self, workspace: &str) -> mlua::Result<()> {
        if !host_runtime::set_active_workspace_name(workspace) {
            return Err(mlua::Error::external(format!(
                "failed to activate workspace '{workspace}': host runtime is not available"
            )));
        }
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
        if !host_runtime::rename_workspace(old_workspace, new_workspace) {
            return Err(mlua::Error::external(format!(
                "failed to rename workspace '{old_workspace}' to '{new_workspace}': host runtime is not available"
            )));
        }
        Ok(())
    }

    pub(crate) fn root_tab_infos(&self) -> mlua::Result<Vec<RuntimeEntryInfo>> {
        self.ensure_runtime_available()?;
        Ok(host_runtime::root_runtime_entry_infos())
    }

    pub(crate) fn root_sessions(&self) -> mlua::Result<Vec<SessionRef>> {
        Ok(self
            .root_tab_infos()?
            .into_iter()
            .filter_map(|info| info.session_id.map(SessionRef::new))
            .collect())
    }

    pub(crate) fn root_title(&self) -> mlua::Result<String> {
        host_runtime::root_window_title()
            .ok_or_else(|| mlua::Error::external("root window not available"))
    }

    pub(crate) fn set_root_title(&self, title: &str) -> mlua::Result<()> {
        if host_runtime::set_root_window_title(title) {
            Ok(())
        } else {
            Err(mlua::Error::external("root window not available"))
        }
    }

    pub(crate) fn root_active_runtime_id(&self) -> Option<RuntimeId> {
        host_runtime::root_active_runtime_id()
    }

    pub(crate) fn root_active_tab_info(&self) -> mlua::Result<Option<RuntimeEntryInfo>> {
        Ok(self.root_active_runtime_id().and_then(|runtime_id| {
            self.tab_info_by_ref(RootTabRef::from_runtime_id(runtime_id))
                .ok()
        }))
    }

    pub(crate) fn root_active_session(&self) -> mlua::Result<Option<SessionRef>> {
        Ok(self
            .root_active_tab_info()?
            .and_then(|info| info.session_id.map(SessionRef::new)))
    }

    pub(crate) fn root_active_session_id(&self) -> mlua::Result<Option<String>> {
        Ok(self
            .root_active_tab_info()?
            .and_then(|info| info.session_id))
    }

    pub(crate) fn root_active_terminal(&self) -> mlua::Result<Option<TerminalRef>> {
        Ok(self
            .root_active_tab_info()?
            .and_then(|info| info.active_terminal_handle)
            .map(TerminalRef::from_terminal_handle))
    }

    pub(crate) fn panes(&self) -> mlua::Result<Vec<Arc<dyn Pane>>> {
        self.ensure_runtime_available()?;
        Ok(host_runtime::iter_panes())
    }

    pub(crate) fn pane(&self, terminal: TerminalRef) -> mlua::Result<Arc<dyn Pane>> {
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
        func: impl FnOnce(&Arc<dyn Pane>) -> R,
    ) -> mlua::Result<R> {
        let pane = self.pane(terminal)?;
        Ok(func(&pane))
    }

    pub(crate) fn with_pane_result<R>(
        &self,
        terminal: TerminalRef,
        func: impl FnOnce(&Arc<dyn Pane>) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        let pane = self.pane(terminal)?;
        func(&pane)
    }

    pub(crate) fn tab_info_by_ref(&self, tab: RootTabRef) -> mlua::Result<RuntimeEntryInfo> {
        self.runtime_entry_info_by_runtime_id(tab.runtime_id())
            .ok_or_else(|| {
                mlua::Error::external(format!(
                    "session handle {} not found",
                    tab.runtime_id().as_u64()
                ))
            })
    }

    fn session_tab_info(&self, session_id: &str) -> mlua::Result<RuntimeEntryInfo> {
        self.runtime_entry_info_by_session_id(session_id)
            .ok_or_else(|| {
                mlua::Error::external(format!("session '{}' not found in runtime", session_id))
            })
    }

    pub(crate) fn session_active_terminal_instance_id(
        &self,
        session_id: &str,
    ) -> mlua::Result<Option<u64>> {
        self.session_tab_info(session_id)
            .map(|info| info.active_terminal_instance_id)
    }

    pub(crate) fn session_window(&self, session_id: &str) -> mlua::Result<Option<WindowRef>> {
        let info = self.session_tab_info(session_id)?;
        Ok(info
            .active_terminal_handle
            .and_then(|handle| self.terminal_by_handle(handle))
            .map(|_| WindowRef::root()))
    }

    pub(crate) fn session_title(&self, session_id: &str) -> mlua::Result<String> {
        self.session_tab_info(session_id).map(|info| info.title)
    }

    pub(crate) fn set_session_title(&self, session_id: &str, title: &str) -> mlua::Result<()> {
        if host_runtime::set_runtime_entry_title_by_session_id(session_id, title) {
            Ok(())
        } else {
            Err(mlua::Error::external(format!(
                "session '{}' not found in runtime",
                session_id
            )))
        }
    }

    pub(crate) fn active_terminal_for_session(
        &self,
        session_id: &str,
    ) -> mlua::Result<Option<TerminalRef>> {
        self.session_tab_info(session_id).map(|info| {
            info.active_terminal_handle
                .map(TerminalRef::from_terminal_handle)
        })
    }

    pub(crate) fn terminals_for_session(&self, session_id: &str) -> mlua::Result<Vec<TerminalRef>> {
        let handles = host_runtime::runtime_entry_terminal_handles_by_session_id(session_id);
        if handles.is_empty() && !host_runtime::runtime_entry_exists_for_session(session_id) {
            return Err(mlua::Error::external(format!(
                "session '{}' not found in runtime",
                session_id
            )));
        }
        Ok(handles
            .into_iter()
            .map(TerminalRef::from_terminal_handle)
            .collect())
    }

    pub(crate) fn terminal_direction_for_session(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> mlua::Result<Option<TerminalRef>> {
        if !host_runtime::runtime_entry_exists_for_session(session_id) {
            return Err(mlua::Error::external(format!(
                "session '{}' not found in runtime",
                session_id
            )));
        }
        Ok(
            host_runtime::runtime_entry_terminal_handle_in_direction_by_session_id(
                session_id, direction,
            )
            .map(TerminalRef::from_terminal_handle),
        )
    }

    pub(crate) fn set_session_zoomed(&self, session_id: &str, zoomed: bool) -> mlua::Result<bool> {
        host_runtime::set_runtime_entry_zoomed_by_session_id(session_id, zoomed).ok_or_else(|| {
            mlua::Error::external(format!("session '{}' not found in runtime", session_id))
        })
    }

    pub(crate) fn session_terminals_with_info(
        &self,
        session_id: &str,
    ) -> mlua::Result<Vec<SessionTerminalInfo>> {
        let infos = host_runtime::runtime_entry_terminal_infos_by_session_id(session_id);
        if infos.is_empty() && !host_runtime::runtime_entry_exists_for_session(session_id) {
            return Err(mlua::Error::external(format!(
                "session '{}' not found in runtime",
                session_id
            )));
        }
        Ok(infos
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
                terminal: TerminalRef::from_terminal_handle(info.terminal_handle),
                session_id: info.session_id,
                terminal_instance_id: info.terminal_instance_id,
            })
            .collect())
    }

    pub(crate) fn rotate_session_counter_clockwise(&self, session_id: &str) -> mlua::Result<()> {
        if host_runtime::rotate_runtime_entry_counter_clockwise_by_session_id(session_id) {
            Ok(())
        } else {
            Err(mlua::Error::external(format!(
                "session '{}' not found in runtime",
                session_id
            )))
        }
    }

    pub(crate) fn rotate_session_clockwise(&self, session_id: &str) -> mlua::Result<()> {
        if host_runtime::rotate_runtime_entry_clockwise_by_session_id(session_id) {
            Ok(())
        } else {
            Err(mlua::Error::external(format!(
                "session '{}' not found in runtime",
                session_id
            )))
        }
    }

    pub(crate) fn session_size(&self, session_id: &str) -> mlua::Result<engine_term::TerminalSize> {
        self.session_tab_info(session_id).map(|info| info.size)
    }

    pub(crate) fn root_tab_for_terminal(&self, terminal: TerminalRef) -> mlua::Result<RootTabRef> {
        self.runtime_id_for_terminal(terminal)
            .map(RootTabRef::from_runtime_id)
            .ok_or_else(|| {
                mlua::Error::external(format!(
                    "terminal handle {} not found in runtime",
                    terminal.terminal_handle_value()
                ))
            })
    }

    pub(crate) fn terminal_window(&self, terminal: TerminalRef) -> Option<WindowRef> {
        self.root_tab_for_terminal(terminal)
            .ok()
            .map(|_| WindowRef::root())
    }

    pub(crate) fn terminal_session(
        &self,
        terminal: TerminalRef,
    ) -> mlua::Result<Option<SessionRef>> {
        self.with_pane(terminal, |pane| pane_session_id(pane).map(SessionRef::new))
    }

    pub(crate) fn activate_root_tab(&self, tab: RootTabRef) -> mlua::Result<()> {
        let updated = self.focus_root_runtime_id(tab.runtime_id());
        if updated {
            Ok(())
        } else {
            Err(mlua::Error::external(format!(
                "session handle {} is not attached to root window",
                tab.runtime_id().as_u64()
            )))
        }
    }

    pub(crate) fn activate_terminal(&self, terminal: TerminalRef) -> mlua::Result<()> {
        let tab = self.root_tab_for_terminal(terminal)?;
        self.activate_root_tab(tab)?;
        if host_runtime::set_runtime_entry_active_terminal(
            tab.runtime_id(),
            terminal.terminal_handle(),
        ) {
            Ok(())
        } else {
            Err(mlua::Error::external(format!(
                "terminal handle {} not found in runtime",
                terminal.terminal_handle_value()
            )))
        }
    }

    pub(crate) fn activate_session(&self, session: &SessionRef) -> mlua::Result<()> {
        let terminal = self
            .active_terminal_for_session(session.as_str())?
            .ok_or_else(|| {
                mlua::Error::external(format!(
                    "session '{}' has no active terminal",
                    session.as_str()
                ))
            })?;
        self.activate_terminal(terminal)
    }

    pub(crate) fn root_window_spawn_context(
        &self,
    ) -> mlua::Result<(engine_term::TerminalSize, Option<TerminalRef>)> {
        self.ensure_runtime_available()?;
        let (size, pane) = self.root_window_spawn_state();
        let pane = pane.map(TerminalRef::from_terminal_handle);
        Ok((size, pane))
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
        source: SplitSource,
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
        let _ = lua.set_app_data(LuaBridgeHost);
    }
}

fn get_host_for_lua(lua: &Lua) -> mlua::Result<LuaBridgeHost> {
    if let Some(host) = lua.app_data_ref::<LuaBridgeHost>() {
        return Ok((*host).clone());
    }
    get_host()
}

pub(crate) fn pane_metadata_string(pane: &Arc<dyn Pane>, key: &str) -> Option<String> {
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

pub(crate) fn pane_metadata_u64(pane: &Arc<dyn Pane>, key: &str) -> Option<u64> {
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

pub(crate) fn pane_session_id(pane: &Arc<dyn Pane>) -> Option<String> {
    pane_metadata_string(pane, "chatminal_session_id")
}

pub(crate) fn pane_terminal_instance_id(pane: &Arc<dyn Pane>) -> Option<u64> {
    pane_metadata_u64(pane, "chatminal_terminal_instance_id")
}

pub(crate) fn terminal_ref_for_pane(pane: &dyn Pane) -> TerminalRef {
    TerminalRef::from_terminal_handle(terminal_handle_for_pane(pane))
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
                .map(|pane| terminal_ref_for_pane(pane.as_ref()))
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
    use host_runtime::client::ClientId;
    use std::sync::{Arc, Mutex};

    static LUA_BRIDGE_HOST_RUNTIME_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn bind_host_to_lua_registers_vm_local_host() {
        let lua = Lua::new();

        assert!(lua.app_data_ref::<LuaBridgeHost>().is_none());

        bind_host_to_lua(&lua);

        assert!(lua.app_data_ref::<LuaBridgeHost>().is_some());
    }

    #[test]
    fn get_host_for_lua_prefers_bound_vm_host() {
        let lua = Lua::new();
        bind_host_to_lua(&lua);

        assert!(get_host_for_lua(&lua).is_ok());
    }

    #[test]
    fn rename_workspace_accepts_non_root_workspace_rename() {
        let _guard = LUA_BRIDGE_HOST_RUNTIME_TEST_LOCK.lock().unwrap();
        host_runtime::shutdown_host_runtime();

        let mux = host_runtime::initialize_host_runtime(None).expect("init host runtime");
        let client = Arc::new(ClientId::new());
        mux.register_client(Arc::clone(&client));
        mux.replace_identity(Some(Arc::clone(&client)));
        assert!(host_runtime::set_root_window_workspace_name(
            "root-workspace"
        ));
        assert!(host_runtime::set_active_workspace_for_client(
            &client,
            "detached-workspace",
        ));

        let host = LuaBridgeHost;
        assert_eq!(
            host.active_workspace().expect("active workspace"),
            "detached-workspace"
        );
        host.rename_workspace("detached-workspace", "renamed-workspace")
            .expect("rename detached workspace");

        assert_eq!(
            host_runtime::active_workspace_for_client(&client).as_deref(),
            Some("renamed-workspace")
        );
        assert_eq!(
            host.active_workspace().expect("renamed active workspace"),
            "renamed-workspace"
        );
        assert_eq!(
            host_runtime::root_window_workspace_name().as_deref(),
            Some("root-workspace")
        );
        assert_eq!(
            host.root_workspace().expect("root workspace"),
            "root-workspace"
        );

        host_runtime::shutdown_host_runtime();
    }
}
