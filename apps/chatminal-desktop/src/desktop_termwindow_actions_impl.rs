pub fn perform_key_assignment(
    &mut self,
    pane: &Arc<dyn OverlayPane>,
    assignment: &KeyAssignment,
) -> anyhow::Result<PerformAssignmentResult> {
    use crate::chatminal_runtime::{DesktopSplitDirection, DesktopSplitRequest, DesktopSplitSize};
    use KeyAssignment::*;

    if let Some(modal) = self.get_modal() {
        if modal.perform_assignment(assignment, self) {
            return Ok(PerformAssignmentResult::Handled);
        }
    }

    match pane.perform_assignment(assignment) {
        PerformAssignmentResult::Unhandled => {}
        result => return Ok(result),
    }

    let window = self.window.as_ref().map(|w| w.clone());

    if let Some(session_bar_assignment) =
        crate::desktop_commands::session_bar_assignment_for_key_assignment(assignment)
    {
        match session_bar_assignment {
            crate::desktop_commands::SessionBarAssignment::ActivateRelative { delta, wrap } => {
                self.activate_runtime_entry_relative(delta, wrap)?;
            }
            crate::desktop_commands::SessionBarAssignment::ActivateLast => {
                self.activate_last_runtime_entry()?;
            }
            crate::desktop_commands::SessionBarAssignment::ActivateIndex(index) => {
                self.activate_runtime_entry_index(index)?;
            }
            crate::desktop_commands::SessionBarAssignment::MoveTo(index) => {
                self.move_runtime_entry(index)?;
            }
            crate::desktop_commands::SessionBarAssignment::MoveRelative(delta) => {
                self.move_runtime_relative(delta)?;
            }
        }
        return Ok(PerformAssignmentResult::Handled);
    }

    match assignment {
        ActivateKeyTable {
            name,
            timeout_milliseconds,
            replace_current,
            one_shot,
            until_unknown,
            prevent_fallback,
        } => {
            anyhow::ensure!(
                self.input_map.has_table(name),
                "ActivateKeyTable: no key_table named {}",
                name
            );
            self.key_table_state.activate(KeyTableArgs {
                name,
                timeout_milliseconds: *timeout_milliseconds,
                replace_current: *replace_current,
                one_shot: *one_shot,
                until_unknown: *until_unknown,
                prevent_fallback: *prevent_fallback,
            });
            self.update_title();
        }
        PopKeyTable => {
            self.key_table_state.pop();
            self.update_title();
        }
        ClearKeyTableStack => {
            self.key_table_state.clear_stack();
            self.update_title();
        }
        Multiple(actions) => {
            for a in actions {
                self.perform_key_assignment(pane, a)?;
            }
        }
        SpawnSession => {
            self.spawn_runtime_entry();
        }
        SpawnCommandInNewSession(spawn) => {
            self.spawn_command(spawn, SpawnWhere::NewSession);
        }
        SplitHorizontal(spawn) => {
            log::trace!("SplitHorizontal {:?}", spawn);
            self.spawn_command(
                spawn,
                SpawnWhere::SplitSession(DesktopSplitRequest {
                    direction: DesktopSplitDirection::Horizontal,
                    target_is_second: true,
                    size: DesktopSplitSize::Percent(50),
                    top_level: false,
                }),
            );
        }
        SplitVertical(spawn) => {
            log::trace!("SplitVertical {:?}", spawn);
            self.spawn_command(
                spawn,
                SpawnWhere::SplitSession(DesktopSplitRequest {
                    direction: DesktopSplitDirection::Vertical,
                    target_is_second: true,
                    size: DesktopSplitSize::Percent(50),
                    top_level: false,
                }),
            );
        }
        ToggleFullScreen => {
            self.window.as_ref().unwrap().toggle_fullscreen();
        }
        ToggleAlwaysOnTop => {
            let window = self.window.clone().unwrap();
            let current_level = self.window_state.as_window_level();

            match current_level {
                WindowLevel::AlwaysOnTop => {
                    window.set_window_level(WindowLevel::Normal);
                }
                WindowLevel::AlwaysOnBottom | WindowLevel::Normal => {
                    window.set_window_level(WindowLevel::AlwaysOnTop);
                }
            }
        }
        ToggleAlwaysOnBottom => {
            let window = self.window.clone().unwrap();
            let current_level = self.window_state.as_window_level();

            match current_level {
                WindowLevel::AlwaysOnBottom => {
                    window.set_window_level(WindowLevel::Normal);
                }
                WindowLevel::AlwaysOnTop | WindowLevel::Normal => {
                    window.set_window_level(WindowLevel::AlwaysOnBottom);
                }
            }
        }
        SetWindowLevel(level) => {
            let window = self.window.clone().unwrap();
            window.set_window_level(level.clone());
        }
        CopyTo(dest) => {
            let text = self.selection_text(pane);
            self.copy_to_clipboard(*dest, text);
        }
        CopyTextTo { text, destination } => {
            self.copy_to_clipboard(*destination, text.clone());
        }
        PasteFrom(source) => {
            self.paste_from_clipboard(pane, *source);
        }
        DecreaseFontSize => self.decrease_font_size(),
        IncreaseFontSize => self.increase_font_size(),
        ResetFontSize => self.reset_font_size(),
        ResetFontAndWindowSize => {
            if let Some(w) = window.as_ref() {
                self.reset_font_and_window_size(&w)?
            }
        }
        SendString(s) => pane.writer().write_all(s.as_bytes())?,
        SendKey(key) => {
            use keyevent::Key;
            let mods = key.mods;
            if let Key::Code(key) = self
                .win_key_code_to_termwiz_key_code(&key.key.resolve(self.config.key_map_preference))
            {
                pane.key_down(key, mods)?;
            }
        }
        Hide => {
            if let Some(w) = window.as_ref() {
                w.hide();
            }
        }
        Show => {
            if let Some(w) = window.as_ref() {
                w.show();
            }
        }
        CloseCurrentSession { confirm } => self.close_current_runtime_entry(*confirm),
        Nop | DisableDefaultAssignment => {}
        ReloadConfiguration => config::reload(),
        ScrollByPage(n) => self.scroll_by_page(**n, pane)?,
        ScrollByLine(n) => self.scroll_by_line(*n, pane)?,
        ScrollByCurrentEventWheelDelta => self.scroll_by_current_event_wheel_delta(pane)?,
        ScrollToPrompt(n) => self.scroll_to_prompt(*n, pane)?,
        ScrollToTop => self.scroll_to_top(pane),
        ScrollToBottom => self.scroll_to_bottom(pane),
        ToggleRealtimeFooter => {
            self.show_terminal_footer = !self.show_terminal_footer;
            if let Some(window) = self.window.as_ref().cloned() {
                let dimensions = self.dimensions;
                self.apply_dimensions(&dimensions, None, &window);
                window.invalidate();
            }
        }
        ShowSessionNavigator => self.show_runtime_entry_navigator(),
        ShowDebugOverlay => self.show_debug_overlay(),
        HideApplication => {
            let con = Connection::get().expect("call on gui thread");
            con.hide_application();
        }
        QuitApplication => {
            self.request_quit_application()?;
        }
        SelectTextAtMouseCursor(mode) => self.select_text_at_mouse_cursor(*mode, pane),
        ExtendSelectionToMouseCursor(mode) => self.extend_selection_at_mouse_cursor(*mode, pane),
        ClearSelection => {
            self.clear_selection(pane);
        }
        StartWindowDrag => {
            self.window_drag_position = self.current_mouse_event.clone();
        }
        EmitEvent(name) => {
            self.emit_window_event(name, None);
        }
        CompleteSelectionOrOpenLinkAtMouseCursor(dest) => {
            let text = self.selection_text(pane);
            if !text.is_empty() {
                self.copy_to_clipboard(*dest, text);
                let window = self.window.as_ref().unwrap();
                window.invalidate();
            }
        }
        CompleteSelection(dest) => {
            let text = self.selection_text(pane);
            if !text.is_empty() {
                self.copy_to_clipboard(*dest, text);
                let window = self.window.as_ref().unwrap();
                window.invalidate();
            }
        }
        Search(pattern) => {
            if let Some(pane) = self.active_terminal_instance_or_overlay() {
                let pane_id = crate::desktop_termwindow_types::terminal_ui_key_for_pane(&*pane);
                let mut replace_current = false;
                if let Some(existing) = pane.downcast_ref::<CopyOverlay>() {
                    let mut params = existing.get_params();
                    params.editing_search = true;
                    if !pattern.is_empty() {
                        params.pattern = self.resolve_search_pattern(pattern.clone(), &pane);
                    }
                    existing.apply_params(params);
                    replace_current = true;
                } else {
                    let search = CopyOverlay::with_pane(
                        self,
                        &pane,
                        CopyModeParams {
                            pattern: self.resolve_search_pattern(pattern.clone(), &pane),
                            editing_search: true,
                        },
                    )?;
                    self.assign_overlay_for_terminal_handle(pane_id, search);
                }
                self.terminal_ui_state(pane_id)
                    .overlay
                    .as_mut()
                    .map(|overlay| {
                        overlay.key_table_state.activate(KeyTableArgs {
                            name: "search_mode",
                            timeout_milliseconds: None,
                            replace_current,
                            one_shot: false,
                            until_unknown: false,
                            prevent_fallback: false,
                        });
                    });
            }
        }
        QuickSelect => {
            if let Some(pane) = self.active_terminal_instance_no_overlay() {
                let pane_id = crate::desktop_termwindow_types::terminal_ui_key_for_pane(&*pane);
                let qa =
                    QuickSelectOverlay::with_pane(self, &pane, &QuickSelectArguments::default());
                self.assign_overlay_for_terminal_handle(pane_id, qa);
            }
        }
        QuickSelectArgs(args) => {
            if let Some(pane) = self.active_terminal_instance_no_overlay() {
                let pane_id = crate::desktop_termwindow_types::terminal_ui_key_for_pane(&*pane);
                let qa = QuickSelectOverlay::with_pane(self, &pane, args);
                self.assign_overlay_for_terminal_handle(pane_id, qa);
            }
        }
        ActivateCopyMode => {
            if let Some(pane) = self.active_terminal_instance_or_overlay() {
                let pane_id = crate::desktop_termwindow_types::terminal_ui_key_for_pane(&*pane);
                let mut replace_current = false;
                if let Some(existing) = pane.downcast_ref::<CopyOverlay>() {
                    let mut params = existing.get_params();
                    params.editing_search = false;
                    existing.apply_params(params);
                    replace_current = true;
                } else {
                    let copy = CopyOverlay::with_pane(
                        self,
                        &pane,
                        CopyModeParams {
                            pattern: OverlayPattern::default(),
                            editing_search: false,
                        },
                    )?;
                    self.assign_overlay_for_terminal_handle(pane_id, copy);
                }
                self.terminal_ui_state(pane_id)
                    .overlay
                    .as_mut()
                    .map(|overlay| {
                        overlay.key_table_state.activate(KeyTableArgs {
                            name: "copy_mode",
                            timeout_milliseconds: None,
                            replace_current,
                            one_shot: false,
                            until_unknown: false,
                            prevent_fallback: false,
                        });
                    });
            }
        }
        AdjustSplitSize(direction, amount) => {
            log::warn!(
                "AdjustSplitSize({direction:?}, {amount}) is not supported in Chatminal session UI"
            );
            return Ok(PerformAssignmentResult::Handled);
        }
        ActivateTerminalByIndex(index) => {
            if !self.active_runtime_has_overlay() {
                if self.chatminal_sidebar.is_enabled() {
                    if let Some(pos) = self.get_panes_to_render().into_iter().nth(*index) {
                        if !self.focus_active_session_terminal_instance(&pos.pane) {
                            self.focus_terminal_handle(&pos.pane);
                        }
                    }
                } else {
                    let _ = self.activate_terminal_index_in_active_render_target(*index);
                }
            }
        }
        ActivateSessionDirection(direction) => {
            if !self.active_runtime_has_overlay() {
                let focused_leaf = self.chatminal_sidebar.is_enabled();
                if !focused_leaf {
                    let _ = self.activate_terminal_direction_in_active_render_target(*direction);
                }
            }
        }
        ToggleTerminalZoomState => {
            log::warn!("ToggleTerminalZoomState is not supported in Chatminal session UI");
            return Ok(PerformAssignmentResult::Handled);
        }
        SetTerminalZoomState(zoomed) => {
            log::warn!(
                "SetTerminalZoomState({zoomed}) is not supported in Chatminal session UI"
            );
            return Ok(PerformAssignmentResult::Handled);
        }
        SwitchWorkspaceRelative(_) | SwitchToWorkspace { .. } => {
            log::warn!("workspace switching has been removed from Chatminal Desktop");
            return Ok(PerformAssignmentResult::Handled);
        }
        CopyMode(_) => {
            // NOP here; handled by the overlay directly
        }
        RotatePanes(direction) => {
            log::warn!("RotatePanes({direction:?}) is not supported in Chatminal session UI");
            return Ok(PerformAssignmentResult::Handled);
        }
        SplitSession(split) => {
            log::trace!("SplitSession {:?}", split);
            self.spawn_command(
                &split.command,
                SpawnWhere::SplitSession(DesktopSplitRequest {
                    direction: match split.direction {
                        SessionDirection::Down | SessionDirection::Up => {
                            DesktopSplitDirection::Vertical
                        }
                        SessionDirection::Left | SessionDirection::Right => {
                            DesktopSplitDirection::Horizontal
                        }
                        SessionDirection::Next | SessionDirection::Prev => {
                            log::error!("Invalid direction {:?} for SplitSession", split.direction);
                            return Ok(PerformAssignmentResult::Handled);
                        }
                    },
                    target_is_second: match split.direction {
                        SessionDirection::Down | SessionDirection::Right => true,
                        SessionDirection::Up | SessionDirection::Left => false,
                        SessionDirection::Next | SessionDirection::Prev => unreachable!(),
                    },
                    size: match split.size {
                        SplitSize::Percent(n) => DesktopSplitSize::Percent(n),
                        SplitSize::Cells(n) => DesktopSplitSize::Cells(n),
                    },
                    top_level: split.top_level,
                }),
            );
        }
        SessionSelect(args) => {
            let modal = crate::termwindow::paneselect::PaneSelector::new(self, args);
            self.set_modal(Rc::new(modal));
        }
        ResetTerminal => {
            if let Ok(session_pane) = pane
                .clone()
                .downcast_arc::<crate::desktop_session_host::ChatminalSessionPane>()
            {
                session_pane.reset_display_state_with_flash();
            } else {
                pane.reset_display_state();
            }
        }
        OpenUri(link) => {
            engine_open_url::open_url(link);
        }
        ActivateCommandPalette => {
            let modal = crate::termwindow::palette::CommandPalette::new(self);
            self.set_modal(Rc::new(modal));
        }
        PromptInputLine(args) => self.show_prompt_input_line(args),
        InputSelector(args) => self.show_input_selector(args),
        Confirmation(args) => self.show_confirmation(args),
        _ => unreachable!("handled by session bar assignment translator"),
    };
    Ok(PerformAssignmentResult::Handled)
}
