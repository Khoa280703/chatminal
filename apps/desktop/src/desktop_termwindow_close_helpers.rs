use crate::runtime_module::desktop_active_session_entry_binding;

impl TermWindow {
    fn close_runtime_entry_at_index(&mut self, entry_idx: usize, confirm: bool) {
        let Some(runtime_entry) = self.get_session_entry_information().get(entry_idx).cloned()
        else {
            return;
        };

        let render_scope_id = runtime_entry.render_target_id;
        if self.close_chatminal_session_for_render_scope(render_scope_id) {
            return;
        }
        if confirm
            && !self.render_scope_can_close_without_prompting(render_scope_id, CloseReason::Tab)
        {
            if self
                .activate_runtime_entry_index(entry_idx as isize)
                .is_err()
            {
                return;
            }

            self.spawn_overlay_for_render_scope(render_scope_id, move |tab_id, term| {
                show_close_runtime_entry_overlay(tab_id, term)
            });
        } else {
            self.remove_runtime_entry_scope(render_scope_id);
            self.sync_active_chatminal_session_from_runtime();
        }
    }

    fn close_current_runtime_entry(&mut self, confirm: bool) {
        let active_entry = desktop_active_session_entry_binding();
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
        }
    }
}
