//! GuiWin represents a Gui TermWindow (as opposed to a Mux window) in lua code
use super::luaerr;
use crate::termwindow::TermWindowNotif;
use crate::TermWindow;
use config::keyassignment::{ClipboardCopyDestination, KeyAssignment};
use engine_dynamic::{FromDynamic, ToDynamic};
use engine_toast_notification::ToastNotification;
use luahelper::*;
use mlua::{UserData, UserDataMethods};
use window::{Connection, ConnectionOps, DeadKeyStatus, WindowOps, WindowState};

pub type DesktopWindowId = u64;

#[derive(Clone)]
pub struct GuiWin {
    pub window_id: DesktopWindowId,
    pub active_workspace: String,
    pub window: ::window::Window,
}

impl GuiWin {
    pub fn new(term_window: &TermWindow) -> Self {
        let window = term_window.window.clone().unwrap();
        Self {
            window,
            window_id: term_window.window_id as DesktopWindowId,
            active_workspace: term_window.active_workspace_name(),
        }
    }
}

impl UserData for GuiWin {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!(
                "GuiWin(window_id:{}, pid:{})",
                this.window_id,
                unsafe { libc::getpid() }
            ))
        });

        methods.add_method("window_id", |_, this, _: ()| Ok(this.window_id));

        methods.add_method(
            "set_inner_size",
            |_, this, (width, height): (usize, usize)| {
                this.window
                    .notify(TermWindowNotif::SetInnerSize { width, height });
                Ok(())
            },
        );
        methods.add_method("set_position", |_, this, (x, y): (isize, isize)| {
            this.window.set_window_position(euclid::point2(x, y));
            Ok(())
        });
        methods.add_method("maximize", |_, this, _: ()| {
            this.window.maximize();
            Ok(())
        });
        methods.add_method("restore", |_, this, _: ()| {
            this.window.restore();
            Ok(())
        });
        methods.add_method("toggle_fullscreen", |_, this, _: ()| {
            this.window.toggle_fullscreen();
            Ok(())
        });
        methods.add_method("focus", |_, this, _: ()| {
            this.window.focus();
            Ok(())
        });
        methods.add_method(
            "toast_notification",
            |_, _, (title, message, url, timeout): (String, String, Option<String>, Option<u64>)| {
                engine_toast_notification::show(ToastNotification {
                    title,
                    message,
                    url,
                    timeout: timeout.map(std::time::Duration::from_millis)
                });
                Ok(())
            },
        );
        methods.add_method("get_appearance", |_, _, _: ()| {
            Ok(Connection::get().unwrap().get_appearance().to_string())
        });
        methods.add_method("set_right_status", |_, this, status: String| {
            this.window.notify(TermWindowNotif::SetRightStatus(status));
            Ok(())
        });
        methods.add_method("set_left_status", |_, this, status: String| {
            this.window.notify(TermWindowNotif::SetLeftStatus(status));
            Ok(())
        });
        methods.add_async_method("get_dimensions", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window.notify(TermWindowNotif::GetDimensions(tx));
            let (dims, window_state) = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            #[derive(FromDynamic, ToDynamic)]
            struct Dims {
                pixel_width: usize,
                pixel_height: usize,
                dpi: usize,
                is_full_screen: bool,
            }
            impl_lua_conversion_dynamic!(Dims);

            let dims = Dims {
                pixel_width: dims.pixel_width,
                pixel_height: dims.pixel_height,
                dpi: dims.dpi,
                is_full_screen: window_state.contains(WindowState::FULL_SCREEN),
                // FIXME: expose other states here
            };
            Ok(dims)
        });
        methods.add_async_method(
            "get_selection_text_for_leaf_id",
            |_, this, leaf_id: u64| async move {
                let (tx, rx) = smol::channel::bounded(1);
                this.window.notify(TermWindowNotif::GetSelectionForLeafId {
                    leaf_id,
                    tx,
                });
                let text = rx
                    .recv()
                    .await
                    .map_err(|e| anyhow::anyhow!("{:#}", e))
                    .map_err(luaerr)?;

                Ok(text)
            },
        );
        methods.add_async_method("current_event", |lua, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.current_event.to_dynamic()).ok();
                })));
            let result = rx.recv().await.map_err(mlua::Error::external)?;
            luahelper::dynamic_to_lua_value(lua, result)
        });
        methods.add_async_method("active_session_id", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.active_session_id()).ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method("active_surface_id", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.active_surface_id().map(|id| id.as_u64()))
                        .ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method("active_leaf_id", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.active_leaf_id().map(|id| id.as_u64()))
                        .ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method(
            "perform_action_for_leaf_id",
            |_, this, (assignment, leaf_id): (KeyAssignment, u64)| async move {
                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::PerformAssignmentForLeafId {
                        leaf_id,
                        assignment,
                        tx: Some(tx),
                    });
                let result = rx.recv().await.map_err(mlua::Error::external)?;
                result.map_err(mlua::Error::external)
            },
        );
        methods.add_async_method(
            "perform_action_on_active_leaf",
            |_, this, assignment: KeyAssignment| async move {
                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                        let result = term_window
                            .active_leaf_id()
                            .map(|id| id.as_u64())
                            .ok_or_else(|| anyhow::anyhow!("no active leaf"));
                        tx.try_send(result.map_err(|err| err.to_string())).ok();
                    })));
                let leaf_id = rx.recv().await.map_err(mlua::Error::external)?;
                let leaf_id = leaf_id.map_err(mlua::Error::external)?;

                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::PerformAssignmentForLeafId {
                        leaf_id,
                        assignment,
                        tx: Some(tx),
                    });
                let result = rx.recv().await.map_err(mlua::Error::external)?;
                result.map_err(mlua::Error::external)
            },
        );
        methods.add_async_method("effective_config", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window.notify(TermWindowNotif::GetEffectiveConfig(tx));
            let config = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok((*config).clone())
        });
        methods.add_async_method("get_config_overrides", |lua, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window.notify(TermWindowNotif::GetConfigOverrides(tx));
            let overrides = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            dynamic_to_lua_value(lua, overrides)
        });
        methods.add_method("set_config_overrides", |_, this, value: mlua::Value| {
            let value = lua_value_to_dynamic(value)?;
            this.window
                .notify(TermWindowNotif::SetConfigOverrides(value));
            Ok(())
        });
        methods.add_async_method("is_focused", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.focused.is_some()).ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method("leader_is_active", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.leader_is_active()).ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method("composition_status", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(match term_window.composition_status() {
                        DeadKeyStatus::None => None,
                        DeadKeyStatus::Composing(s) => Some(s.clone()),
                    })
                    .ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method("active_key_table", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.current_key_table_name()).ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method("keyboard_modifiers", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.current_modifier_and_led_state())
                        .ok();
                })));
            let (mods, leds) = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok((mods.to_string(), leds.to_string()))
        });
        methods.add_method("active_workspace", |_, this, _: ()| {
            Ok(this.active_workspace.clone())
        });
        methods.add_method(
            "copy_to_clipboard",
            |_, this, (text, clipboard): (String, Option<ClipboardCopyDestination>)| {
                let clipboard = clipboard.unwrap_or_default();
                this.window
                    .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                        term_window.copy_to_clipboard(clipboard, text);
                    })));
                Ok(())
            },
        );
        methods.add_async_method(
            "get_selection_escapes_for_leaf_id",
            |_, this, leaf_id: u64| async move {
                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::GetSelectionEscapesForLeafId {
                        leaf_id,
                        tx,
                    });
                let result = rx.recv().await.map_err(mlua::Error::external)?;
                result.map_err(mlua::Error::external)
            },
        );
    }
}
