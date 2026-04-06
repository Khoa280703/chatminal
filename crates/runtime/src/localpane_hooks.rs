use crate::pane::PaneId;
use config::ExitBehavior;
use std::sync::Arc;
use terminal_emulator::Alert;

#[derive(Clone)]
pub struct LocalPaneHooks {
    on_input: Arc<dyn Fn() + Send + Sync>,
    on_inline_output: Arc<dyn Fn(PaneId, String) + Send + Sync>,
    on_alert: Arc<dyn Fn(PaneId, Alert) + Send + Sync>,
    on_child_exit_cleanup: Arc<dyn Fn(PaneId, ExitBehavior) + Send + Sync>,
}

impl LocalPaneHooks {
    pub fn noop() -> Self {
        Self {
            on_input: Arc::new(|| {}),
            on_inline_output: Arc::new(|_, _| {}),
            on_alert: Arc::new(|_, _| {}),
            on_child_exit_cleanup: Arc::new(|_, _| {}),
        }
    }

    pub fn record_input(&self) {
        (self.on_input)();
    }

    pub fn emit_inline_output(&self, pane_id: PaneId, message: String) {
        (self.on_inline_output)(pane_id, message);
    }

    pub fn emit_alert(&self, pane_id: PaneId, alert: Alert) {
        (self.on_alert)(pane_id, alert);
    }

    pub fn run_child_exit_cleanup(&self, pane_id: PaneId, exit_behavior: ExitBehavior) {
        (self.on_child_exit_cleanup)(pane_id, exit_behavior);
    }

    pub fn set_input(&mut self, on_input: Arc<dyn Fn() + Send + Sync>) {
        self.on_input = on_input;
    }

    pub fn set_inline_output(
        &mut self,
        on_inline_output: Arc<dyn Fn(PaneId, String) + Send + Sync>,
    ) {
        self.on_inline_output = on_inline_output;
    }

    pub fn set_alert(&mut self, on_alert: Arc<dyn Fn(PaneId, Alert) + Send + Sync>) {
        self.on_alert = on_alert;
    }

    pub fn set_child_exit_cleanup(
        &mut self,
        on_child_exit_cleanup: Arc<dyn Fn(PaneId, ExitBehavior) + Send + Sync>,
    ) {
        self.on_child_exit_cleanup = on_child_exit_cleanup;
    }
}

impl Default for LocalPaneHooks {
    fn default() -> Self {
        Self::noop()
    }
}
