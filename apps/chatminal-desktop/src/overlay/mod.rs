use crate::termwindow::TermWindow;
use engine_term::{TerminalConfiguration, TerminalSize};
use mux::pane::Pane;
use mux::tab::Tab;
use mux::termwiztermtab::{TermWizTerminal, allocate};
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
    confirm_close_pane, confirm_close_tab, confirm_close_window, confirm_quit_program,
};
pub use copy::{CopyModeParams, CopyOverlay};
pub use debug::show_debug_overlay;
pub use launcher::{LauncherArgs, LauncherFlags, launcher};
pub use quickselect::QuickSelectOverlay;

pub fn start_overlay<T, F>(
    term_window: &TermWindow,
    tab: &Arc<Tab>,
    func: F,
) -> (
    Arc<dyn Pane>,
    Pin<Box<dyn std::future::Future<Output = anyhow::Result<T>>>>,
)
where
    T: Send + 'static,
    F: Send + 'static + FnOnce(u64, TermWizTerminal) -> anyhow::Result<T>,
{
    let tab_id = tab.tab_id();
    let tab_size = tab.get_size();
    let term_config: Arc<dyn TerminalConfiguration + Send + Sync> =
        Arc::new(config::TermConfig::with_config(term_window.config.clone()));
    let (tw_term, tw_tab) = allocate(tab_size, term_config);

    let window = term_window.window.clone().unwrap();
    let surface_id = term_window.active_surface_id();

    let overlay_pane_id = tw_tab.pane_id();

    let future = promise::spawn::spawn_into_new_thread(move || {
        let res = func(tab_id as u64, tw_term);
        if let Some(surface_id) = surface_id {
            TermWindow::schedule_cancel_overlay_for_surface(
                window,
                surface_id,
                Some(overlay_pane_id as u64),
            );
        } else {
            TermWindow::schedule_cancel_overlay_for_host_surface(
                window,
                tab_id as u64,
                Some(overlay_pane_id as u64),
            );
        }
        res
    });

    (tw_tab, Box::pin(future))
}

pub fn start_overlay_pane<T, F>(
    term_window: &TermWindow,
    pane: &Arc<dyn Pane>,
    func: F,
) -> (
    Arc<dyn Pane>,
    Pin<Box<dyn std::future::Future<Output = anyhow::Result<T>>>>,
)
where
    T: Send + 'static,
    F: Send + 'static + FnOnce(u64, TermWizTerminal) -> anyhow::Result<T>,
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
    let (tw_term, tw_tab) = allocate(size, term_config);

    let window = term_window.window.clone().unwrap();

    let future = promise::spawn::spawn_into_new_thread(move || {
        let res = func(pane_id as u64, tw_term);
        TermWindow::schedule_cancel_overlay_for_leaf(window, pane_id as u64);
        res
    });

    (tw_tab, Box::pin(future))
}
