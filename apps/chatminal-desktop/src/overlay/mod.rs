use crate::termwindow::TermWindow;
use engine_term::{TerminalConfiguration, TerminalSize};
use crate::chatminal_runtime::overlay_compat::OverlayPane;
use crate::chatminal_runtime::overlay_compat::{allocate_overlay_terminal, OverlayTerminal};
use std::pin::Pin;
use std::sync::Arc;

pub mod confirm;
pub mod confirm_close_pane;
pub mod copy;
pub mod debug;
pub mod launcher;
pub mod prompt;
pub mod quickselect;
pub mod selector;

pub use confirm_close_pane::{
    confirm_close_tab, confirm_close_window, confirm_quit_program,
};
pub use copy::{CopyModeParams, CopyOverlay};
pub use debug::show_debug_overlay;
pub use launcher::{launcher, LauncherArgs, LauncherFlags};
pub use quickselect::QuickSelectOverlay;

pub fn start_overlay<T, F>(
    term_window: &TermWindow,
    scope_id: u64,
    scope_size: TerminalSize,
    func: F,
) -> (
    Arc<dyn OverlayPane>,
    Pin<Box<dyn std::future::Future<Output = anyhow::Result<T>>>>,
)
where
    T: Send + 'static,
    F: Send + 'static + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
{
    let tab_id = scope_id;
    let term_config: Arc<dyn TerminalConfiguration + Send + Sync> =
        Arc::new(config::TermConfig::with_config(term_window.config.clone()));
    let (tw_term, tw_tab) = allocate_overlay_terminal(scope_size, term_config);

    let window = term_window.window.clone().unwrap();

    let overlay_pane_id = tw_tab.pane_id();

    let future = promise::spawn::spawn_into_new_thread(move || {
        let res = func(tab_id, tw_term);
        TermWindow::schedule_cancel_overlay_for_render_scope(
            window,
            tab_id,
            Some(overlay_pane_id as u64),
        );
        res
    });

    (tw_tab, Box::pin(future))
}

