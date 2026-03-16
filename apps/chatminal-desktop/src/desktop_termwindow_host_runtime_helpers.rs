impl TermWindow {
    pub(crate) fn active_terminal_handle(&self) -> Option<u64> {
        if self.chatminal_sidebar.is_enabled() {
            return crate::chatminal_runtime::desktop_current_active_terminal_handle(
                self.window_id as DesktopWindowId,
            )
            .map(|handle| handle.as_u64());
        }
        self.active_terminal_instance_from_active_render_target()
            .map(|pane| pane.pane_id() as u64)
    }

    pub(crate) fn active_workspace_name(&self) -> String {
        if self.chatminal_sidebar.is_enabled() {
            return self
                .active_session_id()
                .unwrap_or_else(|| crate::chatminal_runtime::DESKTOP_LAYOUT_WORKSPACE_ID.to_string());
        }
        Self::host_workspace_name()
    }

    fn terminal_handle_arc(&self, pane_id: TerminalUiKey) -> Option<Arc<dyn OverlayPane>> {
        crate::chatminal_runtime::terminal_handle_arc(
            crate::desktop_termwindow_types::pane_id_from_terminal_ui_key(pane_id)?,
        )
    }

    fn remove_terminal_handle(&self, pane_id: TerminalUiKey) {
        if let Some(pane_id) = crate::desktop_termwindow_types::pane_id_from_terminal_ui_key(pane_id)
        {
            crate::chatminal_runtime::remove_terminal_handle(pane_id);
        }
    }

    fn kill_host_window(&self) {
        crate::chatminal_runtime::kill_host_window(self.window_id);
    }

    fn with_host_window<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&RuntimeWindow) -> R,
    {
        let window = Self::with_host_window_by_id(self.window_id, func)?;
        Some(window)
    }

    fn with_host_window_by_id<R, F>(window_id: EngineWindowId, func: F) -> Option<R>
    where
        F: FnOnce(&RuntimeWindow) -> R,
    {
        crate::chatminal_runtime::with_host_window(window_id, func)
    }

    fn with_host_window_mut<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&mut RuntimeWindow) -> R,
    {
        crate::chatminal_runtime::with_host_window_mut(self.window_id, func)
    }

    fn remove_runtime_entry_scope(&self, render_scope_id: u64) {
        crate::chatminal_runtime::remove_runtime_entry_scope(render_scope_id);
    }

    fn host_window_exists(window_id: EngineWindowId) -> bool {
        crate::chatminal_runtime::host_window_exists(window_id)
    }

    fn host_window_contains_render_scope(window_id: EngineWindowId, render_scope_id: u64) -> bool {
        crate::chatminal_runtime::host_window_contains_render_scope(window_id, render_scope_id)
    }

    fn host_workspace_name() -> String {
        crate::chatminal_runtime::host_workspace_name()
    }

    fn host_workspace_names() -> Vec<String> {
        crate::chatminal_runtime::host_workspace_names()
    }

    fn generate_host_workspace_name() -> String {
        crate::chatminal_runtime::generate_host_workspace_name()
    }

    fn set_host_workspace(name: &str) {
        crate::chatminal_runtime::set_host_workspace(name);
    }

    fn host_workspace_has_windows(name: &str) -> bool {
        crate::chatminal_runtime::host_workspace_has_windows(name)
    }

    pub(crate) fn resolve_terminal_handle(
        &self,
        terminal_handle: u64,
    ) -> anyhow::Result<Arc<dyn OverlayPane>> {
        self.terminal_handle_arc(terminal_handle)
            .ok_or_else(|| anyhow!("host pane id {terminal_handle} is not valid"))
    }

    pub(crate) fn selection_text_for_terminal_handle(
        &self,
        terminal_handle: u64,
    ) -> anyhow::Result<String> {
        let pane = self.resolve_terminal_handle(terminal_handle)?;
        Ok(self.selection_text(&pane))
    }

    pub(crate) fn selection_escapes_for_terminal_handle(
        &self,
        terminal_handle: u64,
    ) -> anyhow::Result<String> {
        let pane = self.resolve_terminal_handle(terminal_handle)?;
        let lines = self.selection_lines(&pane);
        lines_to_escapes(lines)
    }

    pub(crate) fn perform_assignment_for_terminal_handle(
        &mut self,
        terminal_handle: u64,
        assignment: &KeyAssignment,
    ) -> anyhow::Result<()> {
        let pane = self.resolve_terminal_handle(terminal_handle)?;
        self.perform_key_assignment(&pane, assignment).map(|_| ())
    }

    fn focus_active_session_terminal_instance(&self, pane: &Arc<dyn OverlayPane>) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        crate::chatminal_runtime::desktop_focus_session_terminal_handle(
            self.window_id as DesktopWindowId,
            crate::chatminal_runtime::SessionTerminalHandle::new(pane.pane_id() as u64),
        )
        .is_some()
    }

    fn focus_terminal_handle(&self, pane: &Arc<dyn OverlayPane>) -> bool {
        crate::chatminal_runtime::focus_terminal_handle(pane.pane_id())
    }

    fn swap_active_with_session_terminal_instance(
        &self,
        pane: &Arc<dyn OverlayPane>,
        keep_focus: bool,
    ) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        crate::chatminal_runtime::desktop_swap_active_with_terminal_handle(
            self.window_id as DesktopWindowId,
            crate::chatminal_runtime::SessionTerminalHandle::new(pane.pane_id() as u64),
            keep_focus,
        )
    }

    fn resolve_public_terminal_instance(&self, public_id: u64) -> Option<Arc<dyn OverlayPane>> {
        if let Some(pane) = self
            .active_terminal_instance_or_overlay()
            .filter(|pane| pane_matches_public_id(&**pane, public_id))
        {
            return Some(pane);
        }

        if let Some(pane) = self.terminal_handle_arc(public_id) {
            return Some(pane);
        }

        self.with_host_window(|window| {
            for tab in window.iter() {
                for pos in tab.iter_panes_ignoring_zoom() {
                    if pane_matches_public_id(&*pos.pane, public_id) {
                        return Some(pos.pane.clone());
                    }
                }
            }
            None
        })
        .flatten()
    }

    pub(crate) fn is_session_ui_mode(&self) -> bool {
        self.chatminal_sidebar.is_enabled()
    }

    fn render_target_overlay(&self, render_scope_id: u64) -> Option<Arc<dyn OverlayPane>> {
        self.runtime_ui_state(render_scope_id)
            .overlay
            .as_ref()
            .map(|overlay| overlay.pane.clone())
    }

    fn active_render_target_overlay(&self) -> Option<Arc<dyn OverlayPane>> {
        let render_scope_id = self.active_render_scope_id()?;
        self.render_target_overlay(render_scope_id)
    }

    fn active_runtime_has_overlay(&self) -> bool {
        self.active_render_target_overlay().is_some()
    }

    fn spawn_overlay_for_render_scope_capability<T, F>(
        &mut self,
        render_scope_id: u64,
        scope_size: TerminalSize,
        func: F,
    )
    where
        T: Send + 'static,
        F: Send + 'static
            + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
    {
        let (overlay, future) = start_overlay(self, render_scope_id, scope_size, func);
        self.assign_overlay_for_render_scope(render_scope_id, overlay);
        promise::spawn::spawn(future).detach();
    }

    fn spawn_overlay_on_active_render_scope<T, F>(&mut self, func: F) -> bool
    where
        T: Send + 'static,
        F: Send + 'static
            + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
    {
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return false;
        };
        let Some(scope_size) = self.render_scope_size(render_scope_id) else {
            return false;
        };
        self.spawn_overlay_for_render_scope_capability(render_scope_id, scope_size, func);
        true
    }

    fn spawn_overlay_for_render_scope<T, F>(&mut self, render_scope_id: u64, func: F) -> bool
    where
        T: Send + 'static,
        F: Send + 'static
            + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
    {
        let Some(scope_size) = self.render_scope_size(render_scope_id) else {
            return false;
        };
        self.spawn_overlay_for_render_scope_capability(render_scope_id, scope_size, func);
        true
    }

    fn with_render_scope_capability_by_id<R, F>(&self, render_scope_id: u64, func: F) -> Option<R>
    where
        F: FnOnce(&Arc<OverlayRenderScope>) -> R,
    {
        let tab = self.render_scope_capability(render_scope_id)?;
        Some(func(&tab))
    }

    fn with_active_render_scope_capability<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&Arc<OverlayRenderScope>) -> R,
    {
        let render_scope_id = self.active_render_scope_id()?;
        self.with_render_scope_capability_by_id(render_scope_id, func)
    }

    fn with_active_render_scope_capability_if_no_overlay<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&Arc<OverlayRenderScope>) -> R,
    {
        if self.active_runtime_has_overlay() {
            return None;
        }
        self.with_active_render_scope_capability(func)
    }

    fn render_scope_size(&self, render_scope_id: u64) -> Option<TerminalSize> {
        self.with_render_scope_capability_by_id(render_scope_id, |tab| tab.get_size())
    }

    fn resize_render_scope(&self, render_scope_id: u64, size: TerminalSize) -> bool {
        self.with_render_scope_capability_by_id(render_scope_id, |tab| {
            tab.resize(size);
            true
        })
        .unwrap_or(false)
    }

    fn resize_render_scope_split(
        &self,
        render_scope_id: u64,
        split: TerminalSplit,
        delta: isize,
    ) -> Option<TerminalSplit> {
        self.with_render_scope_capability_by_id(render_scope_id, |tab| {
            tab.resize_split_by(split.index, delta);
            crate::chatminal_runtime::overlay_split_layouts(tab)
                .into_iter()
                .nth(split.index)
                .map(TerminalSplit::from_mux)
        })
        .flatten()
    }

    pub(crate) fn set_active_runtime_zoomed(&self, zoomed: bool) -> Option<bool> {
        self.with_active_render_scope_capability(|tab| tab.set_zoomed(zoomed))
    }

    pub(crate) fn toggle_active_runtime_zoom(&self) -> bool {
        self.with_active_render_scope_capability(|tab| tab.toggle_zoom())
            .is_some()
    }

    pub(crate) fn adjust_active_terminal_size(
        &self,
        direction: SessionDirection,
        amount: usize,
    ) -> bool {
        self.with_active_render_scope_capability_if_no_overlay(|tab| {
            tab.adjust_pane_size(direction, amount);
        })
        .is_some()
    }

    pub(crate) fn rotate_active_terminals(&self, direction: RotationDirection) -> bool {
        self.with_active_render_scope_capability(|tab| match direction {
            RotationDirection::Clockwise => tab.rotate_clockwise(),
            RotationDirection::CounterClockwise => tab.rotate_counter_clockwise(),
        })
        .is_some()
    }

    pub(crate) fn activate_terminal_handle_in_active_runtime(&self, terminal_handle: TerminalUiKey) -> bool {
        self.with_active_render_scope_capability(|tab| {
            tab.iter_panes()
                .iter()
                .position(|pos| pos.pane.pane_id() as u64 == terminal_handle)
                .map(|tab_index| {
                    tab.set_active_idx(tab_index);
                })
                .is_some()
        })
        .unwrap_or(false)
    }

    pub(crate) fn swap_active_with_terminal_handle_in_active_runtime(
        &self,
        terminal_handle: TerminalUiKey,
        keep_focus: bool,
    ) -> bool {
        self.with_active_render_scope_capability(|tab| {
            tab.iter_panes()
                .iter()
                .position(|pos| pos.pane.pane_id() as u64 == terminal_handle)
                .and_then(|tab_index| tab.swap_active_with_index(tab_index, keep_focus))
                .is_some()
        })
        .unwrap_or(false)
    }

    fn active_terminal_instance_from_active_render_target(&self) -> Option<Arc<dyn OverlayPane>> {
        self.with_active_render_scope_capability(|tab| tab.get_active_pane())
            .flatten()
    }

    fn active_render_target_contains_terminal(&self, pane_id: TerminalUiKey) -> bool {
        let Some(pane_id) = crate::desktop_termwindow_types::pane_id_from_terminal_ui_key(pane_id)
        else {
            return false;
        };
        self.with_active_render_scope_capability(|tab| tab.contains_pane(pane_id))
            .unwrap_or(false)
    }

    fn activate_terminal_index_in_active_render_target(&self, index: usize) -> bool {
        self.with_active_render_scope_capability(|tab| {
            let panes = tab.iter_panes();
            if panes.iter().position(|p| p.index == index).is_some() {
                tab.set_active_idx(index);
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
    }

    fn activate_terminal_direction_in_active_render_target(&self, direction: SessionDirection) -> bool {
        self.with_active_render_scope_capability(|tab| {
            tab.activate_pane_direction(direction);
            true
        })
        .unwrap_or(false)
    }

    fn active_render_target_splits(&self) -> Vec<TerminalSplit> {
        self.with_active_render_scope_capability(|tab| {
            crate::chatminal_runtime::overlay_split_layouts(tab)
        })
            .map(|splits| splits.into_iter().map(TerminalSplit::from_mux).collect())
            .unwrap_or_default()
    }

    fn active_render_target_positioned_panes(&self) -> Vec<TerminalPaneLayout> {
        self.active_render_scope_id()
            .map(|render_scope_id| self.get_positioned_panes_for_render_scope(render_scope_id))
            .unwrap_or_default()
    }
}
