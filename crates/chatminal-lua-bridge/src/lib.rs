use chatminal_runtime::RuntimeId;
use config::keyassignment::SessionDirection;
use config::lua::mlua::{self, Lua, UserData, UserDataMethods, Value as LuaValue};
use config::lua::{get_or_create_module, get_or_create_sub_module};
use engine_dynamic::{FromDynamic, ToDynamic, Value};
use host_runtime::pane::Pane;
use host_runtime::spawn_target::SplitSource;
use host_runtime::tab::{SplitDirection, SplitRequest, SplitSize, Tab};
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

    pub(crate) fn active_workspace(&self) -> mlua::Result<String> {
        host_runtime::root_window_workspace_name()
            .ok_or_else(|| mlua::Error::external("root window not available"))
    }

    pub(crate) fn workspace_names(&self) -> mlua::Result<Vec<String>> {
        if !host_runtime::is_host_runtime_available() {
            return Err(mlua::Error::external("root window not available"));
        }
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
        let workspaces = self.workspace_names()?;
        if workspaces
            .iter()
            .any(|workspace| workspace == new_workspace)
        {
            Ok(())
        } else {
            Err(mlua::Error::external(format!(
                "failed to rename workspace '{old_workspace}' to '{new_workspace}'"
            )))
        }
    }

    pub(crate) fn root_tab_infos(&self) -> mlua::Result<Vec<RuntimeEntryInfo>> {
        if !host_runtime::is_host_runtime_available() {
            return Err(mlua::Error::external("root window not available"));
        }
        Ok(host_runtime::root_runtime_entry_infos())
    }

    pub(crate) fn panes(&self) -> mlua::Result<Vec<Arc<dyn Pane>>> {
        if !host_runtime::is_host_runtime_available() {
            return Err(mlua::Error::external("root window not available"));
        }
        Ok(host_runtime::iter_panes())
    }

    pub(crate) fn pane(&self, terminal: TerminalRef) -> mlua::Result<Arc<dyn Pane>> {
        host_runtime::terminal_by_handle(terminal.terminal_handle()).ok_or_else(|| {
            mlua::Error::external(format!(
                "terminal handle {} not found in runtime",
                terminal.pane_id()
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

    pub(crate) fn tab_by_ref(&self, tab: RootTabRef) -> mlua::Result<Arc<Tab>> {
        host_runtime::runtime_entry_by_runtime_id(tab.runtime_id()).ok_or_else(|| {
            mlua::Error::external(format!(
                "session handle {} not found",
                tab.runtime_id().as_u64()
            ))
        })
    }

    pub(crate) fn tab_info_by_ref(&self, tab: RootTabRef) -> mlua::Result<RuntimeEntryInfo> {
        host_runtime::runtime_entry_info_by_runtime_id(tab.runtime_id()).ok_or_else(|| {
            mlua::Error::external(format!(
                "session handle {} not found",
                tab.runtime_id().as_u64()
            ))
        })
    }

    pub(crate) fn tab_by_session_id(&self, session_id: &str) -> mlua::Result<Arc<Tab>> {
        host_runtime::runtime_entry_by_session_id(session_id).ok_or_else(|| {
            mlua::Error::external(format!("session '{}' not found in runtime", session_id))
        })
    }

    pub(crate) fn session_active_terminal_instance_id(
        &self,
        session_id: &str,
    ) -> mlua::Result<Option<u64>> {
        host_runtime::runtime_entry_info_by_session_id(session_id)
            .map(|info| info.active_terminal_instance_id)
            .ok_or_else(|| {
                mlua::Error::external(format!("session '{}' not found in runtime", session_id))
            })
    }

    pub(crate) fn session_window(&self, session_id: &str) -> mlua::Result<Option<WindowRef>> {
        let info = host_runtime::runtime_entry_info_by_session_id(session_id).ok_or_else(|| {
            mlua::Error::external(format!("session '{}' not found in runtime", session_id))
        })?;
        Ok(info
            .active_terminal_handle
            .and_then(|handle| {
                self.root_tab_for_terminal(TerminalRef::from_terminal_handle(handle))
                    .ok()
            })
            .map(|_| WindowRef::root()))
    }

    pub(crate) fn session_title(&self, session_id: &str) -> mlua::Result<String> {
        host_runtime::runtime_entry_info_by_session_id(session_id)
            .map(|info| info.title)
            .ok_or_else(|| {
                mlua::Error::external(format!("session '{}' not found in runtime", session_id))
            })
    }

    pub(crate) fn set_session_title(&self, session_id: &str, title: &str) -> mlua::Result<()> {
        let tab = self.tab_by_session_id(session_id)?;
        tab.set_title(title);
        Ok(())
    }

    pub(crate) fn active_terminal_for_session(
        &self,
        session_id: &str,
    ) -> mlua::Result<Option<TerminalRef>> {
        host_runtime::runtime_entry_info_by_session_id(session_id)
            .map(|info| {
                info.active_terminal_handle
                    .map(TerminalRef::from_terminal_handle)
            })
            .ok_or_else(|| {
                mlua::Error::external(format!("session '{}' not found in runtime", session_id))
            })
    }

    pub(crate) fn terminals_for_session(&self, session_id: &str) -> mlua::Result<Vec<TerminalRef>> {
        let tab = self.tab_by_session_id(session_id)?;
        Ok(tab
            .iter_panes_ignoring_zoom()
            .into_iter()
            .map(|info| {
                TerminalRef::from_terminal_handle(host_runtime::terminal_handle_for_pane(
                    info.pane.as_ref(),
                ))
            })
            .collect())
    }

    pub(crate) fn terminal_direction_for_session(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> mlua::Result<Option<TerminalRef>> {
        let tab = self.tab_by_session_id(session_id)?;
        let panes = tab.iter_panes_ignoring_zoom();
        Ok(tab.get_pane_direction(direction, true).map(|pane_index| {
            TerminalRef::from_terminal_handle(host_runtime::terminal_handle_for_pane(
                panes[pane_index].pane.as_ref(),
            ))
        }))
    }

    pub(crate) fn set_session_zoomed(&self, session_id: &str, zoomed: bool) -> mlua::Result<bool> {
        let tab = self.tab_by_session_id(session_id)?;
        Ok(tab.set_zoomed(zoomed))
    }

    pub(crate) fn session_terminals_with_info(
        &self,
        session_id: &str,
    ) -> mlua::Result<Vec<SessionTerminalInfo>> {
        let tab = self.tab_by_session_id(session_id)?;
        Ok(tab
            .iter_panes_ignoring_zoom()
            .into_iter()
            .map(|pos| SessionTerminalInfo {
                leaf: LeafInfo {
                    index: pos.index,
                    is_active: pos.is_active,
                    is_zoomed: pos.is_zoomed,
                    left: pos.left,
                    top: pos.top,
                    width: pos.width,
                    pixel_width: pos.pixel_width,
                    height: pos.height,
                    pixel_height: pos.pixel_height,
                },
                terminal: TerminalRef::from_terminal_handle(
                    host_runtime::terminal_handle_for_pane(pos.pane.as_ref()),
                ),
                session_id: pane_session_id(&pos.pane),
                terminal_instance_id: pane_terminal_instance_id(&pos.pane),
            })
            .collect())
    }

    pub(crate) fn rotate_session_counter_clockwise(&self, session_id: &str) -> mlua::Result<()> {
        let tab = self.tab_by_session_id(session_id)?;
        tab.rotate_counter_clockwise();
        Ok(())
    }

    pub(crate) fn rotate_session_clockwise(&self, session_id: &str) -> mlua::Result<()> {
        let tab = self.tab_by_session_id(session_id)?;
        tab.rotate_clockwise();
        Ok(())
    }

    pub(crate) fn session_size(&self, session_id: &str) -> mlua::Result<engine_term::TerminalSize> {
        host_runtime::runtime_entry_info_by_session_id(session_id)
            .map(|info| info.size)
            .ok_or_else(|| {
                mlua::Error::external(format!("session '{}' not found in runtime", session_id))
            })
    }

    pub(crate) fn root_tab_for_terminal(&self, terminal: TerminalRef) -> mlua::Result<RootTabRef> {
        host_runtime::resolve_runtime_id_for_terminal_handle(terminal.terminal_handle())
            .map(RootTabRef::from_runtime_id)
            .ok_or_else(|| {
                mlua::Error::external(format!(
                    "terminal handle {} not found in runtime",
                    terminal.pane_id()
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
        let updated = host_runtime::focus_root_runtime_entry(tab.runtime_id());
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
        let pane = self.pane(terminal)?;
        let tab = self.root_tab_for_terminal(terminal)?;
        self.activate_root_tab(tab)?;
        let tab = self.tab_by_ref(tab)?;
        tab.set_active_pane(&pane);
        Ok(())
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
        if !host_runtime::is_host_runtime_available() {
            return Err(mlua::Error::external("root window not available"));
        }
        let (size, pane) = host_runtime::root_window_spawn_context_state();
        let pane = pane.map(TerminalRef::from_terminal_handle);
        Ok((size, pane))
    }

    pub(crate) async fn spawn_session_from_root_window(
        &self,
        cmd_builder: Option<CommandBuilder>,
        cwd: Option<String>,
    ) -> mlua::Result<(SessionRef, TerminalRef)> {
        let (size, pane) = self.root_window_spawn_context()?;
        let (_tab, pane) = host_runtime::spawn_tab(
            cmd_builder,
            cwd,
            size,
            pane.map(TerminalRef::terminal_handle),
        )
        .await
        .map_err(|e| mlua::Error::external(format!("{:#?}", e)))?;
        let terminal_handle = host_runtime::terminal_handle_for_pane(pane.as_ref());
        let terminal_ref = TerminalRef::from_terminal_handle(terminal_handle);
        let session_id = pane_session_id(&pane).ok_or_else(|| {
            mlua::Error::external("spawned session has no chatminal session_id")
        })?;
        Ok((SessionRef::new(session_id), terminal_ref))
    }

    pub(crate) async fn split_pane(
        &self,
        terminal: TerminalRef,
        request: SplitRequest,
        source: SplitSource,
    ) -> mlua::Result<(Arc<dyn Pane>, engine_term::TerminalSize)> {
        host_runtime::split_pane(terminal.terminal_handle(), request, source)
            .await
            .map_err(|e| mlua::Error::external(format!("{:#?}", e)))
    }

    pub(crate) async fn split_terminal(
        &self,
        terminal: TerminalRef,
        request: SplitRequest,
        source: SplitSource,
    ) -> mlua::Result<TerminalRef> {
        let (pane, _size) = self.split_pane(terminal, request, source).await?;
        Ok(TerminalRef::from_terminal_handle(
            host_runtime::terminal_handle_for_pane(pane.as_ref()),
        ))
    }
}

fn get_host() -> mlua::Result<LuaBridgeHost> {
    LuaBridgeHost::global()
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


pub fn register(lua: &Lua) -> anyhow::Result<()> {
    let session_module = get_or_create_sub_module(lua, "session")?;

    session_module.set(
        "get_active_workspace",
        lua.create_function(|_, _: ()| {
            let host = get_host()?;
            host.active_workspace()
        })?,
    )?;

    session_module.set(
        "get_workspace_names",
        lua.create_function(|_, _: ()| {
            let host = get_host()?;
            host.workspace_names()
        })?,
    )?;

    session_module.set(
        "set_active_workspace",
        lua.create_function(|_, workspace: String| {
            let host = get_host()?;
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
        lua.create_function(|_, (old_workspace, new_workspace): (String, String)| {
            let host = get_host()?;
            host.rename_workspace(&old_workspace, &new_workspace)
        })?,
    )?;

    session_module.set(
        "get_window",
        lua.create_function(|_, _: ()| Ok(WindowRef::root()))?,
    )?;

    session_module.set(
        "all_sessions",
        lua.create_function(|_, _: ()| {
            let host = get_host()?;
            Ok(host
                .root_tab_infos()?
                .into_iter()
                .filter_map(|info| info.session_id.map(SessionRef::new))
                .collect::<Vec<SessionRef>>())
        })?,
    )?;

    session_module.set(
        "list_sessions",
        lua.create_function(|_, _: ()| {
            let host = get_host()?;
            Ok(host
                .root_tab_infos()?
                .into_iter()
                .filter_map(|info| info.session_id.map(SessionRef::new))
                .collect::<Vec<SessionRef>>())
        })?,
    )?;

    session_module.set(
        "all_terminals",
        lua.create_function(|_, _: ()| {
            let host = get_host()?;
            Ok(host
                .panes()?
                .into_iter()
                .map(|pane| {
                    TerminalRef::from_terminal_handle(host_runtime::terminal_handle_for_pane(
                        pane.as_ref(),
                    ))
                })
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
    async fn spawn(self, window: &WindowRef) -> mlua::Result<(SessionRef, TerminalRef, WindowRef)> {
        let host = get_host()?;
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
