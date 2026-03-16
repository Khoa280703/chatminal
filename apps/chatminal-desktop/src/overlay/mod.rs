use crate::termwindow::TermWindow;
use engine_term::{TerminalConfiguration, TerminalSize};
use crate::chatminal_runtime::overlay_compat::OverlayPane;
use crate::chatminal_runtime::overlay_compat::OverlayRenderScope;
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
    confirm_close_chatminal_session_leaf_or_session, confirm_close_pane, confirm_close_tab,
    confirm_close_window, confirm_quit_program,
};
pub use copy::{CopyModeParams, CopyOverlay};
pub use debug::show_debug_overlay;
pub use launcher::{launcher, LauncherArgs, LauncherFlags};
pub use quickselect::QuickSelectOverlay;

pub fn start_overlay<T, F>(
    term_window: &TermWindow,
    tab: &Arc<OverlayRenderScope>,
    func: F,
) -> (
    Arc<dyn OverlayPane>,
    Pin<Box<dyn std::future::Future<Output = anyhow::Result<T>>>>,
)
where
    T: Send + 'static,
    F: Send + 'static + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
{
    let tab_id = tab.tab_id();
    let tab_size = tab.get_size();
    let term_config: Arc<dyn TerminalConfiguration + Send + Sync> =
        Arc::new(config::TermConfig::with_config(term_window.config.clone()));
    let (tw_term, tw_tab) = allocate_overlay_terminal(tab_size, term_config);

    let window = term_window.window.clone().unwrap();

    let overlay_pane_id = tw_tab.pane_id();

    let future = promise::spawn::spawn_into_new_thread(move || {
        let res = func(tab_id as u64, tw_term);
        TermWindow::schedule_cancel_overlay_for_render_scope(
            window,
            tab_id as u64,
            Some(overlay_pane_id as u64),
        );
        res
    });

    (tw_tab, Box::pin(future))
}

pub fn start_overlay_pane<T, F>(
    term_window: &TermWindow,
    pane: &Arc<dyn OverlayPane>,
    func: F,
) -> (
    Arc<dyn OverlayPane>,
    Pin<Box<dyn std::future::Future<Output = anyhow::Result<T>>>>,
)
where
    T: Send + 'static,
    F: Send + 'static + FnOnce(u64, OverlayTerminal) -> anyhow::Result<T>,
{
    let pane_id = pane.pane_id();
    let dims = pane.get_dimensions();
    let size = TerminalSize {
        cols: dims.cols,
        rows: dims.viewport_rows,
        pixel_width: term_window.render_metrics.cell_size.width as usize * dims.cols,
        pixel_height: term_window.render_metrics.cell_size.height as usize * dims.viewport_rows,
        dpi: dims.dpi,
    };
    let term_config: Arc<dyn TerminalConfiguration + Send + Sync> =
        Arc::new(config::TermConfig::with_config(term_window.config.clone()));
    let (tw_term, tw_tab) = allocate_overlay_terminal(size, term_config);

    let window = term_window.window.clone().unwrap();

    let future = promise::spawn::spawn_into_new_thread(move || {
        let res = func(pane_id as u64, tw_term);
        TermWindow::schedule_cancel_overlay_for_terminal_handle(window, pane_id as u64);
        res
    });

    (tw_tab, Box::pin(future))
}
