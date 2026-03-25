impl TermWindow {
    pub fn renderable_dimensions_for_pane(
        &self,
        pane: &Arc<dyn OverlayPane>,
    ) -> RenderableDimensions {
        let dims = pane.get_dimensions();
        let Some(layout) = self
            .get_panes_to_render()
            .into_iter()
            .find(|pos| pos.pane.pane_id() == pane.pane_id())
        else {
            return dims;
        };

        RenderableDimensions {
            cols: layout.width.max(1).min(dims.cols.max(1)),
            viewport_rows: layout.height.max(1).min(dims.viewport_rows.max(1)),
            pixel_width: layout.pixel_width.max(1).min(dims.pixel_width.max(1)),
            pixel_height: layout.pixel_height.max(1).min(dims.pixel_height.max(1)),
            ..dims
        }
    }

    pub fn terminal_ui_state(&self, pane_id: TerminalUiKey) -> RefMut<'_, TerminalUiState> {
        RefMut::map(self.terminal_ui_state_by_handle.borrow_mut(), |state| {
            state.entry(pane_id).or_insert_with(TerminalUiState::default)
        })
    }

    pub fn runtime_ui_state(&self, render_scope_id: u64) -> RefMut<'_, RuntimeUiState> {
        RefMut::map(self.runtime_ui_state.borrow_mut(), |state| {
            state
                .entry(render_scope_id)
                .or_insert_with(RuntimeUiState::default)
        })
    }

    /// Resize overlays to match their corresponding tab/pane dimensions
    pub fn resize_overlays(&self) {
        for (_, state) in self.runtime_ui_state.borrow().iter() {
            if let Some(overlay) = state.overlay.as_ref().map(|o| &o.pane) {
                overlay.resize(self.terminal_size).ok();
            }
        }
        for (pane_id, state) in self.terminal_ui_state_by_handle.borrow().iter() {
                if let Some(overlay) = state.overlay.as_ref().map(|o| &o.pane) {
                if let Some(pane) = self.terminal_handle_arc(*pane_id) {
                    let dims = self.renderable_dimensions_for_pane(&pane);
                    overlay
                        .resize(TerminalSize {
                            cols: dims.cols,
                            rows: dims.viewport_rows,
                            dpi: self.terminal_size.dpi,
                            pixel_height: (self.terminal_size.pixel_height
                                / self.terminal_size.rows)
                                * dims.viewport_rows,
                            pixel_width: (self.terminal_size.pixel_width / self.terminal_size.cols)
                                * dims.cols,
                        })
                        .ok();
                }
            }
        }
    }

    pub fn get_viewport(&self, pane_id: TerminalUiKey) -> Option<StableRowIndex> {
        self.terminal_ui_state(pane_id).viewport
    }

    pub fn set_viewport(
        &mut self,
        pane_id: TerminalUiKey,
        position: Option<StableRowIndex>,
        dims: RenderableDimensions,
    ) {
        let pos = match position {
            Some(pos) => {
                // Drop out of scrolling mode if we're off the bottom
                if pos >= dims.physical_top {
                    None
                } else {
                    Some(pos.max(dims.scrollback_top))
                }
            }
            None => None,
        };

        let mut state = self.terminal_ui_state(pane_id);
        if pos != state.viewport {
            state.viewport = pos;

            // This is a bit gross.  If we add other overlays that need this information,
            // this should get extracted out into a trait
            if let Some(overlay) = state.overlay.as_ref() {
                if let Some(copy) = overlay.pane.downcast_ref::<CopyOverlay>() {
                    copy.viewport_changed(pos);
                } else if let Some(qs) = overlay.pane.downcast_ref::<QuickSelectOverlay>() {
                    qs.viewport_changed(pos);
                }
            }
        }
        self.window.as_ref().unwrap().invalidate();
    }

    fn maybe_scroll_to_bottom_for_input(&mut self, pane: &Arc<dyn OverlayPane>) {
        if self.config.scroll_to_bottom_on_input {
            self.scroll_to_bottom(pane);
        }
    }

    fn scroll_to_top(&mut self, pane: &Arc<dyn OverlayPane>) {
        let dims = self.renderable_dimensions_for_pane(pane);
        self.set_viewport(pane.pane_id() as u64, Some(dims.scrollback_top), dims);
    }

    fn scroll_to_bottom(&mut self, pane: &Arc<dyn OverlayPane>) {
        self.terminal_ui_state(pane.pane_id() as u64).viewport = None;
    }

    fn active_terminal_instance(&self) -> Option<Arc<dyn OverlayPane>> {
        if let Some(terminal_handle) = self.active_terminal_handle() {
            if let Ok(pane) = self.resolve_terminal_handle(terminal_handle) {
                return Some(pane);
            }
        }

        if self.chatminal_sidebar.is_enabled() {
            return None;
        }

        self.active_terminal_instance_from_active_render_target()
    }

    fn active_terminal_instance_no_overlay(&self) -> Option<Arc<dyn OverlayPane>> {
        self.active_terminal_instance()
    }

    /// Returns a leaf we can interact with; this will typically be
    /// the active host leaf for the window, but if the window has a host-tab-wide
    /// overlay (such as the launcher / tab navigator),
    /// then that will be returned instead. Otherwise, if the leaf has
    /// an active overlay (such as search or copy mode) then that will
    /// be returned.
    pub fn active_terminal_instance_or_overlay(&self) -> Option<Arc<dyn OverlayPane>> {
        if let Some(render_target_overlay) = self.active_render_target_overlay() {
            Some(render_target_overlay)
        } else {
            let pane = self.active_terminal_instance()?;
            let pane_id = pane.pane_id() as u64;
            self.terminal_ui_state(pane_id)
                .overlay
                .as_ref()
                .map(|overlay| overlay.pane.clone())
                .or_else(|| Some(pane))
        }
    }

}
