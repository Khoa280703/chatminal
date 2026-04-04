use crate::chatminal_runtime::{
    DESKTOP_LAYOUT_WORKSPACE_ID, SessionRenderTargetId, desktop_focus_session_terminal_handle,
    desktop_pane_for_session, desktop_render_state_for_session,
    desktop_session_entry_binding_for_render_target, desktop_session_id_for_terminal_handle,
    desktop_session_terminal_binding,
};
use crate::desktop_host_runtime::{
    focus_terminal_handle_by_id, host_window_contains_render_scope, host_window_exists,
    host_workspace_name, remove_runtime_entry_scope, remove_terminal_handle, terminal_handle_arc,
    terminal_handle_for_pane as terminal_handle_for_overlay_pane, with_host_window,
    with_host_window_mut,
};

impl TermWindow {
    pub(crate) fn active_terminal_handle(&self) -> Option<u64> {
        if self.chatminal_sidebar.is_enabled() {
            return self
                .active_session_id()
                .as_deref()
                .and_then(desktop_render_state_for_session)
                .and_then(|render_state| {
                    render_state
                        .panes
                        .iter()
                        .find(|pane| pane.is_active)
                        .or_else(|| {
                            render_state.active_terminal_instance_id.and_then(
                                |active_terminal_instance_id| {
                                    render_state.panes.iter().find(|pane| {
                                        pane.terminal_instance_id == active_terminal_instance_id
                                    })
                                },
                            )
                        })
                        .map(|pane| pane.terminal_handle.as_u64())
                });
        }
        self.active_terminal_instance_from_active_render_target()
            .map(|pane| terminal_ui_key_for_pane(&*pane))
    }

    pub(crate) fn active_workspace_name(&self) -> String {
        if self.chatminal_sidebar.is_enabled() {
            return self
                .active_session_id()
                .unwrap_or_else(|| DESKTOP_LAYOUT_WORKSPACE_ID.to_string());
        }
        Self::host_workspace_name()
    }

    fn terminal_handle_arc(&self, pane_id: TerminalUiKey) -> Option<Arc<dyn OverlayPane>> {
        terminal_handle_arc(crate::desktop_termwindow_types::terminal_handle_for_ui_key(
            pane_id,
        ))
    }

    fn remove_terminal_handle(&self, pane_id: TerminalUiKey) {
        remove_terminal_handle(crate::desktop_termwindow_types::terminal_handle_for_ui_key(
            pane_id,
        ));
    }

    fn with_host_window<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&RuntimeWindow) -> R,
    {
        with_host_window(func)
    }

    fn with_host_window_mut<R, F>(&self, func: F) -> Option<R>
    where
        F: FnOnce(&mut RuntimeWindow) -> R,
    {
        with_host_window_mut(func)
    }

    fn remove_runtime_entry_scope(&self, render_scope_id: u64) {
        remove_runtime_entry_scope(render_scope_id);
    }

    fn host_window_exists() -> bool {
        host_window_exists()
    }

    fn host_window_contains_render_scope(render_scope_id: u64) -> bool {
        host_window_contains_render_scope(render_scope_id)
    }

    fn host_workspace_name() -> String {
        host_workspace_name()
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

    fn focus_active_session_terminal_instance(&mut self, pane: &Arc<dyn OverlayPane>) -> bool {
        if !self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let terminal_handle = terminal_handle_for_overlay_pane(&**pane);
        let focused = desktop_focus_session_terminal_handle(terminal_handle).is_some();
        if focused {
            log::warn!(
                "focus_active_session_terminal_instance: pane={} resolved_via_handle_focus",
                terminal_handle.as_u64()
            );
            if let Some(binding) = desktop_session_terminal_binding(terminal_handle) {
                if let Err(err) =
                    notify_runtime_session_activated(&binding.session_id, binding.runtime_id)
                {
                    log::error!(
                        "failed to notify runtime bridge about pane-focus activation: {err}"
                    );
                }
                self.chatminal_sidebar
                    .set_active_session_local(&binding.session_id);
                self.chatminal_sidebar
                    .set_session_status_local(&binding.session_id, "running");
                self.chatminal_sidebar
                    .select_single_session(&binding.session_id);
            }
            return true;
        }
        let Some(session_id) = desktop_session_id_for_terminal_handle(terminal_handle) else {
            return false;
        };
        self.activate_chatminal_session_target(&session_id, None)
            .is_some()
    }

    fn focus_terminal_handle(&self, pane: &Arc<dyn OverlayPane>) -> bool {
        focus_terminal_handle_by_id(terminal_handle_for_overlay_pane(&**pane)).is_ok()
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

    fn session_render_target_overlay(&self, session_id: &str) -> Option<Arc<dyn OverlayPane>> {
        if !self.chatminal_sidebar.is_enabled() {
            return None;
        }
        let render_scope_id = desktop_render_state_for_session(session_id)?
            .render_target_id()
            .as_u64();
        self.render_target_overlay(render_scope_id)
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
    ) where
        T: Send + 'static,
        F: Send + 'static + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
    {
        let (overlay, future) = start_overlay(self, render_scope_id, scope_size, func);
        self.assign_overlay_for_render_scope(render_scope_id, overlay);
        promise::spawn::spawn(async move {
            if let Err(err) = future.await {
                log::error!("overlay failed for render_scope_id={render_scope_id}: {err:#}");
            }
        })
        .detach();
    }

    fn spawn_overlay_on_active_render_scope<T, F>(&mut self, func: F) -> bool
    where
        T: Send + 'static,
        F: Send + 'static + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
    {
        let Some(render_scope_id) = self.active_render_scope_id() else {
            log::error!(
                "spawn_overlay_on_active_render_scope: missing active render scope for active_session={:?}",
                self.active_session_id()
            );
            return false;
        };
        let Some(scope_size) = self.render_scope_size(render_scope_id) else {
            log::error!(
                "spawn_overlay_on_active_render_scope: missing render scope size for render_scope_id={} active_session={:?}",
                render_scope_id,
                self.active_session_id()
            );
            return false;
        };
        self.spawn_overlay_for_render_scope_capability(render_scope_id, scope_size, func);
        true
    }

    fn spawn_overlay_for_render_scope<T, F>(&mut self, render_scope_id: u64, func: F) -> bool
    where
        T: Send + 'static,
        F: Send + 'static + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
    {
        let Some(scope_size) = self.render_scope_size(render_scope_id) else {
            return false;
        };
        self.spawn_overlay_for_render_scope_capability(render_scope_id, scope_size, func);
        true
    }

    fn session_id_for_render_target(&self, render_scope_id: u64) -> Option<String> {
        if !self.chatminal_sidebar.is_enabled() {
            return None;
        }
        desktop_session_entry_binding_for_render_target(SessionRenderTargetId::new(render_scope_id))
            .map(|entry| entry.session_id)
    }

    fn session_render_scope_id(&self, session_id: &str) -> Option<u64> {
        if !self.chatminal_sidebar.is_enabled() {
            return None;
        }
        desktop_render_state_for_session(session_id).map(|state| state.render_target_id().as_u64())
    }

    fn session_pane_for_render_target(&self, render_scope_id: u64) -> Option<Arc<dyn OverlayPane>> {
        let session_id = self.session_id_for_render_target(render_scope_id)?;
        desktop_pane_for_session(&session_id).map(|pane| pane as Arc<dyn OverlayPane>)
    }

    fn reconcile_active_session_focus_from_runtime_lookup(&self) -> Option<String> {
        if !self.chatminal_sidebar.is_enabled() {
            return None;
        }
        let lookup = match desktop_session_window_snapshot() {
            Ok(snapshot) => snapshot.lookup,
            Err(err) => {
                log::error!("failed to load desktop session window snapshot: {err}");
                return None;
            }
        };
        match reconcile_runtime_session_lookup(&lookup) {
            Ok(action) => match action {
                DesktopSessionBridgeAction::FocusSession { session_id } => Some(session_id),
                DesktopSessionBridgeAction::Noop => None,
            },
            Err(err) => {
                log::error!("failed to reconcile session lookup: {err}");
                None
            }
        }
    }

    fn render_scope_can_close_without_prompting_via_host(
        &self,
        render_scope_id: u64,
        reason: CloseReason,
    ) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            return self
                .session_pane_for_render_target(render_scope_id)
                .map(|pane| pane.can_close_without_prompting(reason))
                .unwrap_or(false);
        }
        crate::desktop_host_runtime::host_render_scope_can_close_without_prompting(
            render_scope_id,
            reason,
        )
    }

    fn render_scope_size(&self, render_scope_id: u64) -> Option<TerminalSize> {
        if self.chatminal_sidebar.is_enabled() {
            let session_id = self.session_id_for_render_target(render_scope_id)?;
            return desktop_render_state_for_session(&session_id).map(|state| state.terminal_size);
        }
        crate::desktop_host_runtime::host_render_scope_size(render_scope_id)
    }

    fn resize_render_scope(&self, render_scope_id: u64, size: TerminalSize) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            return false;
        }
        crate::desktop_host_runtime::host_resize_render_scope(render_scope_id, size)
    }

    fn resize_render_scope_split(
        &self,
        render_scope_id: u64,
        split: TerminalSplit,
        delta: isize,
    ) -> Option<TerminalSplit> {
        if self.chatminal_sidebar.is_enabled() {
            return None;
        }
        crate::desktop_host_runtime::host_resize_render_scope_split(
            render_scope_id,
            split.index,
            delta,
        )
        .map(TerminalSplit::from_mux)
    }

    pub(crate) fn set_active_runtime_zoomed(&self, zoomed: bool) -> Option<bool> {
        if self.chatminal_sidebar.is_enabled() {
            return Some(false);
        }
        let render_scope_id = self.active_render_scope_id()?;
        crate::desktop_host_runtime::host_set_render_scope_zoomed(render_scope_id, zoomed)
    }

    pub(crate) fn toggle_active_runtime_zoom(&self) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return false;
        };
        crate::desktop_host_runtime::host_toggle_render_scope_zoom(render_scope_id)
    }

    pub(crate) fn adjust_active_terminal_size(
        &self,
        direction: SessionDirection,
        amount: usize,
    ) -> bool {
        if self.active_runtime_has_overlay() {
            return false;
        }
        if self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return false;
        };
        crate::desktop_host_runtime::host_adjust_render_scope_terminal_size(
            render_scope_id,
            direction,
            amount,
        )
    }

    pub(crate) fn rotate_active_terminals(&self, direction: RotationDirection) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            return false;
        }
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return false;
        };
        crate::desktop_host_runtime::host_rotate_render_scope_terminals(render_scope_id, direction)
    }

    pub(crate) fn swap_active_with_terminal_handle_in_active_runtime(
        &self,
        terminal_handle: TerminalUiKey,
        keep_focus: bool,
    ) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            let _ = (terminal_handle, keep_focus);
            return false;
        }
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return false;
        };
        crate::desktop_host_runtime::host_swap_active_with_terminal_handle_in_render_scope(
            render_scope_id,
            terminal_handle,
            keep_focus,
        )
    }

    fn active_terminal_instance_from_active_render_target(&self) -> Option<Arc<dyn OverlayPane>> {
        let render_scope_id = self.active_render_scope_id()?;
        if self.chatminal_sidebar.is_enabled() {
            return self.session_pane_for_render_target(render_scope_id);
        }
        crate::desktop_host_runtime::host_active_terminal_in_render_scope(render_scope_id)
    }

    fn activate_terminal_index_in_active_render_target(&self, index: usize) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            return index == 0
                && self
                    .active_terminal_instance_from_active_render_target()
                    .is_some();
        }
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return false;
        };
        crate::desktop_host_runtime::host_activate_terminal_index_in_render_scope(
            render_scope_id,
            index,
        )
    }

    fn activate_terminal_direction_in_active_render_target(
        &self,
        direction: SessionDirection,
    ) -> bool {
        if self.chatminal_sidebar.is_enabled() {
            let _ = direction;
            return false;
        }
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return false;
        };
        crate::desktop_host_runtime::host_activate_terminal_direction_in_render_scope(
            render_scope_id,
            direction,
        )
    }

    fn active_render_target_splits(&self) -> Vec<TerminalSplit> {
        if self.chatminal_sidebar.is_enabled() {
            return vec![];
        }
        let Some(render_scope_id) = self.active_render_scope_id() else {
            return vec![];
        };
        crate::desktop_host_runtime::host_overlay_split_layouts_by_id(render_scope_id)
            .into_iter()
            .map(TerminalSplit::from_mux)
            .collect()
    }

    fn active_render_target_positioned_panes(&self) -> Vec<TerminalPaneLayout> {
        self.active_render_scope_id()
            .map(|render_scope_id| self.get_positioned_panes_for_render_scope(render_scope_id))
            .unwrap_or_default()
    }
}
