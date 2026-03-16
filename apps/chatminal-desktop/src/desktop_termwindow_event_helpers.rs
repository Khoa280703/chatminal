impl TermWindow {
    fn emit_status_event(&mut self) {
        self.emit_window_event("update-right-status", None);
        self.emit_window_event("update-status", None);
    }

    fn schedule_window_event(&mut self, name: &str, pane_id: Option<TerminalUiKey>) {
        let window = GuiWin::new(self);
        let pane = match pane_id {
            Some(pane_id) => self.terminal_handle_arc(pane_id),
            None => None,
        };
        let pane = match pane {
            Some(pane) => pane,
            None => match self.active_terminal_instance_or_overlay() {
                Some(pane) => pane,
                None => return,
            },
        };
        let pane_id = pane.pane_id() as u64;
        let name = name.to_string();

        async fn do_event(
            lua: Option<Rc<mlua::Lua>>,
            name: String,
            window: GuiWin,
            pane_id: u64,
        ) -> anyhow::Result<()> {
            let again = if let Some(lua) = lua {
                let args = lua.pack_multi((window.clone(), pane_id))?;

                if let Err(err) = config::lua::emit_event(&lua, (name.clone(), args)).await {
                    log::error!("while processing {} event: {:#}", name, err);
                }
                true
            } else {
                false
            };

            window
                .window
                .notify(TermWindowNotif::FinishWindowEvent { name, again });

            Ok(())
        }

        promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
            do_event(lua, name, window, pane_id)
        }))
        .detach();
    }

    /// Called as part of finishing up a callout to lua.
    /// If again==false it means that there isn't a lua config
    /// to execute against, so we should just mark as done.
    /// Otherwise, if there is a queued item, schedule it now.
    fn finish_window_event(&mut self, name: &str, again: bool) {
        let state = self
            .event_states
            .entry(name.to_string())
            .or_insert(EventState::None);
        if again {
            match state {
                EventState::InProgress => {
                    *state = EventState::None;
                }
                EventState::InProgressWithQueued(pane) => {
                    let pane = *pane;
                    *state = EventState::InProgress;
                    self.schedule_window_event(name, pane);
                }
                EventState::None => {}
            }
        } else {
            *state = EventState::None;
        }
    }

    pub fn emit_window_event(&mut self, name: &str, pane_id: Option<TerminalUiKey>) {
        if self.active_terminal_instance_or_overlay().is_none() || self.window.is_none() {
            return;
        }

        let state = self
            .event_states
            .entry(name.to_string())
            .or_insert(EventState::None);
        match state {
            EventState::InProgress => {
                // Flag that we want to run again when the currently
                // executing event calls finish_window_event().
                *state = EventState::InProgressWithQueued(pane_id);
                return;
            }
            EventState::InProgressWithQueued(other_pane) => {
                // We've already got one copy executing and another
                // pending dispatch, so don't queue another.
                if pane_id != *other_pane {
                    log::warn!(
                        "Cannot queue {} event for pane {:?}, as \
                         there is already an event queued for pane {:?} \
                         in the same window",
                        name,
                        pane_id,
                        other_pane
                    );
                }
                return;
            }
            EventState::None => {
                // Nothing pending, so schedule a call now
                *state = EventState::InProgress;
                self.schedule_window_event(name, pane_id);
            }
        }
    }

    fn check_for_dirty_lines_and_invalidate_selection(&mut self, pane: &Arc<dyn OverlayPane>) {
        let dims = pane.get_dimensions();
        let viewport = self
            .get_viewport(pane.pane_id() as u64)
            .unwrap_or(dims.physical_top);
        let visible_range = viewport..viewport + dims.viewport_rows as StableRowIndex;
        let seqno = self.selection(pane.pane_id() as u64).seqno;
        let dirty = pane.get_changed_since(visible_range, seqno);

        if dirty.is_empty() {
            return;
        }
        if pane.downcast_ref::<CopyOverlay>().is_none()
            && pane.downcast_ref::<QuickSelectOverlay>().is_none()
        {
            // If any of the changed lines intersect with the
            // selection, then we need to clear the selection, but not
            // when the search overlay is active; the search overlay
            // marks lines as dirty to force invalidate them for
            // highlighting purpose but also manipulates the selection
            // and we want to allow it to retain the selection it made!

            let clear_selection =
                if let Some(selection_range) = self.selection(pane.pane_id() as u64).range.as_ref() {
                    let selection_rows = selection_range.rows();
                    selection_rows.into_iter().any(|row| dirty.contains(row))
                } else {
                    false
                };

            if clear_selection {
                self.selection(pane.pane_id() as u64).range.take();
                self.selection(pane.pane_id() as u64).origin.take();
                self.selection(pane.pane_id() as u64).seqno = pane.get_current_seqno();
            }
        }
    }
}

impl TermWindow {
    fn palette(&mut self) -> &ColorPalette {
        if self.palette.is_none() {
            self.palette
                .replace(config::TermConfig::new().color_palette());
        }
        self.palette.as_ref().unwrap()
    }

    pub fn config_was_reloaded(&mut self) {
        log::debug!(
            "config was reloaded, overrides: {:?}",
            self.config_overrides
        );
        self.key_table_state.clear_stack();
        self.connection_name = Connection::get().unwrap().name();
        let config = match config::overridden_config(&self.config_overrides) {
            Ok(config) => config,
            Err(err) => {
                log::error!(
                    "Failed to apply config overrides to window: {:#}: {:?}",
                    err,
                    self.config_overrides
                );
                configuration()
            }
        };
        self.config = config.clone();
        self.palette.take();

        let num_runtime_entries = self.get_session_entry_information().len();
        self.show_session_bar = Self::should_show_session_bar_for_count(&config, num_runtime_entries);
        *self.cursor_blink_state.borrow_mut() = ColorEase::new(
            config.cursor_blink_rate,
            config.cursor_blink_ease_in,
            config.cursor_blink_rate,
            config.cursor_blink_ease_out,
            None,
        );
        *self.blink_state.borrow_mut() = ColorEase::new(
            config.text_blink_rate,
            config.text_blink_ease_in,
            config.text_blink_rate,
            config.text_blink_ease_out,
            None,
        );
        *self.rapid_blink_state.borrow_mut() = ColorEase::new(
            config.text_blink_rate_rapid,
            config.text_blink_rapid_ease_in,
            config.text_blink_rate_rapid,
            config.text_blink_rapid_ease_out,
            None,
        );

        self.show_scroll_bar = config.enable_scroll_bar;
        self.shape_generation += 1;
        {
            let mut shape_cache = self.shape_cache.borrow_mut();
            shape_cache.update_config(&config);
            shape_cache.clear();
        }
        self.line_state_cache.borrow_mut().update_config(&config);
        self.line_quad_cache.borrow_mut().update_config(&config);
        self.line_to_ele_shape_cache
            .borrow_mut()
            .update_config(&config);
        self.fancy_tab_bar.take();
        self.invalidate_fancy_tab_bar();
        self.invalidate_modal();
        self.input_map = InputMap::new(&config);
        self.leader_is_down = None;
        self.render_state.as_mut().map(|rs| rs.config_changed());
        let dimensions = self.dimensions;

        if let Err(err) = self.fonts.config_changed(&config) {
            log::error!("Failed to load font configuration: {:#}", err);
        }

        let term_config: Arc<dyn TerminalConfiguration> =
            Arc::new(TermConfig::with_config(config.clone()));
        if self.chatminal_sidebar.is_enabled() {
            let snapshot = self.chatminal_sidebar.snapshot();
            for session in snapshot.sessions {
                for pane in self.positioned_panes_for_session(&session.session_id) {
                    pane.pane.set_config(Arc::clone(&term_config));
                }
            }
        } else {
            self.with_host_window(|window| {
                for tab in window.iter() {
                    for pane in tab.iter_panes_ignoring_zoom() {
                        pane.pane.set_config(Arc::clone(&term_config));
                    }
                }
            });
        }
        for state in self.terminal_ui_state_by_handle.borrow().values() {
            if let Some(overlay) = &state.overlay {
                overlay.pane.set_config(Arc::clone(&term_config));
            }
        }
        for state in self.runtime_ui_state.borrow().values() {
            if let Some(overlay) = &state.overlay {
                overlay.pane.set_config(Arc::clone(&term_config));
            }
        }

        if let Some(window) = self.window.as_ref().map(|w| w.clone()) {
            self.load_os_parameters();
            self.apply_scale_change(&dimensions, self.fonts.get_font_scale());
            self.apply_dimensions(&dimensions, None, &window);
            window.config_did_change(&config);
            window.invalidate();
        }

        // Do this after we've potentially adjusted scaling based on config/padding
        // and window size
        self.window_background = reload_background_image(
            &config,
            &self.window_background,
            &self.dimensions,
            &self.render_metrics,
        );

        self.invalidate_modal();
        self.emit_window_event("window-config-reloaded", None);
    }

    fn invalidate_modal(&mut self) {
        if let Some(modal) = self.get_modal() {
            modal.reconfigure(self);
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }
    }

    pub fn cancel_modal(&self) {
        self.modal.borrow_mut().take();
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn set_modal(&self, modal: Rc<dyn Modal>) {
        self.modal.borrow_mut().replace(modal);
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    fn get_modal(&self) -> Option<Rc<dyn Modal>> {
        self.modal.borrow().as_ref().map(|m| Rc::clone(&m))
    }

    fn update_scrollbar(&mut self) {
        if !self.show_scroll_bar {
            return;
        }

        let tab = match self.active_terminal_instance_or_overlay() {
            Some(tab) => tab,
            None => return,
        };

        let render_dims = tab.get_dimensions();
        if render_dims == self.last_scroll_info {
            return;
        }

        self.last_scroll_info = render_dims;

        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    /// Called by various bits of code to update the title bar.
    /// Let's also trigger the status event so that it can choose
    /// to update the right-status.
    fn update_title(&mut self) {
        self.schedule_status_update();
        self.update_title_impl();
    }

    fn window_contains_pane(&mut self, pane_id: TerminalUiKey) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            return self
                .get_session_entry_information()
                .iter()
                .flat_map(|entry| entry.terminal_instances.iter())
                .any(|leaf| leaf.host_terminal_handle == pane_id as u64);
        }

        let Some(pane_id) = crate::desktop_termwindow_types::pane_id_from_terminal_ui_key(pane_id)
        else {
            return false;
        };

        self.with_host_window(|window| window.iter().any(|tab| tab.contains_pane(pane_id)))
            .unwrap_or(false)
    }

    fn emit_user_var_event(&mut self, pane_id: TerminalUiKey, name: String, value: String) {
        if !self.window_contains_pane(pane_id) {
            return;
        }

        let window = GuiWin::new(self);
        let pane = match self.terminal_handle_arc(pane_id) {
            Some(pane) => pane.pane_id() as u64,
            None => return,
        };

        async fn do_event(
            lua: Option<Rc<mlua::Lua>>,
            name: String,
            value: String,
            window: GuiWin,
            pane_id: u64,
        ) -> anyhow::Result<()> {
            if let Some(lua) = lua {
                let args = lua.pack_multi((window.clone(), pane_id, name, value))?;
                if let Err(err) =
                    config::lua::emit_event(&lua, ("user-var-changed".to_string(), args)).await
                {
                    log::error!("while processing user-var-changed event: {:#}", err);
                }
            }

            window
                .window
                .notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                    term_window.update_title();
                })));

            Ok(())
        }

        promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
            do_event(lua, name, value, window, pane)
        }))
        .detach();
    }

    /// Called by window:set_right_status after the status has
    /// been updated; let's update the bar
    pub fn update_title_post_status(&mut self) {
        self.update_title_impl();
    }

    fn update_title_impl(&mut self) {
        let surfaces = self.get_session_entry_information();
        let num_tabs = surfaces.len();
        if num_tabs == 0 {
            return;
        }
        let terminal_instances = self.get_terminal_instance_information();
        let active_entry = surfaces.iter().find(|entry| entry.is_active).cloned();
        let active_terminal_instance = terminal_instances.iter().find(|leaf| leaf.is_active).cloned();

        let border = self.get_os_border();
        let tab_bar_height = self.tab_bar_pixel_height().unwrap_or(0.);
        let tab_bar_y = if self.config.session_bar_at_bottom {
            ((self.dimensions.pixel_height as f32) - (tab_bar_height + border.bottom.get() as f32))
                .max(0.)
        } else {
            border.top.get() as f32
        };

        let tab_bar_height = self.tab_bar_pixel_height().unwrap_or(0.);

        let tab_bar_x = self.terminal_tab_bar_left();
        let tab_bar_width = self.terminal_tab_bar_width();
        let hovering_in_tab_bar = match &self.current_mouse_event {
            Some(event) => {
                let mouse_x = event.coords.x as f32;
                let mouse_y = event.coords.y as f32;
                mouse_x >= tab_bar_x
                    && mouse_x < tab_bar_x + tab_bar_width
                    && mouse_y >= tab_bar_y as f32
                    && mouse_y < tab_bar_y as f32 + tab_bar_height
            }
            None => false,
        };

        let new_tab_bar = SessionBarState::new(
            self.terminal_tab_bar_cols(),
            if hovering_in_tab_bar {
                self.current_mouse_event.as_ref().map(|event| {
                    ((event.coords.x as f32 - tab_bar_x).max(0.0)
                        / self.render_metrics.cell_size.width as f32)
                        .floor() as usize
                })
            } else {
                None
            },
            &surfaces,
            &terminal_instances,
            self.config.resolved_palette.tab_bar.as_ref(),
            &self.config,
            &self.left_status,
            &self.right_status,
        );
        if new_tab_bar != self.tab_bar {
            self.tab_bar = new_tab_bar;
            self.invalidate_fancy_tab_bar();
            self.invalidate_modal();
            if let Some(window) = self.window.as_ref() {
                window.invalidate();
            }
        }

        let title = match config::run_immediate_with_lua_config(|lua| {
            if let Some(lua) = lua {
                let surfaces = lua.create_sequence_from(surfaces.clone().into_iter())?;
                let terminal_instances = lua.create_sequence_from(terminal_instances.clone().into_iter())?;

                let v = config::lua::emit_sync_callback(
                    &*lua,
                    (
                        "format-window-title".to_string(),
                        (
                            active_entry.clone(),
                            active_terminal_instance.clone(),
                            surfaces,
                            terminal_instances,
                            (*self.config).clone(),
                        ),
                    ),
                )?;
                match &v {
                    mlua::Value::Nil => Ok(None),
                    _ => Ok(Some(String::from_lua(v, &*lua)?)),
                }
            } else {
                Ok(None)
            }
        }) {
            Ok(s) => s,
            Err(err) => {
                log::warn!("format-window-title: {}", err);
                None
            }
        };

        let title = match title {
            Some(title) => title,
            None => {
                if let (Some(pos), Some(entry)) = (active_terminal_instance, active_entry) {
                    if num_tabs == 1 {
                        format!("{}{}", if pos.is_zoomed { "[Z] " } else { "" }, pos.title)
                    } else {
                        format!(
                            "{}[{}/{}] {}",
                            if pos.is_zoomed { "[Z] " } else { "" },
                            entry.entry_index + 1,
                            num_tabs,
                            pos.title
                        )
                    }
                } else {
                    "".to_string()
                }
            }
        };

        if let Some(window) = self.window.as_ref() {
            window.set_title(&title);

            let show_session_bar = Self::should_show_session_bar_for_count(&self.config, num_tabs);

            // If the number of tabs changed and caused the tab bar to
            // hide/show, then we'll need to resize things.  It is simplest
            // to piggy back on the config reloading code for that, so that
            // is what we're doing.
            if show_session_bar != self.show_session_bar {
                self.config_was_reloaded();
            }
        }
        self.schedule_next_status_update();
    }

    fn schedule_next_status_update(&mut self) {
        if let Some(window) = self.window.as_ref() {
            let now = Instant::now();
            if self.last_status_call <= now {
                let interval = Duration::from_millis(self.config.status_update_interval);
                let target = now + interval;
                self.last_status_call = target;

                let window = window.clone();
                promise::spawn::spawn(async move {
                    Timer::at(target).await;
                    window.notify(TermWindowNotif::EmitStatusUpdate);
                })
                .detach();
            }
        }
    }

    fn update_text_cursor(&mut self, pos: &TerminalPaneLayout) {
        if let Some(win) = self.window.as_ref() {
            let cursor = pos.pane.get_cursor_position();
            let top = pos.pane.get_dimensions().physical_top;
            let tab_bar_height = if self.show_session_bar && !self.config.session_bar_at_bottom {
                self.tab_bar_pixel_height().unwrap()
            } else {
                0.0
            };
            let (padding_left, padding_top) = self.padding_left_top();

            let r = Rect::new(
                Point::new(
                    (((cursor.x + pos.left) as isize).max(0) * self.render_metrics.cell_size.width)
                        .add(padding_left as isize),
                    ((cursor.y + pos.top as isize - top).max(0)
                        * self.render_metrics.cell_size.height)
                        .add(tab_bar_height as isize)
                        .add(padding_top as isize),
                ),
                self.render_metrics.cell_size,
            );
            win.set_text_cursor_position(r);
        }
    }

    fn activate_window(&mut self, window_idx: usize) -> anyhow::Result<()> {
        let windows = front_end().gui_windows();
        if let Some(win) = windows.get(window_idx) {
            win.window.focus();
        }
        Ok(())
    }

    fn activate_window_relative(&mut self, delta: isize, wrap: bool) -> anyhow::Result<()> {
        let windows = front_end().gui_windows();
        let my_idx = windows
            .iter()
            .position(|w| Some(&w.window) == self.window.as_ref())
            .ok_or_else(|| anyhow!("I'm not in the window list!?"))?;

        let idx = my_idx as isize + delta;

        let idx = if wrap {
            let idx = if idx < 0 {
                windows.len() as isize + idx
            } else {
                idx
            };
            idx as usize % windows.len()
        } else {
            if idx < 0 {
                0
            } else if idx >= windows.len() as isize {
                windows.len().saturating_sub(1)
            } else {
                idx as usize
            }
        };

        if let Some(win) = windows.get(idx) {
            win.window.focus();
        }

        Ok(())
    }

    fn activate_runtime_entry_index(&mut self, entry_idx: isize) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return self.activate_chatminal_session_index(entry_idx);
        }
        let activated = self
            .with_host_window_mut(|window| {
                // This logic is coupled with the runtime-entry activation CLI path
                // logic in the desktop entrypoint. If you update this, update that!
                let max = window.len();
                let entry_idx = if entry_idx < 0 {
                    max.saturating_sub(entry_idx.abs() as usize)
                } else {
                    entry_idx as usize
                };

                if entry_idx < max {
                    window.save_and_then_set_active(entry_idx);
                    true
                } else {
                    false
                }
            })
            .ok_or_else(|| anyhow!("no such window"))?;

        if activated {
            if let Some(tab) = self.active_terminal_instance_or_overlay() {
                tab.focus_changed(true);
            }

            self.update_title();
            self.update_scrollbar();
            self.sync_active_chatminal_session_from_mux();
        }
        Ok(())
    }

    fn activate_runtime_entry_relative(&mut self, delta: isize, wrap: bool) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return self.activate_chatminal_session_relative(delta, wrap);
        }
        let entry_idx = self
            .with_host_window(|window| {
                let max = window.len();
                (max, window.get_active_idx() as isize)
            })
            .ok_or_else(|| anyhow!("no such window"))
            .and_then(|(max, active)| {
                ensure!(max > 0, "no more tabs");
                let entry_idx = active + delta;
                Ok(if wrap {
                    let entry_idx = if entry_idx < 0 {
                        max as isize + entry_idx
                    } else {
                        entry_idx
                    };
                    (entry_idx as usize % max) as isize
                } else if entry_idx < 0 {
                    0
                } else if entry_idx >= max as isize {
                    max as isize - 1
                } else {
                    entry_idx
                })
            })?;
        self.activate_runtime_entry_index(entry_idx)
    }

    fn activate_last_runtime_entry(&mut self) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return self.activate_last_chatminal_session();
        }
        let last_idx = self
            .with_host_window(|window| window.get_last_active_idx())
            .ok_or_else(|| anyhow!("no such window"))?;
        match last_idx {
            Some(idx) => self.activate_runtime_entry_index(idx as isize),
            None => Ok(()),
        }
    }

    fn move_runtime_entry(&mut self, entry_idx: usize) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return Ok(());
        }
        self.with_host_window_mut(|window| {
            let max = window.len();
            ensure!(max > 0, "no more tabs");

            let active = window.get_active_idx();

            ensure!(entry_idx < max, "cannot move a runtime out of range");

            let tab = window.remove_by_idx(active);
            window.insert(entry_idx, &tab);
            window.set_active_without_saving(entry_idx);
            Ok::<(), anyhow::Error>(())
        })
        .ok_or_else(|| anyhow!("no such window"))??;

        self.update_title();
        self.update_scrollbar();

        Ok(())
    }

    fn show_input_selector(&mut self, args: &config::keyassignment::InputSelector) {
        // Ignore any current overlay: we're going to cancel it out below
        // and we don't want this new one to reference that cancelled pane
        let pane = match self.active_terminal_instance_no_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane_id = pane.pane_id() as u64;

        self.spawn_overlay_on_active_render_scope(move |_tab_id, term| {
            crate::overlay::selector::selector(term, args, gui_win, pane_id)
        });
    }

    fn show_prompt_input_line(&mut self, args: &PromptInputLine) {
        let pane = match self.active_terminal_instance_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane_id = pane.pane_id() as u64;

        self.spawn_overlay_on_active_render_scope(move |_tab_id, term| {
            crate::overlay::prompt::show_line_prompt_overlay(term, args, gui_win, pane_id)
        });
    }

    fn show_confirmation(&mut self, args: &Confirmation) {
        let pane = match self.active_terminal_instance_or_overlay() {
            Some(pane) => pane,
            None => return,
        };

        let args = args.clone();

        let gui_win = GuiWin::new(self);
        let pane_id = pane.pane_id() as u64;

        self.spawn_overlay_on_active_render_scope(move |_tab_id, term| {
            crate::overlay::confirm::show_confirmation_overlay(term, args, gui_win, pane_id)
        });
    }

    fn show_debug_overlay(&mut self) {
        let gui_win = GuiWin::new(self);

        let opengl_info = self.opengl_info.as_deref().unwrap_or("Unknown").to_string();
        let connection_info = self.connection_name.clone();

        self.spawn_overlay_on_active_render_scope(move |_tab_id, term| {
            crate::overlay::show_debug_overlay(term, gui_win, opengl_info, connection_info)
        });
    }

    fn show_runtime_entry_navigator(&mut self) {
        if self.is_session_ui_mode() {
            return;
        }
        let active_tab_idx = match self.with_host_window(|window| window.get_active_idx()) {
            Some(active_tab_idx) => active_tab_idx,
            None => return,
        };
        let title = "Session Navigator".to_string();
        let args = LauncherActionArgs {
            title: Some(title),
            flags: LauncherFlags::TABS,
            help_text: None,
            fuzzy_help_text: None,
            alphabet: None,
        };
        self.show_launcher_impl(args, active_tab_idx);
    }

    fn show_launcher(&mut self) {
        let title = "Launcher".to_string();
        let mut flags = LauncherFlags::LAUNCH_MENU_ITEMS
            | LauncherFlags::DOMAINS
            | LauncherFlags::KEY_ASSIGNMENTS
            | LauncherFlags::COMMANDS;
        if !self.is_session_ui_mode() {
            flags |= LauncherFlags::WORKSPACES;
        }
        let args = LauncherActionArgs {
            title: Some(title),
            flags,
            help_text: None,
            fuzzy_help_text: None,
            alphabet: None,
        };
        self.show_launcher_impl(args, 0);
    }

    fn show_launcher_impl(&mut self, args: LauncherActionArgs, initial_choice_idx: usize) {
        let engine_window_id = self.window_id;
        let window = self.window.as_ref().unwrap().clone();
        let render_scope_id = match self.active_render_scope_id() {
            Some(render_scope_id) => render_scope_id,
            None => return,
        };

        let pane = match self.active_terminal_instance_or_overlay() {
            Some(pane) => pane,
            None => return,
        };
        let active_terminal_pane = match self.active_terminal_instance() {
            Some(pane) => pane,
            None => return,
        };

        let domain_id_of_current_pane = active_terminal_pane.domain_id();
        let pane_id = pane.pane_id() as u64;
        let title = args.title.unwrap();
        let flags = args.flags;
        let help_text = args.help_text.unwrap_or(
            "Select an item and press Enter=launch  \
             Esc=cancel  /=filter"
                .to_string(),
        );
        let fuzzy_help_text = args
            .fuzzy_help_text
            .unwrap_or("Fuzzy matching: ".to_string());

        let config = &self.config;
        let alphabet = args.alphabet.unwrap_or(config.launcher_alphabet.clone());

        promise::spawn::spawn(async move {
            let args = LauncherArgs::new(
                &title,
                flags,
                engine_window_id as DesktopWindowId,
                pane_id as u64,
                domain_id_of_current_pane,
                &help_text,
                &fuzzy_help_text,
                &alphabet,
            )
            .await;

            let win = window.clone();
            win.notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                let window = window.clone();
                let _ = term_window.spawn_overlay_for_render_scope(
                    render_scope_id,
                    move |_tab_id, term| launcher(args, term, window, initial_choice_idx),
                );
            })));
        })
        .detach();
    }

    /// Returns the Prompt semantic zones
    fn get_semantic_prompt_zones(&mut self, pane: &Arc<dyn OverlayPane>) -> &[StableRowIndex] {
        let cache = self
            .semantic_zones
            .entry(pane.pane_id() as u64)
            .or_insert_with(SemanticZoneCache::default);

        let seqno = pane.get_current_seqno();
        if cache.seqno != seqno {
            let zones = pane.get_semantic_zones().unwrap_or_else(|_| vec![]);
            let mut zones: Vec<StableRowIndex> = zones
                .into_iter()
                .filter_map(|zone| {
                    if zone.semantic_type == engine_term::SemanticType::Prompt {
                        Some(zone.start_y)
                    } else {
                        None
                    }
                })
                .collect();
            // dedup to avoid issues where both left and right prompts are
            // defined: we only care if there were 1+ prompts on a line,
            // not about how many prompts are on a line.
            // upstream issue #1121
            zones.dedup();
            cache.zones = zones;
            cache.seqno = seqno;
        }
        &cache.zones
    }

    fn scroll_to_prompt(
        &mut self,
        amount: isize,
        pane: &Arc<dyn OverlayPane>,
    ) -> anyhow::Result<()> {
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id() as u64)
            .unwrap_or(dims.physical_top);
        let zone = {
            let zones = self.get_semantic_prompt_zones(&pane);
            let idx = match zones.binary_search(&position) {
                Ok(idx) | Err(idx) => idx,
            };
            let idx = ((idx as isize) + amount).max(0) as usize;
            zones.get(idx).cloned()
        };
        if let Some(zone) = zone {
            self.set_viewport(pane.pane_id() as u64, Some(zone), dims);
        }

        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn scroll_by_page(&mut self, amount: f64, pane: &Arc<dyn OverlayPane>) -> anyhow::Result<()> {
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id() as u64)
            .unwrap_or(dims.physical_top) as f64
            + (amount * dims.viewport_rows as f64);
        self.set_viewport(pane.pane_id() as u64, Some(position as isize), dims);
        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn scroll_by_current_event_wheel_delta(
        &mut self,
        pane: &Arc<dyn OverlayPane>,
    ) -> anyhow::Result<()> {
        if let Some(event) = &self.current_mouse_event {
            let amount = match event.kind {
                MouseEventKind::VertWheel(amount) => -amount,
                _ => return Ok(()),
            };
            self.scroll_by_line(amount.into(), pane)?;
        }
        Ok(())
    }

    fn scroll_by_line(
        &mut self,
        amount: isize,
        pane: &Arc<dyn OverlayPane>,
    ) -> anyhow::Result<()> {
        let dims = pane.get_dimensions();
        let position = self
            .get_viewport(pane.pane_id() as u64)
            .unwrap_or(dims.physical_top)
            .saturating_add(amount);
        self.set_viewport(pane.pane_id() as u64, Some(position), dims);
        if let Some(win) = self.window.as_ref() {
            win.invalidate();
        }
        Ok(())
    }

    fn move_runtime_relative(&mut self, delta: isize) -> anyhow::Result<()> {
        if self.is_session_ui_mode() {
            return Ok(());
        }
        let entry_idx = self
            .with_host_window(|window| {
                let max = window.len();
                (max, window.get_active_idx())
            })
            .ok_or_else(|| anyhow!("no such window"))
            .and_then(|(max, active)| {
                ensure!(max > 0, "no more tabs");
                Ok(if active as isize + delta < 0 {
                    0usize
                } else if active as isize + delta >= max as isize {
                    max - 1
                } else {
                    (active as isize + delta) as usize
                })
            })?;
        self.move_runtime_entry(entry_idx)
    }

    fn do_open_link_at_mouse_cursor(&self, pane: &Arc<dyn OverlayPane>) {
        // They clicked on a link, so let's open it!
        // We need to ensure that we spawn the `open` call outside of the context
        // of our window loop; on Windows it can cause a panic due to
        // triggering our WndProc recursively.
        // We get that assurance for free as part of the async dispatch that we
        // perform below; here we allow the user to define an `open-uri` event
        // handler that can bypass the normal `open_url` functionality.
        if let Some(link) = self.current_highlight.as_ref().cloned() {
            let window = GuiWin::new(self);
            let pane_id = pane.pane_id() as u64;

            async fn open_uri(
                lua: Option<Rc<mlua::Lua>>,
                window: GuiWin,
                pane_id: u64,
                link: String,
            ) -> anyhow::Result<()> {
                let default_click = match lua {
                    Some(lua) => {
                        let args = lua.pack_multi((window, pane_id, link.clone()))?;
                        config::lua::emit_event(&lua, ("open-uri".to_string(), args))
                            .await
                            .map_err(|e| {
                                log::error!("while processing open-uri event: {:#}", e);
                                e
                            })?
                    }
                    None => true,
                };
                if default_click {
                    log::info!("clicking {}", link);
                    engine_open_url::open_url(&link);
                }
                Ok(())
            }

            promise::spawn::spawn(config::with_lua_config_on_main_thread(move |lua| {
                open_uri(lua, window, pane_id, link.uri().to_string())
            }))
            .detach();
        }
    }
}
