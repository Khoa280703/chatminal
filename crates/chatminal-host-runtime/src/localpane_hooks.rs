use crate::pane::PaneId;
use crate::{
    dispatch_default_output_for_terminal_handle, dispatch_inline_output_for_pane, notify_mux,
    prune_dead_windows_on_main_thread, resolve_pane_id, tab_by_id, with_root_window_mut,
    MuxNotification,
};
use engine_term::Alert;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct LocalPaneHooks {
    on_input: Arc<dyn Fn() + Send + Sync>,
    on_inline_output: Arc<dyn Fn(PaneId, String) + Send + Sync>,
    on_alert: Arc<dyn Fn(PaneId, Alert) + Send + Sync>,
    on_child_exit_cleanup: Arc<dyn Fn() + Send + Sync>,
}

impl LocalPaneHooks {
    pub(crate) fn mux_default() -> Self {
        Self {
            on_input: Arc::new(|| {
                let _ = crate::record_input_for_current_identity();
            }),
            on_inline_output: Arc::new(|pane_id, message| {
                dispatch_inline_output_for_pane(
                    pane_id,
                    message,
                    Arc::new(dispatch_default_output_for_terminal_handle),
                );
            }),
            on_alert: Arc::new(|pane_id, alert| {
                promise::spawn::spawn_into_main_thread(async move {
                    match &alert {
                        Alert::WindowTitleChanged(title) => {
                            let _ = with_root_window_mut(|window| {
                                let owns_pane = window.get_active().map(|active_tab| {
                                    active_tab
                                        .iter_panes_ignoring_zoom()
                                        .into_iter()
                                        .any(|pos| pos.pane.pane_id() == pane_id)
                                });
                                if owns_pane == Some(true) {
                                    window.set_title(title);
                                }
                            });
                        }
                        Alert::TabTitleChanged(title) => {
                            if let Some(tab_id) = resolve_pane_id(pane_id) {
                                if let Some(tab) = tab_by_id(tab_id) {
                                    tab.set_title(title.as_deref().unwrap_or(""));
                                }
                            }
                        }
                        _ => {}
                    }

                    notify_mux(MuxNotification::Alert { pane_id, alert });
                })
                .detach();
            }),
            on_child_exit_cleanup: Arc::new(|| {
                prune_dead_windows_on_main_thread();
            }),
        }
    }

    pub(crate) fn record_input(&self) {
        (self.on_input)();
    }

    pub(crate) fn emit_inline_output(&self, pane_id: PaneId, message: String) {
        (self.on_inline_output)(pane_id, message);
    }

    pub(crate) fn emit_alert(&self, pane_id: PaneId, alert: Alert) {
        (self.on_alert)(pane_id, alert);
    }

    pub(crate) fn prune_dead_windows(&self) {
        (self.on_child_exit_cleanup)();
    }

    pub(crate) fn set_input(&mut self, on_input: Arc<dyn Fn() + Send + Sync>) {
        self.on_input = on_input;
    }

    pub(crate) fn set_inline_output(
        &mut self,
        on_inline_output: Arc<dyn Fn(PaneId, String) + Send + Sync>,
    ) {
        self.on_inline_output = on_inline_output;
    }

    pub(crate) fn set_alert(&mut self, on_alert: Arc<dyn Fn(PaneId, Alert) + Send + Sync>) {
        self.on_alert = on_alert;
    }

    pub(crate) fn set_child_exit_cleanup(
        &mut self,
        on_child_exit_cleanup: Arc<dyn Fn() + Send + Sync>,
    ) {
        self.on_child_exit_cleanup = on_child_exit_cleanup;
    }
}

impl Default for LocalPaneHooks {
    fn default() -> Self {
        Self::mux_default()
    }
}
