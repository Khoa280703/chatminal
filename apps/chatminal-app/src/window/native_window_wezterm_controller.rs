use std::time::{Duration, Instant};

use crate::config::InputPipelineMode;
use crate::ipc::ChatminalClient;
use crate::terminal_pane_adapter::SessionPaneRegistry;
use crate::terminal_workspace_binding_runtime::{
    apply_event_to_workspace_binding_state, bootstrap_workspace_binding_state,
};

use super::ChatminalWindowApp;
use super::input_worker::TerminalInputWorker;
use super::reducer::choose_selected_session_id;

impl ChatminalWindowApp {
    pub(super) fn new(
        endpoint: &str,
        preview_lines: usize,
        cols: usize,
        rows: usize,
        input_pipeline_mode: InputPipelineMode,
    ) -> Result<Self, String> {
        let client = ChatminalClient::connect(endpoint)?;
        let mut pane_registry = SessionPaneRegistry::new();
        let state = bootstrap_workspace_binding_state(
            &client,
            &mut pane_registry,
            preview_lines,
            cols,
            rows,
        )?;
        let selected_session_id = state.workspace.active_session_id.clone().or_else(|| {
            state
                .workspace
                .sessions
                .first()
                .map(|value| value.session_id.clone())
        });

        Ok(Self {
            endpoint: endpoint.to_string(),
            client,
            pane_registry,
            state,
            selected_session_id,
            preview_lines,
            pane_cols: cols,
            pane_rows: rows,
            new_session_name: String::new(),
            pending_session_create: None,
            last_error: None,
            input_worker: TerminalInputWorker::spawn(endpoint),
            cached_terminal_text: String::new(),
            render_dirty: true,
            pending_resize: None,
            last_resize_request_at: Instant::now()
                .checked_sub(Duration::from_millis(120))
                .unwrap_or_else(Instant::now),
            terminal_has_focus: true,
            ime_blur_flush_armed: false,
            ime_composition_state: Default::default(),
            ime_commit_deduper: Default::default(),
            input_pipeline_mode,
            legacy_ready_marker_written: false,
        })
    }

    pub(super) fn poll_daemon_events(&mut self) -> bool {
        let mut changed = false;
        for _ in 0..128 {
            match self.client.recv_event(Duration::from_millis(0)) {
                Ok(Some(event)) => {
                    apply_event_to_workspace_binding_state(
                        &mut self.state,
                        &mut self.pane_registry,
                        event,
                    );
                    changed = true;
                }
                Ok(None) => return changed,
                Err(err) => {
                    self.last_error = Some(format!("event stream error: {err}"));
                    return changed;
                }
            }
        }
        changed
    }

    pub(super) fn reload_workspace(&mut self) {
        match bootstrap_workspace_binding_state(
            &self.client,
            &mut self.pane_registry,
            self.preview_lines,
            self.pane_cols,
            self.pane_rows,
        ) {
            Ok(next_state) => {
                self.state = next_state;
                self.last_error = None;
                self.render_dirty = true;
                self.normalize_selection();
            }
            Err(err) => self.last_error = Some(format!("reload workspace failed: {err}")),
        }
    }

    pub(super) fn refresh_cached_terminal_text(&mut self) {
        if !self.render_dirty {
            return;
        }
        self.cached_terminal_text = self.active_pane_text();
        self.render_dirty = false;
    }

    fn normalize_selection(&mut self) {
        let session_ids = self
            .state
            .workspace
            .sessions
            .iter()
            .map(|value| value.session_id.clone())
            .collect::<Vec<_>>();
        self.selected_session_id = choose_selected_session_id(
            &session_ids,
            self.selected_session_id.as_deref(),
            self.state.workspace.active_session_id.as_deref(),
        );
    }

    fn active_pane_text(&self) -> String {
        let session_id = self.selected_session_id.as_deref().or(self
            .state
            .workspace
            .active_session_id
            .as_deref());
        let Some(session_id) = session_id else {
            return String::new();
        };
        let Some(pane_id) = self.pane_registry.pane_for_session(session_id) else {
            return String::new();
        };
        self.state
            .adapter
            .pane_snapshot(pane_id)
            .map(|value| value.visible_text)
            .unwrap_or_default()
    }
}
