use config::keyassignment::SessionDirection;

use super::*;
use luahelper::mlua::Value;
use luahelper::{from_lua, to_lua};
use std::sync::Arc;

/// Chatminal session handle for Lua scripts.
/// Identity is the chatminal session_id string — SSH/serial sessions are not represented here.
#[derive(Clone, Debug)]
pub struct SessionRef(pub String);

impl SessionRef {
    /// Resolve to the underlying engine Tab via Mux lookup by chatminal session_id.
    pub fn resolve<'a>(&self, mux: &'a Arc<Mux>) -> mlua::Result<Arc<Tab>> {
        mux.get_tab_by_chatminal_session_id(&self.0)
            .ok_or_else(|| mlua::Error::external(format!("session '{}' not found in runtime", self.0)))
    }
}

impl UserData for SessionRef {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, _this, _: ()| {
            Ok(format!("SessionRef(pid:{})", unsafe { libc::getpid() }))
        });
        // O(1): session_id is the identity itself — no lookup needed.
        methods.add_method("session_id", |_, this, _: ()| {
            Ok(this.0.clone())
        });
        methods.add_method("active_terminal_instance_id", |_, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            Ok(tab
                .get_active_pane()
                .and_then(|pane| pane_terminal_instance_id(&pane)))
        });
        methods.add_method("window", |_, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            // resolve_pane_id gives us the window_id without scanning all windows.
            Ok(tab
                .get_active_pane()
                .and_then(|pane| mux.resolve_pane_id(pane.pane_id()))
                .map(|(window_id, _tab_id)| WindowRef(window_id)))
        });
        methods.add_method("get_title", |_, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            Ok(tab.get_title())
        });
        methods.add_method("set_title", |_, this, title: String| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            Ok(tab.set_title(&title))
        });
        methods.add_method("active_terminal", |_, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            Ok(tab.get_active_pane().map(|pane| TerminalRef(pane.pane_id())))
        });
        methods.add_method("terminals", |_, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            Ok(tab
                .iter_panes_ignoring_zoom()
                .into_iter()
                .map(|info| TerminalRef(info.pane.pane_id()))
                .collect::<Vec<TerminalRef>>())
        });

        methods.add_method("get_terminal_direction", |_, this, direction: Value| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            let panes = tab.iter_panes_ignoring_zoom();

            let dir: SessionDirection = from_lua(direction)?;
            let pane = tab
                .get_pane_direction(dir, true)
                .map(|pane_index| TerminalRef(panes[pane_index].pane.pane_id()));
            Ok(pane)
        });

        methods.add_method("set_zoomed", |_, this, zoomed: bool| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            let was_zoomed = tab.set_zoomed(zoomed);
            Ok(was_zoomed)
        });

        methods.add_method("terminals_with_info", |lua, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;

            let result = lua.create_table()?;
            for (idx, pos) in tab.iter_panes_ignoring_zoom().into_iter().enumerate() {
                let info = LeafInfo {
                    index: pos.index,
                    is_active: pos.is_active,
                    is_zoomed: pos.is_zoomed,
                    left: pos.left,
                    top: pos.top,
                    width: pos.width,
                    pixel_width: pos.pixel_width,
                    height: pos.height,
                    pixel_height: pos.pixel_height,
                };
                let info = luahelper::dynamic_to_lua_value(lua, info.to_dynamic())?;
                match &info {
                    LuaValue::Table(t) => {
                        t.set("terminal", TerminalRef(pos.pane.pane_id()))?;
                        t.set("session_id", pane_session_id(&pos.pane))?;
                        t.set("terminal_instance_id", pane_terminal_instance_id(&pos.pane))?;
                    }
                    _ => {}
                }
                result.set(idx + 1, info)?;
            }

            Ok(result)
        });

        methods.add_method("rotate_counter_clockwise", |_, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            tab.rotate_counter_clockwise();
            Ok(())
        });

        methods.add_method("rotate_clockwise", |_, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            tab.rotate_counter_clockwise();
            Ok(())
        });

        methods.add_method("get_size", |lua, this, _: ()| {
            let mux = get_mux()?;
            let tab = this.resolve(&mux)?;
            to_lua(lua, tab.get_size())
        });

        methods.add_method("activate", move |_lua, this, ()| {
            let mux = Mux::get();
            let tab = this.resolve(&mux)?;

            let pane = tab.get_active_pane().ok_or_else(|| {
                mlua::Error::external(format!("session '{}' has no active terminal", this.0))
            })?;

            let (window_id, tab_id) =
                mux.resolve_pane_id(pane.pane_id()).ok_or_else(|| {
                    mlua::Error::external(format!("active terminal {} not found", pane.pane_id()))
                })?;
            {
                let mut window = mux.get_window_mut(window_id).ok_or_else(|| {
                    mlua::Error::external(format!("window {window_id} not found"))
                })?;
                let tab_idx = window.idx_by_id(tab_id).ok_or_else(|| {
                    mlua::Error::external(format!("session '{}' is not attached to window {window_id}", this.0))
                })?;
                window.save_and_then_set_active(tab_idx);
            }
            Ok(())
        });
    }
}
