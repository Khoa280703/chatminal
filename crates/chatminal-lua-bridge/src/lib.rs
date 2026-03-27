use config::lua::mlua::{self, Lua, UserData, UserDataMethods, Value as LuaValue};
use config::lua::{get_or_create_module, get_or_create_sub_module};
use engine_dynamic::{FromDynamic, ToDynamic, Value};
use luahelper::impl_lua_conversion_dynamic;
use host_runtime::pane::Pane;
use host_runtime::spawn_target::SplitSource;
use host_runtime::tab::{SplitDirection, SplitRequest, SplitSize, Tab};
use host_runtime::window::Window;
use host_runtime::Mux;
use portable_pty::CommandBuilder;
use std::collections::HashMap;
use std::sync::Arc;

mod leaf;
mod session;
mod window;

pub use leaf::TerminalRef;
pub use session::SessionRef;
pub use window::WindowRef;

fn get_mux() -> mlua::Result<Arc<Mux>> {
    Mux::try_get().ok_or_else(|| mlua::Error::external("cannot get Mux!?"))
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

pub(crate) fn session_id_for_tab(tab: &Arc<Tab>) -> Option<String> {
    tab.iter_panes()
        .into_iter()
        .find_map(|pos| pane_session_id(&pos.pane))
}

/// Construct a SessionRef from a Tab if it has a chatminal session_id.
/// Returns None for SSH/serial/legacy sessions — Option A: chatminal sessions only.
pub(crate) fn make_session_ref(tab: &Arc<Tab>) -> Option<SessionRef> {
    session_id_for_tab(tab).map(SessionRef)
}

pub fn register(lua: &Lua) -> anyhow::Result<()> {
    let session_module = get_or_create_sub_module(lua, "session")?;

    session_module.set(
        "get_active_workspace",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            Ok(mux.active_workspace())
        })?,
    )?;

    session_module.set(
        "get_workspace_names",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            Ok(mux.iter_workspaces())
        })?,
    )?;

    session_module.set(
        "set_active_workspace",
        lua.create_function(|_, workspace: String| {
            let mux = get_mux()?;
            let workspaces = mux.iter_workspaces();
            if workspaces.contains(&workspace) {
                Ok(mux.set_active_workspace(&workspace))
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
            let mux = get_mux()?;
            mux.rename_workspace(&old_workspace, &new_workspace);
            Ok(())
        })?,
    )?;

    session_module.set(
        "get_window",
        lua.create_function(|_, _: ()| Ok(WindowRef::root()))?,
    )?;

    session_module.set(
        "all_sessions",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            let window = mux.root_window();
            Ok(window
                .iter()
                .filter_map(|tab| make_session_ref(tab))
                .collect::<Vec<SessionRef>>())
        })?,
    )?;

    session_module.set(
        "list_sessions",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            let window = mux.root_window();
            Ok(window
                .iter()
                .filter_map(|tab| make_session_ref(tab))
                .collect::<Vec<SessionRef>>())
        })?,
    )?;

    session_module.set(
        "all_terminals",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            Ok(mux
                .iter_panes()
                .into_iter()
                .map(|pane| TerminalRef(pane.pane_id()))
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
        let mux = get_mux()?;
        let size;
        let pane;

        {
            let window = window.resolve(&mux)?;
            size = window
                .get_by_idx(0)
                .map(|tab| tab.get_size())
                .unwrap_or_else(|| config::configuration().initial_size(0, None));

            pane = window
                .get_active()
                .and_then(|tab| tab.get_active_pane().map(|pane| pane.pane_id()));
        };

        let (cmd_builder, cwd) = self.cmd_builder.to_command_builder();

        let (tab, pane) = mux
            .spawn_tab(cmd_builder, cwd, size, pane)
            .await
            .map_err(|e| mlua::Error::external(format!("{:#?}", e)))?;

        let session_ref = make_session_ref(&tab).ok_or_else(|| {
            mlua::Error::external("spawned session has no chatminal session_id")
        })?;
        Ok((session_ref, TerminalRef(pane.pane_id()), *window))
    }
}

#[derive(Clone, FromDynamic, ToDynamic)]
struct SessionInfo {
    pub index: usize,
    pub is_active: bool,
}
impl_lua_conversion_dynamic!(SessionInfo);

#[derive(Clone, FromDynamic, ToDynamic)]
struct LeafInfo {
    /// Topological leaf index that can be used to reference this leaf within the session tab.
    pub index: usize,
    /// True if this leaf is active at the time the position was computed.
    pub is_active: bool,
    /// True if this leaf is zoomed.
    pub is_zoomed: bool,
    /// Cell offset from the session tab origin to the left edge of this leaf.
    pub left: usize,
    /// Cell offset from the session tab origin to the top edge of this leaf.
    pub top: usize,
    /// Width of this leaf in cells.
    pub width: usize,
    pub pixel_width: usize,
    /// Height of this leaf in cells.
    pub height: usize,
    pub pixel_height: usize,
}
impl_lua_conversion_dynamic!(LeafInfo);
