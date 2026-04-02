use config::keyassignment::SessionDirection;

use super::*;
use luahelper::mlua::Value;
use luahelper::{from_lua, to_lua};

/// Chatminal session handle for Lua scripts.
/// Identity is the chatminal session_id string — SSH/serial sessions are not represented here.
#[derive(Clone, Debug)]
pub struct SessionRef(String);

impl SessionRef {
    pub(crate) fn new(session_id: String) -> Self {
        Self(session_id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_owned_id(&self) -> String {
        self.0.clone()
    }
}

impl UserData for SessionRef {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, _this, _: ()| {
            Ok(format!("SessionRef(pid:{})", unsafe { libc::getpid() }))
        });
        // O(1): session_id is the identity itself — no lookup needed.
        methods.add_method("session_id", |_, this, _: ()| Ok(this.to_owned_id()));
        methods.add_method("active_terminal_instance_id", |_, this, _: ()| {
            let host = get_host()?;
            host.session_active_terminal_instance_id(this.as_str())
        });
        methods.add_method("window", |_, this, _: ()| {
            let host = get_host()?;
            host.session_window(this.as_str())
        });
        methods.add_method("get_title", |_, this, _: ()| {
            let host = get_host()?;
            host.session_title(this.as_str())
        });
        methods.add_method("set_title", |_, this, title: String| {
            let host = get_host()?;
            host.set_session_title(this.as_str(), &title)
        });
        methods.add_method("active_terminal", |_, this, _: ()| {
            let host = get_host()?;
            host.active_terminal_for_session(this.as_str())
        });
        methods.add_method("terminals", |_, this, _: ()| {
            let host = get_host()?;
            host.terminals_for_session(this.as_str())
        });

        methods.add_method("get_terminal_direction", |_, this, direction: Value| {
            let host = get_host()?;
            let dir: SessionDirection = from_lua(direction)?;
            host.terminal_direction_for_session(this.as_str(), dir)
        });

        methods.add_method("set_zoomed", |_, this, zoomed: bool| {
            let host = get_host()?;
            host.set_session_zoomed(this.as_str(), zoomed)
        });

        methods.add_method("terminals_with_info", |lua, this, _: ()| {
            let host = get_host()?;
            let terminals = host.session_terminals_with_info(this.as_str())?;
            let result = lua.create_table()?;
            for (idx, terminal_info) in terminals.into_iter().enumerate() {
                let info = luahelper::dynamic_to_lua_value(lua, terminal_info.leaf.to_dynamic())?;
                match &info {
                    LuaValue::Table(t) => {
                        t.set("terminal", terminal_info.terminal)?;
                        t.set("session_id", terminal_info.session_id)?;
                        t.set("terminal_instance_id", terminal_info.terminal_instance_id)?;
                    }
                    _ => {}
                }
                result.set(idx + 1, info)?;
            }

            Ok(result)
        });

        methods.add_method("rotate_counter_clockwise", |_, this, _: ()| {
            let host = get_host()?;
            host.rotate_session_counter_clockwise(this.as_str())
        });

        methods.add_method("rotate_clockwise", |_, this, _: ()| {
            let host = get_host()?;
            host.rotate_session_clockwise(this.as_str())
        });

        methods.add_method("get_size", |lua, this, _: ()| {
            let host = get_host()?;
            to_lua(lua, host.session_size(this.as_str())?)
        });

        methods.add_method("activate", move |_lua, this, ()| {
            let host = get_host()?;
            host.activate_session(this)
        });
    }
}
