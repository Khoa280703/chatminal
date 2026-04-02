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
            let _host = get_host()?;
            let _ = this;
            host_runtime::root_window_workspace_name()
                .ok_or_else(|| mlua::Error::external("root window not available"))
        });
        methods.add_method("set_workspace", |_, this, new_name: String| {
            let _host = get_host()?;
            let _ = this;
            if host_runtime::set_root_window_workspace_name(&new_name) {
                Ok(())
            } else {
                Err(mlua::Error::external("root window not available"))
            }
        });
        methods.add_async_method("spawn_session", |_, this, spawn: SpawnSession| async move {
            spawn.spawn(this).await
        });
        methods.add_method("get_title", |_, this, _: ()| {
            let _host = get_host()?;
            let _ = this;
            host_runtime::root_window_title()
                .ok_or_else(|| mlua::Error::external("root window not available"))
        });
        methods.add_method("set_title", |_, this, title: String| {
            let _host = get_host()?;
            let _ = this;
            if host_runtime::set_root_window_title(&title) {
                Ok(())
            } else {
                Err(mlua::Error::external("root window not available"))
            }
        });
        methods.add_method("sessions", |_, this, _: ()| {
            let host = get_host()?;
            let _ = this;
            // Only chatminal sessions; SSH/serial tabs return None from make_session_ref.
            Ok(host
                .root_tab_infos()?
                .into_iter()
                .filter_map(|info| info.session_id.map(SessionRef::new))
                .collect::<Vec<SessionRef>>())
        });
        methods.add_method("sessions_with_info", |lua, this, _: ()| {
            let host = get_host()?;
            let _ = this;
            let result = lua.create_table()?;
            let active_runtime_id = host_runtime::root_active_runtime_id();
            let mut out_index = 0usize;
            for entry_info in host.root_tab_infos()? {
                let Some(session_id) = entry_info.session_id.clone() else {
                    continue;
                };
                let session_ref = SessionRef::new(session_id);
                let session_info = SessionInfo {
                    index: out_index,
                    is_active: active_runtime_id == Some(entry_info.runtime_id),
                };
                let info = luahelper::dynamic_to_lua_value(lua, session_info.to_dynamic())?;
                if let LuaValue::Table(t) = &info {
                    t.set("session_id", session_ref.to_owned_id())?;
                    t.set("session", session_ref)?;
                    t.set(
                        "active_terminal_instance_id",
                        entry_info.active_terminal_instance_id,
                    )?;
                }
                out_index += 1;
                result.set(out_index, info)?;
            }
            Ok(result)
        });
        methods.add_method("active_session", |_, this, _: ()| {
            let host = get_host()?;
            let _ = this;
            host_runtime::root_active_runtime_id()
                .and_then(|runtime_id| {
                    host.tab_info_by_ref(RootTabRef::from_runtime_id(runtime_id))
                        .ok()
                })
                .and_then(|info| info.session_id.map(SessionRef::new))
                .ok_or_else(|| mlua::Error::external("root window not available"))
        });
        methods.add_method("active_session_id", |_, this, _: ()| {
            let host = get_host()?;
            let _ = this;
            Ok(host_runtime::root_active_runtime_id()
                .and_then(|runtime_id| {
                    host.tab_info_by_ref(RootTabRef::from_runtime_id(runtime_id))
                        .ok()
                })
                .and_then(|info| info.session_id))
        });
        methods.add_method("active_terminal", |_, this, _: ()| {
            let host = get_host()?;
            let _ = this;
            Ok(host_runtime::root_active_runtime_id()
                .and_then(|runtime_id| {
                    host.tab_info_by_ref(RootTabRef::from_runtime_id(runtime_id))
                        .ok()
                })
                .and_then(|info| info.active_terminal_handle)
                .and_then(|handle| usize::try_from(handle.as_u64()).ok())
                .map(TerminalRef::from_pane_id))
        });
    }
}
