use super::*;

#[derive(Clone, Copy, Debug)]
pub struct WindowRef;

impl WindowRef {
    pub fn root() -> Self {
        Self
    }
}

impl UserData for WindowRef {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, _this, _: ()| {
            Ok(format!("WindowRef(root, pid:{})", unsafe {
                libc::getpid()
            }))
        });
        methods.add_method("primary_window_id", |_, _this, _: ()| Ok(root_window_id()));
        methods.add_async_method("gui_window", |lua, _this, _: ()| async move {
            // Weakly bound to the gui module; mux cannot hard-depend
            // on chatminal-desktop, but we can runtime resolve the appropriate module
            let api_mod = get_or_create_module(lua, "chatminal")
                .map_err(|err| mlua::Error::external(format!("{err:#}")))?;
            let gui: mlua::Table = api_mod.get("gui")?;
            let func: mlua::Function = gui.get("gui_window")?;
            func.call_async::<_, mlua::Value>(()).await
        });
        methods.add_method("get_workspace", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            host.root_workspace()
        });
        methods.add_method("set_workspace", |lua, this, new_name: String| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            host.set_root_workspace(&new_name)
        });
        methods.add_async_method(
            "spawn_session",
            |lua, this, spawn: SpawnSession| async move { spawn.spawn(lua, this).await },
        );
        methods.add_method("get_title", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            host.root_title()
        });
        methods.add_method("set_title", |lua, this, title: String| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            host.set_root_title(&title)
        });
        methods.add_method("sessions", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            host.root_sessions()
        });
        methods.add_method("sessions_with_info", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            let result = lua.create_table()?;
            let mut out_index = 0usize;
            for session_record in host.root_sessions_with_info()? {
                let Some(session_id) = session_record.session_id.clone() else {
                    continue;
                };
                let session_ref = SessionRef::new(session_id);
                let session_info = SessionInfo {
                    index: out_index,
                    is_active: session_record.is_active,
                };
                let info = luahelper::dynamic_to_lua_value(lua, session_info.to_dynamic())?;
                if let LuaValue::Table(t) = &info {
                    t.set("session_id", session_ref.to_owned_id())?;
                    t.set("session", session_ref)?;
                    t.set(
                        "active_terminal_instance_id",
                        session_record.active_terminal_instance_id,
                    )?;
                }
                out_index += 1;
                result.set(out_index, info)?;
            }
            Ok(result)
        });
        methods.add_method("active_session", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            host.root_active_session()
        });
        methods.add_method("active_session_id", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            host.root_active_session_id()
        });
        methods.add_method("active_terminal", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let _ = this;
            host.root_active_terminal()
        });
    }
}
