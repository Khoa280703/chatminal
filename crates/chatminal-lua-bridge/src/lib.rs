use config::keyassignment::SpawnTabDomain;
use config::lua::mlua::{self, Lua, UserData, UserDataMethods, Value as LuaValue};
use config::lua::{get_or_create_module, get_or_create_sub_module};
use engine_dynamic::{FromDynamic, ToDynamic, Value};
use engine_term::TerminalSize;
use luahelper::impl_lua_conversion_dynamic;
use mlua::UserDataRef;
use mux::domain::{DomainId, SplitSource};
use mux::pane::{Pane, PaneId};
use mux::tab::{SplitDirection, SplitRequest, SplitSize, Tab, TabId};
use mux::window::{Window, WindowId};
use mux::Mux;
use portable_pty::CommandBuilder;
use std::collections::HashMap;
use std::sync::Arc;

mod domain;
mod leaf;
mod surface;
mod window;

pub use domain::DomainRef;
pub use leaf::LeafRef;
pub use surface::SurfaceRef;
pub use window::WindowRef;

fn get_mux() -> mlua::Result<Arc<Mux>> {
    Mux::try_get().ok_or_else(|| mlua::Error::external("cannot get Mux!?"))
}

pub(crate) fn pane_metadata_u64(pane: &Arc<dyn Pane>, key: &str) -> Option<u64> {
    match pane.get_metadata() {
        Value::Object(obj) => {
            obj.get(&Value::String(key.to_string()))
                .and_then(|value| match value {
                    Value::U64(value) => Some(*value),
                    _ => None,
                })
        }
        _ => None,
    }
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

pub(crate) fn pane_session_id(pane: &Arc<dyn Pane>) -> Option<String> {
    pane_metadata_string(pane, "chatminal_session_id")
}

pub(crate) fn pane_surface_id(pane: &Arc<dyn Pane>) -> Option<u64> {
    pane_metadata_u64(pane, "chatminal_surface_id")
}

pub(crate) fn pane_leaf_id(pane: &Arc<dyn Pane>) -> Option<u64> {
    pane_metadata_u64(pane, "chatminal_leaf_id")
}

pub(crate) fn resolve_pane_by_public_id(mux: &Arc<Mux>, public_id: u64) -> Option<Arc<dyn Pane>> {
    PaneId::try_from(public_id)
        .ok()
        .and_then(|pane_id| mux.get_pane(pane_id))
        .or_else(|| {
            mux.iter_panes()
                .into_iter()
                .find(|pane| pane_leaf_id(pane) == Some(public_id))
        })
}

pub(crate) fn surface_session_id(tab: &Arc<Tab>) -> Option<String> {
    tab.iter_panes()
        .into_iter()
        .find_map(|pos| pane_session_id(&pos.pane))
}

pub(crate) fn surface_public_id(tab: &Arc<Tab>) -> Option<u64> {
    tab.iter_panes()
        .into_iter()
        .find_map(|pos| pane_surface_id(&pos.pane))
}

pub(crate) fn surface_active_leaf_id(tab: &Arc<Tab>) -> Option<u64> {
    tab.get_active_pane().and_then(|pane| pane_leaf_id(&pane))
}

pub(crate) fn resolve_surface_by_id(mux: &Arc<Mux>, surface_id: u64) -> Option<Arc<Tab>> {
    mux.iter_windows().into_iter().find_map(|window_id| {
        mux.get_window(window_id).and_then(|window| {
            window
                .iter()
                .find(|tab| surface_public_id(tab) == Some(surface_id))
                .cloned()
        })
    })
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
        lua.create_function(|_, window_id: WindowId| {
            let mux = get_mux()?;
            let window = WindowRef(window_id);
            let _resolved = window.resolve(&mux)?;
            Ok(window)
        })?,
    )?;

    session_module.set(
        "spawn_window",
        lua.create_async_function(|_, spawn: SpawnWindow| async move { spawn.spawn().await })?,
    )?;

    session_module.set(
        "get_domain",
        lua.create_function(|_, domain: LuaValue| {
            let mux = get_mux()?;
            match domain {
                LuaValue::Nil => Ok(Some(DomainRef(mux.default_domain().domain_id()))),
                LuaValue::String(s) => match s.to_str() {
                    Ok(name) => Ok(mux
                        .get_domain_by_name(name)
                        .map(|dom| DomainRef(dom.domain_id()))),
                    Err(err) => Err(mlua::Error::external(format!(
                        "invalid domain identifier passed to session.get_domain: {err:#}"
                    ))),
                },
                LuaValue::Integer(id) => match TryInto::<DomainId>::try_into(id) {
                    Ok(id) => Ok(mux.get_domain(id).map(|dom| DomainRef(dom.domain_id()))),
                    Err(err) => Err(mlua::Error::external(format!(
                        "invalid domain identifier passed to session.get_domain: {err:#}"
                    ))),
                },
                _ => Err(mlua::Error::external(
                    "invalid domain identifier passed to session.get_domain".to_string(),
                )),
            }
        })?,
    )?;

    session_module.set(
        "all_domains",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            Ok(mux
                .iter_domains()
                .into_iter()
                .map(|dom| DomainRef(dom.domain_id()))
                .collect::<Vec<DomainRef>>())
        })?,
    )?;

    session_module.set(
        "set_default_domain",
        lua.create_function(|_, domain: UserDataRef<DomainRef>| {
            let mux = get_mux()?;
            let domain = domain.resolve(&mux)?;
            mux.set_default_domain(&domain);
            Ok(())
        })?,
    )?;

    session_module.set(
        "get_surface",
        lua.create_function(|_, surface_id: u64| {
            let mux = get_mux()?;
            let tab = resolve_surface_by_id(&mux, surface_id).ok_or_else(|| {
                mlua::Error::external(format!("surface id {surface_id} not found"))
            })?;
            Ok(SurfaceRef(tab.tab_id()))
        })?,
    )?;

    session_module.set(
        "get_leaf",
        lua.create_function(|_, leaf_id: u64| {
            let mux = get_mux()?;
            let pane = resolve_pane_by_public_id(&mux, leaf_id)
                .ok_or_else(|| mlua::Error::external(format!("leaf id {leaf_id} not found")))?;
            Ok(LeafRef(pane.pane_id()))
        })?,
    )?;

    session_module.set(
        "all_windows",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            Ok(mux
                .iter_windows()
                .into_iter()
                .map(WindowRef)
                .collect::<Vec<WindowRef>>())
        })?,
    )?;

    session_module.set(
        "all_surfaces",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            let mut surfaces = vec![];
            for window_id in mux.iter_windows() {
                if let Some(window) = mux.get_window(window_id) {
                    for tab in window.iter() {
                        if surface_public_id(tab).is_some() {
                            surfaces.push(SurfaceRef(tab.tab_id()));
                        }
                    }
                }
            }
            Ok(surfaces)
        })?,
    )?;

    session_module.set(
        "all_leaves",
        lua.create_function(|_, _: ()| {
            let mux = get_mux()?;
            Ok(mux
                .iter_panes()
                .into_iter()
                .filter(|pane| pane_leaf_id(pane).is_some())
                .map(|pane| LeafRef(pane.pane_id()))
                .collect::<Vec<LeafRef>>())
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
enum HandySplitDirection {
    Left,
    Right,
    Top,
    Bottom,
}
impl_lua_conversion_dynamic!(HandySplitDirection);

impl Default for HandySplitDirection {
    fn default() -> Self {
        Self::Right
    }
}

#[derive(Debug, FromDynamic, ToDynamic)]
struct SpawnWindow {
    #[dynamic(default = "spawn_surface_default_domain")]
    domain: SpawnTabDomain,
    width: Option<usize>,
    height: Option<usize>,
    workspace: Option<String>,
    position: Option<config::GuiPosition>,
    #[dynamic(flatten)]
    cmd_builder: CommandBuilderFrag,
}
impl_lua_conversion_dynamic!(SpawnWindow);

fn spawn_surface_default_domain() -> SpawnTabDomain {
    SpawnTabDomain::DefaultDomain
}

impl SpawnWindow {
    async fn spawn(self) -> mlua::Result<(SurfaceRef, LeafRef, WindowRef)> {
        let mux = get_mux()?;

        let size = match (self.width, self.height) {
            (Some(cols), Some(rows)) => TerminalSize {
                rows,
                cols,
                ..Default::default()
            },
            _ => config::configuration().initial_size(0, None),
        };

        let (cmd_builder, cwd) = self.cmd_builder.to_command_builder();
        let (tab, pane, window_id) = mux
            .spawn_tab_or_window(
                None,
                self.domain,
                cmd_builder,
                cwd,
                size,
                None,
                self.workspace.unwrap_or_else(|| mux.active_workspace()),
                self.position,
            )
            .await
            .map_err(|e| mlua::Error::external(format!("{:#?}", e)))?;

        Ok((
            SurfaceRef(tab.tab_id()),
            LeafRef(pane.pane_id()),
            WindowRef(window_id),
        ))
    }
}

#[derive(Debug, FromDynamic, ToDynamic)]
struct SpawnSurface {
    #[dynamic(default)]
    domain: SpawnTabDomain,
    #[dynamic(flatten)]
    cmd_builder: CommandBuilderFrag,
}
impl_lua_conversion_dynamic!(SpawnSurface);

impl SpawnSurface {
    async fn spawn(self, window: &WindowRef) -> mlua::Result<(SurfaceRef, LeafRef, WindowRef)> {
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

        let (tab, pane, window_id) = mux
            .spawn_tab_or_window(
                Some(window.0),
                self.domain,
                cmd_builder,
                cwd,
                size,
                pane,
                String::new(),
                None, // optional gui window position
            )
            .await
            .map_err(|e| mlua::Error::external(format!("{:#?}", e)))?;

        Ok((
            SurfaceRef(tab.tab_id()),
            LeafRef(pane.pane_id()),
            WindowRef(window_id),
        ))
    }
}

#[derive(Clone, FromDynamic, ToDynamic)]
struct SurfaceInfo {
    pub index: usize,
    pub is_active: bool,
}
impl_lua_conversion_dynamic!(SurfaceInfo);

#[derive(Clone, FromDynamic, ToDynamic)]
struct LeafInfo {
    /// Topological leaf index that can be used to reference this leaf within the surface.
    pub index: usize,
    /// True if this leaf is active at the time the position was computed.
    pub is_active: bool,
    /// True if this leaf is zoomed.
    pub is_zoomed: bool,
    /// Cell offset from the surface origin to the left edge of this leaf.
    pub left: usize,
    /// Cell offset from the surface origin to the top edge of this leaf.
    pub top: usize,
    /// Width of this leaf in cells.
    pub width: usize,
    pub pixel_width: usize,
    /// Height of this leaf in cells.
    pub height: usize,
    pub pixel_height: usize,
}
impl_lua_conversion_dynamic!(LeafInfo);
