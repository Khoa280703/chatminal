impl TermWindow {
    fn cancel_overlay_for_render_scope(
        &mut self,
        render_scope_id: u64,
        host_terminal_handle: Option<TerminalUiKey>,
    ) {
        if host_terminal_handle.is_some() {
            let current = self
                .render_target_overlay(render_scope_id)
                .map(|overlay| overlay.pane_id() as u64);
            if current != host_terminal_handle {
                return;
            }
        }
        if let Some(overlay) = self.runtime_ui_state(render_scope_id).overlay.take() {
            self.remove_terminal_handle(overlay.pane.pane_id() as u64);
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn schedule_cancel_overlay_for_render_scope(
        window: Window,
        render_scope_id: u64,
        host_terminal_handle: Option<u64>,
    ) {
        window.notify(TermWindowNotif::CancelOverlayForRenderScope {
            render_target_id: render_scope_id,
            pane_id: host_terminal_handle,
        });
    }

    fn cancel_overlay_for_terminal_handle(&mut self, terminal_handle: TerminalUiKey) {
        if let Some(overlay) = self.terminal_ui_state(terminal_handle).overlay.take() {
            // Ungh, when I built the CopyOverlay, its pane doesn't get
            // added to the mux and instead it reports the overlaid
            // pane id.  Take care to avoid killing ourselves off
            // when closing the CopyOverlay
            if terminal_handle != overlay.pane.pane_id() as u64 {
                self.remove_terminal_handle(overlay.pane.pane_id() as u64);
            }
        }
        if let Some(window) = self.window.as_ref() {
            window.invalidate();
        }
    }

    pub fn schedule_cancel_overlay_for_terminal_handle(window: Window, terminal_handle: u64) {
        window.notify(TermWindowNotif::CancelOverlayForTerminalHandle(terminal_handle));
    }

    pub fn assign_overlay_for_terminal_handle(
        &mut self,
        terminal_handle: TerminalUiKey,
        pane: Arc<dyn OverlayPane>,
    ) {
        self.cancel_overlay_for_terminal_handle(terminal_handle);
        self.terminal_ui_state(terminal_handle)
            .overlay
            .replace(OverlayState {
                pane,
                key_table_state: KeyTableState::default(),
            });
        self.update_title();
    }

    pub fn assign_overlay_for_render_scope(
        &mut self,
        render_scope_id: u64,
        overlay: Arc<dyn OverlayPane>,
    ) {
        self.cancel_overlay_for_render_scope(render_scope_id, None);
        self.runtime_ui_state(render_scope_id)
            .overlay
            .replace(OverlayState {
                pane: overlay,
                key_table_state: KeyTableState::default(),
            });
        self.update_title();
    }

}
