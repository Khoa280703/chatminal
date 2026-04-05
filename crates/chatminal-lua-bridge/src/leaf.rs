use super::*;
use engine_term::{SemanticZone, StableRowIndex};
use luahelper::mlua::LuaSerdeExt;
use luahelper::{dynamic_to_lua_value, from_lua, to_lua};
use mlua::Value;
use std::cmp::Ordering;
use std::sync::Arc;
use termwiz::cell::SemanticType;
use termwiz_funcs::lines_to_escapes;
use url_funcs::Url;

#[derive(Clone, Copy, Debug)]
pub struct TerminalRef(chatminal_runtime::SessionTerminalHandle);

impl TerminalRef {
    pub const fn from_terminal_handle(
        terminal_handle: chatminal_runtime::SessionTerminalHandle,
    ) -> Self {
        Self(terminal_handle)
    }

    pub const fn terminal_handle(self) -> chatminal_runtime::SessionTerminalHandle {
        self.0
    }

    pub const fn terminal_handle_value(self) -> u64 {
        self.0.as_u64()
    }
}

fn text_from_semantic_zone(pane: &Arc<dyn LuaPane>, zone: SemanticZone) -> String {
    let mut last_was_wrapped = false;
    let first_row = zone.start_y;
    let last_row = zone.end_y;

    fn cols_for_row(zone: &SemanticZone, row: StableRowIndex) -> std::ops::Range<usize> {
        if row < zone.start_y || row > zone.end_y {
            0..0
        } else if zone.start_y == zone.end_y {
            if zone.start_x <= zone.end_x {
                zone.start_x..zone.end_x.saturating_add(1)
            } else {
                zone.end_x..zone.start_x.saturating_add(1)
            }
        } else if row == zone.end_y {
            0..zone.end_x.saturating_add(1)
        } else if row == zone.start_y {
            zone.start_x..usize::MAX
        } else {
            0..usize::MAX
        }
    }

    let mut s = String::new();
    for line in pane.get_logical_lines(zone.start_y..zone.end_y + 1) {
        if !s.is_empty() && !last_was_wrapped {
            s.push('\n');
        }
        let last_idx = line.physical_lines.len().saturating_sub(1);
        for (idx, phys) in line.physical_lines.iter().enumerate() {
            let this_row = line.first_row + idx as StableRowIndex;
            if this_row >= first_row && this_row <= last_row {
                let last_phys_idx = phys.len().saturating_sub(1);

                let cols = cols_for_row(&zone, this_row);
                let last_col_idx = cols.end.saturating_sub(1).min(last_phys_idx);
                let col_span = phys.columns_as_str(cols);
                if idx == last_idx {
                    s.push_str(col_span.trim_end());
                } else {
                    s.push_str(&col_span);
                }

                last_was_wrapped = last_col_idx == last_phys_idx
                    && phys
                        .get_cell(last_col_idx)
                        .map(|c| c.attrs().wrapped())
                        .unwrap_or(false);
            }
        }
    }

    s
}

impl TerminalRef {
    fn get_text_from_semantic_zone(&self, lua: &Lua, zone: SemanticZone) -> mlua::Result<String> {
        let host = get_host_for_lua(lua)?;
        host.with_pane(*self, |pane| text_from_semantic_zone(pane, zone))
    }
}

impl UserData for TerminalRef {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, _this, _: ()| {
            Ok(format!("TerminalRef(pid:{})", unsafe { libc::getpid() }))
        });
        methods.add_method("terminal_instance_id", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane_terminal_instance_id(pane))
        });
        methods.add_method("session_id", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane_session_id(pane))
        });

        methods.add_async_method(
            "split",
            |lua, this, args: Option<SplitSession>| async move {
                args.unwrap_or_default().run(lua, this).await
            },
        );

        methods.add_method("send_paste", |lua, this, text: String| {
            let host = get_host_for_lua(lua)?;
            host.with_pane_result(*this, |pane| {
                pane.send_paste(&text)
                    .map_err(|e| mlua::Error::external(format!("{:#}", e)))?;
                Ok(())
            })
        });

        methods.add_method("paste", |lua, this, text: String| {
            let host = get_host_for_lua(lua)?;
            host.with_pane_result(*this, |pane| {
                pane.send_paste(&text)
                    .map_err(|e| mlua::Error::external(format!("{:#}", e)))?;
                Ok(())
            })
        });

        methods.add_method("send_text", |lua, this, text: String| {
            let host = get_host_for_lua(lua)?;
            host.with_pane_result(*this, |pane| {
                pane.send_text(&text)
                    .map_err(|e| mlua::Error::external(format!("{:#}", e)))?;
                Ok(())
            })
        });
        methods.add_method("window", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            Ok(host.terminal_window(*this))
        });
        methods.add_method("session", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.terminal_session(*this)
        });

        methods.add_method("get_title", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane.get_title())
        });

        methods.add_method("get_progress", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let progress = host.with_pane(*this, |pane| pane.get_progress())?;
            lua.to_value(&progress)
        });

        methods.add_method("get_current_working_dir", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| {
                pane.get_current_working_dir(PaneCachePolicy::FetchImmediate)
                    .map(|url| Url { url })
            })
        });

        methods.add_method("get_metadata", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            let value = host.with_pane(*this, |pane| pane.get_metadata())?;
            dynamic_to_lua_value(lua, value)
        });

        methods.add_method("get_foreground_process_name", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| {
                pane.get_foreground_process_name(PaneCachePolicy::FetchImmediate)
            })
        });

        methods.add_method("get_foreground_process_info", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| {
                pane.get_foreground_process_info(PaneCachePolicy::AllowStale)
            })
        });

        methods.add_method("get_cursor_position", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane.get_cursor_position())
        });

        methods.add_method("get_dimensions", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane.get_dimensions())
        });

        methods.add_method("get_user_vars", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane.copy_user_vars())
        });

        methods.add_method("has_unseen_output", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane.has_unseen_output())
        });

        methods.add_method("is_alt_screen_active", |lua, this, _: ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane.is_alt_screen_active())
        });

        // When called with no arguments, returns the lines from the
        // viewport as plain text (no escape sequences).
        // When called with an optional integer argument, returns the
        // last nlines lines of the terminal output.
        // The returned string will have trailing whitespace trimmed.
        methods.add_method("get_lines_as_text", |lua, this, nlines: Option<usize>| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| {
                let dims = pane.get_dimensions();
                let nlines = nlines.unwrap_or(dims.viewport_rows);
                let bottom_row = dims.physical_top + dims.viewport_rows as isize;
                let top_row = bottom_row.saturating_sub(nlines as isize);
                let (_first_row, lines) = pane.get_lines(top_row..bottom_row);
                let mut text = String::new();
                for line in lines {
                    for cell in line.visible_cells() {
                        text.push_str(cell.str());
                    }
                    let trimmed = text.trim_end().len();
                    text.truncate(trimmed);
                    text.push('\n');
                }
                let trimmed = text.trim_end().len();
                text.truncate(trimmed);
                text
            })
        });

        methods.add_method(
            "get_lines_as_escapes",
            |lua, this, nlines: Option<usize>| {
                let host = get_host_for_lua(lua)?;
                host.with_pane_result(*this, |pane| {
                    let dims = pane.get_dimensions();
                    let nlines = nlines.unwrap_or(dims.viewport_rows);
                    let bottom_row = dims.physical_top + dims.viewport_rows as isize;
                    let top_row = bottom_row.saturating_sub(nlines as isize);
                    let (_first_row, lines) = pane.get_lines(top_row..bottom_row);
                    let text = lines_to_escapes(lines).map_err(mlua::Error::external)?;
                    Ok(text)
                })
            },
        );

        methods.add_method(
            "get_logical_lines_as_text",
            |lua, this, nlines: Option<usize>| {
                let host = get_host_for_lua(lua)?;
                host.with_pane(*this, |pane| {
                    let dims = pane.get_dimensions();
                    let nlines = nlines.unwrap_or(dims.viewport_rows);
                    let bottom_row = dims.physical_top + dims.viewport_rows as isize;
                    let top_row = bottom_row.saturating_sub(nlines as isize);
                    let lines = pane.get_logical_lines(top_row..bottom_row);
                    let mut text = String::new();
                    for line in lines {
                        for cell in line.logical.visible_cells() {
                            text.push_str(cell.str());
                        }
                        let trimmed = text.trim_end().len();
                        text.truncate(trimmed);
                        text.push('\n');
                    }
                    let trimmed = text.trim_end().len();
                    text.truncate(trimmed);
                    text
                })
            },
        );

        methods.add_method("inject_output", |lua, this, text: String| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| {
                let mut parser = termwiz::escape::parser::Parser::new();
                let mut actions = vec![];
                parser.parse(text.as_bytes(), |action| actions.push(action));
                pane.perform_actions(actions);
            })?;
            Ok(())
        });

        methods.add_method("get_semantic_zones", |lua, this, of_type: Value| {
            let host = get_host_for_lua(lua)?;
            let of_type: Option<SemanticType> = from_lua(of_type)?;
            let mut zones = host.with_pane_result(*this, |pane| {
                pane.get_semantic_zones()
                    .map_err(|e| mlua::Error::external(format!("{:#}", e)))
            })?;

            if let Some(of_type) = of_type {
                zones.retain(|zone| zone.semantic_type == of_type);
            }

            let zones = to_lua(lua, zones)?;
            Ok(zones)
        });

        methods.add_method(
            "get_semantic_zone_at",
            |lua, this, (x, y): (usize, StableRowIndex)| {
                let host = get_host_for_lua(lua)?;
                let zones = host.with_pane(*this, |pane| {
                    pane.get_semantic_zones().unwrap_or_else(|_| vec![])
                })?;

                fn find_zone(x: usize, y: StableRowIndex, zone: &SemanticZone) -> Ordering {
                    match zone.start_y.cmp(&y) {
                        Ordering::Greater => return Ordering::Greater,
                        // If the zone starts on the same line then check that the
                        // x position is within bounds
                        Ordering::Equal => match zone.start_x.cmp(&x) {
                            Ordering::Greater => return Ordering::Greater,
                            Ordering::Equal | Ordering::Less => {}
                        },
                        Ordering::Less => {}
                    }
                    match zone.end_y.cmp(&y) {
                        Ordering::Less => Ordering::Less,
                        // If the zone ends on the same line then check that the
                        // x position is within bounds
                        Ordering::Equal => match zone.end_x.cmp(&x) {
                            Ordering::Less => Ordering::Less,
                            Ordering::Equal | Ordering::Greater => Ordering::Equal,
                        },
                        Ordering::Greater => Ordering::Equal,
                    }
                }

                match zones.binary_search_by(|zone| find_zone(x, y, zone)) {
                    Ok(idx) => {
                        let zone = to_lua(lua, zones[idx])?;
                        Ok(Some(zone))
                    }
                    Err(_) => Ok(None),
                }
            },
        );

        methods.add_method("get_text_from_semantic_zone", |lua, this, zone: Value| {
            let zone: SemanticZone = from_lua(zone)?;
            this.get_text_from_semantic_zone(lua, zone)
        });

        methods.add_method("get_text_from_region", |lua, this, (start_x, start_y, end_x, end_y): (usize, StableRowIndex, usize, StableRowIndex)| {
            let zone = SemanticZone {
                start_x,
                start_y,
                end_x,
                end_y,
                // semantic_type is not used by get_text_from_semantic_zone
                semantic_type: SemanticType::Output,
            };
            this.get_text_from_semantic_zone(lua, zone)
        });

        methods.add_method("activate", move |lua, this, ()| {
            let host = get_host_for_lua(lua)?;
            host.activate_terminal(*this)
        });

        methods.add_method("get_tty_name", move |lua, this, ()| {
            let host = get_host_for_lua(lua)?;
            host.with_pane(*this, |pane| pane.tty_name())
        });
    }
}

#[derive(Debug, Default, FromDynamic, ToDynamic)]
struct SplitSession {
    #[dynamic(flatten)]
    cmd_builder: CommandBuilderFrag,
    #[dynamic(default)]
    direction: SessionSplitDirection,
    #[dynamic(default)]
    top_level: bool,
    #[dynamic(default = "default_split_size")]
    size: f32,
}
impl_lua_conversion_dynamic!(SplitSession);

fn default_split_size() -> f32 {
    0.5
}

impl SplitSession {
    async fn run(&self, lua: &Lua, pane: &TerminalRef) -> mlua::Result<TerminalRef> {
        let (command, command_dir) = self.cmd_builder.to_command_builder();
        let source = LuaSplitSource::Spawn {
            command,
            command_dir,
        };

        let size = if self.size == 0.0 {
            SplitSize::Percent(50)
        } else if self.size < 1.0 {
            SplitSize::Percent((self.size * 100.).floor() as u8)
        } else {
            SplitSize::Cells(self.size as usize)
        };

        let direction = match self.direction {
            SessionSplitDirection::Right | SessionSplitDirection::Left => {
                SplitDirection::Horizontal
            }
            SessionSplitDirection::Top | SessionSplitDirection::Bottom => SplitDirection::Vertical,
        };

        let request = SplitRequest {
            direction,
            target_is_second: match self.direction {
                SessionSplitDirection::Top | SessionSplitDirection::Left => false,
                SessionSplitDirection::Bottom | SessionSplitDirection::Right => true,
            },
            top_level: self.top_level,
            size,
        };

        let host = get_host_for_lua(lua)?;
        host.split_terminal(*pane, request, source).await
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalRef;
    use chatminal_runtime::SessionTerminalHandle;

    #[test]
    fn terminal_ref_round_trips_terminal_handle() {
        let handle = SessionTerminalHandle::new(42);
        let terminal = TerminalRef::from_terminal_handle(handle);

        assert_eq!(terminal.terminal_handle(), handle);
        assert_eq!(terminal.terminal_handle_value(), 42);
    }
}
