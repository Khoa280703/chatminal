use super::*;
use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard};

#[derive(Clone, Copy, Debug)]
pub struct WindowRef;

impl WindowRef {
    pub fn root() -> Self {
        Self
    }

    pub fn resolve<'a>(
        &self,
        mux: &'a Arc<Mux>,
    ) -> mlua::Result<MappedRwLockReadGuard<'a, Window>> {
        Ok(mux.root_window())
    }

    pub fn resolve_mut<'a>(
        &self,
        mux: &'a Arc<Mux>,
    ) -> mlua::Result<MappedRwLockWriteGuard<'a, Window>> {
        Ok(mux.root_window_mut())
    }
}

impl UserData for WindowRef {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, _this, _: ()| {
            Ok(format!("WindowRef(root, pid:{})", unsafe {
                libc::getpid()
            }))
        });
        methods.add_method("primary_window_id", |_, _this, _: ()| {
            Ok(host_runtime::window::ROOT_WINDOW_ID)
        });
        methods.add_async_method("gui_window", |lua, _this, _: ()| async move {
            // Weakly bound to the gui module; mux cannot hard-depend
            // on chatminal-desktop, but we can runtime resolve the appropriate module
            let api_mod = get_or_create_module(lua, "chatminal")
                .map_err(|err| mlua::Error::external(format!("{err:#}")))?;
            let gui: mlua::Table = api_mod.get("gui")?;
            let func: mlua::Function = gui.get("gui_window")?;
            func.call_async::<_, mlua::Value>(()).await
        });
        methods.add_method("get_workspace", |_, this, _: ()| {
            let mux = get_mux()?;
            let window = this.resolve(&mux)?;
            Ok(window.get_workspace().to_string())
        });
        methods.add_method("set_workspace", |_, this, new_name: String| {
            let mux = get_mux()?;
            let mut window = this.resolve_mut(&mux)?;
            Ok(window.set_workspace(&new_name))
        });
        methods.add_async_method("spawn_session", |_, this, spawn: SpawnSession| async move {
            spawn.spawn(this).await
        });
        methods.add_method("get_title", |_, this, _: ()| {
            let mux = get_mux()?;
            let window = this.resolve(&mux)?;
            Ok(window.get_title().to_string())
        });
        methods.add_method("set_title", |_, this, title: String| {
            let mux = get_mux()?;
            let mut window = this.resolve_mut(&mux)?;
            Ok(window.set_title(&title))
        });
        methods.add_method("sessions", |_, this, _: ()| {
            let mux = get_mux()?;
            let window = this.resolve(&mux)?;
            // Only chatminal sessions; SSH/serial tabs return None from make_session_ref.
            Ok(window
                .iter()
                .filter_map(|tab| make_session_ref(tab))
                .collect::<Vec<SessionRef>>())
        });
        methods.add_method("sessions_with_info", |lua, this, _: ()| {
            let mux = get_mux()?;
            let window = this.resolve(&mux)?;
            let result = lua.create_table()?;
            let active_idx = window.get_active_idx();
            // out_index tracks position in the filtered (chatminal-only) list.
            // info.index matches this filtered position so Lua consumers see consistent indexing.
            let mut out_index = 0usize;
            for (raw_idx, tab) in window.iter().enumerate() {
                // Skip non-chatminal sessions (SSH/serial have no session_id).
                let Some(session_ref) = make_session_ref(tab) else { continue };
                let info = SessionInfo {
                    index: out_index,
                    is_active: raw_idx == active_idx,
                };
                let info = luahelper::dynamic_to_lua_value(lua, info.to_dynamic())?;
                if let LuaValue::Table(t) = &info {
                    t.set("session_id", session_ref.0.clone())?;
                    t.set("session", session_ref)?;
                    t.set(
                        "active_terminal_instance_id",
                        tab.get_active_pane()
                            .and_then(|pane| pane_terminal_instance_id(&pane)),
                    )?;
                }
                out_index += 1;
                result.set(out_index, info)?;
            }
            Ok(result)
        });
        methods.add_method("active_session", |_, this, _: ()| {
            let mux = get_mux()?;
            let window = this.resolve(&mux)?;
            Ok(window.get_active().and_then(|tab| make_session_ref(tab)))
        });
        methods.add_method("active_session_id", |_, this, _: ()| {
            let mux = get_mux()?;
            let window = this.resolve(&mux)?;
            Ok(window.get_active().and_then(|tab| make_session_ref(tab)).map(|s| s.0))
        });
        methods.add_method("active_terminal", |_, this, _: ()| {
            let mux = get_mux()?;
            let window = this.resolve(&mux)?;
            Ok(window
                .get_active()
                .and_then(|tab| tab.get_active_pane().map(|pane| TerminalRef(pane.pane_id()))))
        });
    }
}
