impl TermWindow {
    fn positioned_panes_for_session(&self, session_id: &str) -> Vec<TerminalPaneLayout> {
        // Phase 03+: 1 session = 1 pane invariant — look up pane directly.
        let Some(session_pane) = desktop_pane_for_session(session_id) else {
            return vec![];
        };
        let pane_id = crate::desktop_termwindow_types::terminal_ui_key_for_pane(&*session_pane);
        let cell_width = self.render_metrics.cell_size.width.max(0) as usize;
        let cell_height = self.render_metrics.cell_size.height.max(0) as usize;
        let dims = session_pane.get_dimensions();
        let cols = dims.cols as usize;
        let rows = dims.viewport_rows as usize;
        if let Some(overlay) = self.session_render_target_overlay(session_id) {
            return vec![TerminalPaneLayout {
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 0,
                top: 0,
                width: cols,
                height: rows,
                pixel_width: cols * cell_width,
                pixel_height: rows * cell_height,
                pane: overlay,
            }];
        }
        // Pane-level overlay check (e.g. launcher/prompt on this pane)
        let session_overlay = self.terminal_overlay_pane(pane_id);
        if let Some(overlay) = session_overlay {
            return vec![TerminalPaneLayout {
                index: 0,
                is_active: true,
                is_zoomed: false,
                left: 0,
                top: 0,
                width: cols,
                height: rows,
                pixel_width: cols * cell_width,
                pixel_height: rows * cell_height,
                pane: overlay,
            }];
        }
        let Some(pane) = self.terminal_handle_arc(pane_id) else {
            return vec![];
        };
        vec![TerminalPaneLayout {
            index: 0,
            is_active: true,
            is_zoomed: false,
            left: 0,
            top: 0,
            width: cols,
            height: rows,
            pixel_width: cols * cell_width,
            pixel_height: rows * cell_height,
            pane,
        }]
    }

    fn positioned_splits_for_session(&self, _session_id: &str) -> Vec<TerminalSplit> {
        // After Phase 03: session-level splits are always empty (splits = [] invariant).
        // Workspace-level splits are returned by layout_positioned_splits().
        vec![]
    }

    fn get_splits(&mut self) -> Vec<TerminalSplit> {
        let layout_splits = self.layout_positioned_splits();
        if !layout_splits.is_empty() {
            return layout_splits;
        }
        if self.chatminal_sidebar.is_enabled() {
            if let Some(session_id) = self.active_session_id() {
                let session_splits = self.positioned_splits_for_session(&session_id);
                if !session_splits.is_empty() {
                    return session_splits;
                }
            }
        }
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return vec![];
        };

        if self.render_target_overlay(render_scope_id).is_some() {
            vec![]
        } else {
            self.active_render_target_splits()
        }
    }
}
