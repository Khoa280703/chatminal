//! GuiWin represents a Gui TermWindow (as opposed to a Mux window) in lua code
use super::luaerr;
use crate::termwindow::TermWindowNotif;
use crate::TermWindow;
use config::keyassignment::{ClipboardCopyDestination, KeyAssignment};
use dynamic::{FromDynamic, ToDynamic};
use luahelper::*;
use mlua::{UserData, UserDataMethods};
use toast_notification::ToastNotification;
use window::{Connection, ConnectionOps, DeadKeyStatus, WindowOps, WindowState};

pub type PrimaryGuiWindowId = u64;

#[derive(Clone)]
pub struct GuiWin {
    pub primary_window_id: PrimaryGuiWindowId,
    pub active_workspace: String,
    pub window: ::window::Window,
}

impl GuiWin {
    pub fn new(term_window: &TermWindow) -> Self {
        let window = term_window.window.clone().unwrap();
        Self {
            window,
            primary_window_id: term_window.primary_host_window_id as PrimaryGuiWindowId,
            active_workspace: term_window.active_workspace_name(),
        }
    }
}

impl UserData for GuiWin {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!(
                "GuiWin(primary_window_id:{}, pid:{})",
                this.primary_window_id,
                unsafe { libc::getpid() }
            ))
        });

        methods.add_method("primary_window_id", |_, this, _: ()| {
            Ok(this.primary_window_id)
        });

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
                toast_notification::show(ToastNotification {
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
            "get_selection_text_for_terminal_handle",
            |_, this, terminal_handle: u64| async move {
                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                        tx.try_send(
                            term_window.selection_text_for_terminal_handle(terminal_handle),
                        )
                        .ok();
                    })));
                let text = rx
                    .recv()
                    .await
                    .map_err(|e| anyhow::anyhow!("{:#}", e))
                    .map_err(luaerr)?
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
        methods.add_async_method("active_view_id", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.active_view_id()).ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method("active_terminal_handle", |_, this, _: ()| async move {
            let (tx, rx) = smol::channel::bounded(1);
            this.window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    tx.try_send(term_window.active_terminal_handle()).ok();
                })));
            let result = rx
                .recv()
                .await
                .map_err(|e| anyhow::anyhow!("{:#}", e))
                .map_err(luaerr)?;

            Ok(result)
        });
        methods.add_async_method(
            "perform_action_for_terminal_handle",
            |_, this, (assignment, terminal_handle): (KeyAssignment, u64)| async move {
                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                        tx.try_send(
                            term_window.perform_assignment_for_terminal_handle(
                                terminal_handle,
                                &assignment,
                            ),
                        )
                        .ok();
                    })));
                let result = rx.recv().await.map_err(mlua::Error::external)?;
                result.map_err(mlua::Error::external)
            },
        );
        methods.add_async_method(
            "perform_action_on_active_terminal_handle",
            |_, this, assignment: KeyAssignment| async move {
                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                        let result = term_window
                            .active_terminal_handle()
                            .ok_or_else(|| anyhow::anyhow!("no active terminal handle"));
                        tx.try_send(result.map_err(|err| err.to_string())).ok();
                    })));
                let terminal_handle = rx.recv().await.map_err(mlua::Error::external)?;
                let terminal_handle = terminal_handle.map_err(mlua::Error::external)?;

                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                        tx.try_send(
                            term_window.perform_assignment_for_terminal_handle(
                                terminal_handle,
                                &assignment,
                            ),
                        )
                        .ok();
                    })));
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
            "get_selection_escapes_for_terminal_handle",
            |_, this, terminal_handle: u64| async move {
                let (tx, rx) = smol::channel::bounded(1);
                this.window
                    .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                        tx.try_send(
                            term_window.selection_escapes_for_terminal_handle(terminal_handle),
                        )
                        .ok();
                    })));
                let result = rx.recv().await.map_err(mlua::Error::external)?;
                result.map_err(mlua::Error::external)
            },
        );
    }
}
