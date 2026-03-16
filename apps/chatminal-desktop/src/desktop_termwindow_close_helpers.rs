impl TermWindow {
    fn close_current_pane(&mut self, confirm: bool) {
        if self.chatminal_sidebar.is_enabled() {
            let Some(session_id) = self.active_session_id() else {
                return;
            };
            let Some(pane) = self.active_terminal_instance() else {
                return;
            };
            let pane_id = pane.pane_id() as u64;

            if confirm && !pane.can_close_without_prompting(CloseReason::Pane) {
                let window = self.window.clone().unwrap();
                let session_id_for_confirm = session_id.clone();
                let (overlay, future) =
                    start_overlay_pane(self, &pane, move |host_terminal_handle, term| {
                        confirm_close_chatminal_session_leaf_or_session(
                            session_id_for_confirm.clone(),
                            host_terminal_handle,
                            term,
                            window,
                        )
                    });
                self.assign_overlay_for_terminal_handle(pane_id, overlay);
                promise::spawn::spawn(future).detach();
            } else {
                self.close_chatminal_terminal_handle_or_session(&session_id, pane_id);
            }
            return;
        }

        // Non-session-mode path
        let pane = match self.active_terminal_instance() {
            Some(p) => p,
            None => return,
        };
        let pane_id = pane.pane_id() as u64;
        if confirm && !pane.can_close_without_prompting(CloseReason::Pane) {
            let window = self.window.clone().unwrap();
            let (overlay, future) = start_overlay_pane(self, &pane, move |pane_id, term| {
                show_close_terminal_overlay(pane_id, term, window)
            });
            self.assign_overlay_for_terminal_handle(pane_id, overlay);
            promise::spawn::spawn(future).detach();
        } else {
            self.remove_terminal_handle(pane_id);
        }
    }

    fn close_runtime_entry_at_index(&mut self, entry_idx: usize, confirm: bool) {
        let Some(runtime_entry) = self.with_host_window(|window| window.get_by_idx(entry_idx).cloned()) else {
            return;
        };
        let Some(runtime_entry) = runtime_entry else {
            return;
        };

        let render_scope_id = runtime_entry.tab_id();
        if self.close_chatminal_session_for_render_scope(render_scope_id as u64) {
            return;
        }
        if confirm
            && !self.render_scope_can_close_without_prompting(
                render_scope_id as u64,
                CloseReason::Tab,
            )
        {
            if self.activate_runtime_entry_index(entry_idx as isize).is_err() {
                return;
            }

            self.spawn_overlay_for_render_scope(render_scope_id as u64, move |tab_id, term| {
                show_close_runtime_entry_overlay(tab_id, term)
            });
        } else {
            self.remove_runtime_entry_scope(render_scope_id as u64);
            self.sync_active_chatminal_session_from_mux();
        }
    }

    fn close_current_runtime_entry(&mut self, confirm: bool) {
        if self.chatminal_sidebar.is_enabled() {
            let active_entry = crate::chatminal_runtime::desktop_active_session_entry_binding(
                self.window_id as DesktopWindowId,
            );
            if !confirm {
                if let Some(session_id) = active_entry.as_ref().map(|entry| entry.session_id.clone()) {
                    self.close_chatminal_view_or_session_by_id(&session_id);
                    return;
                }
            }
            if let Some(entry) = active_entry {
                if let Some(render_target_id) = entry.render_target_id {
                    if confirm
                        && !self.render_scope_can_close_without_prompting(
                            render_target_id.as_u64(),
                            CloseReason::Tab,
                        )
                    {
                        self.spawn_overlay_for_render_scope(
                            render_target_id.as_u64(),
                            move |tab_id, term| show_close_runtime_entry_overlay(tab_id, term),
                        );
                    } else {
                        self.close_chatminal_view_or_session_by_id(&entry.session_id);
                    }
                    return;
                }
                self.close_chatminal_view_or_session_by_id(&entry.session_id);
                return;
            }
        }

        let Some(render_scope_id) = self.active_render_scope_id() else {
            return;
        };
        if self.close_chatminal_session_for_render_scope(render_scope_id) {
            return;
        }
        if confirm
            && !self.render_scope_can_close_without_prompting(render_scope_id, CloseReason::Tab)
        {
            self.spawn_overlay_for_render_scope(render_scope_id, move |tab_id, term| {
                show_close_runtime_entry_overlay(tab_id, term)
            });
        } else {
            self.remove_runtime_entry_scope(render_scope_id);
            self.sync_active_chatminal_session_from_mux();
        }
    }

}
