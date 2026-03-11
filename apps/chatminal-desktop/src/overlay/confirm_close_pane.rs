use super::confirm;
use crate::TermWindow;
use mux::Mux;
use mux::pane::PaneId;
use mux::tab::TabId;
use mux::termwiztermtab::TermWizTerminal;
use mux::window::WindowId;
use std::convert::TryFrom;

fn pane_id_from_u64(pane_id: u64) -> anyhow::Result<PaneId> {
    PaneId::try_from(pane_id).map_err(|_| anyhow::anyhow!("invalid pane id {pane_id}"))
}

fn tab_id_from_u64(tab_id: u64) -> anyhow::Result<TabId> {
    TabId::try_from(tab_id).map_err(|_| anyhow::anyhow!("invalid tab id {tab_id}"))
}

fn window_id_from_u64(window_id: u64) -> anyhow::Result<WindowId> {
    WindowId::try_from(window_id).map_err(|_| anyhow::anyhow!("invalid window id {window_id}"))
}

pub fn confirm_close_pane(
    pane_id: u64,
    mut term: TermWizTerminal,
    window: ::window::Window,
) -> anyhow::Result<()> {
    let pane_id = pane_id_from_u64(pane_id)?;
    if confirm::run_confirmation("🛑 Really kill this pane?", &mut term)? {
        promise::spawn::spawn_into_main_thread(async move {
            let mux = Mux::get();
            mux.remove_pane(pane_id);
        })
        .detach();
    }
    TermWindow::schedule_cancel_overlay_for_leaf(window, pane_id as u64);

    Ok(())
}

pub fn confirm_close_tab(
    tab_id: u64,
    mut term: TermWizTerminal,
) -> anyhow::Result<()> {
    let tab_id = tab_id_from_u64(tab_id)?;
    if confirm::run_confirmation(
        "🛑 Really kill this tab and all contained panes?",
        &mut term,
    )? {
        promise::spawn::spawn_into_main_thread(async move {
            let mux = Mux::get();
            mux.remove_tab(tab_id);
        })
        .detach();
    }

    Ok(())
}

pub fn confirm_close_window(
    mut term: TermWizTerminal,
    window_id: u64,
) -> anyhow::Result<()> {
    let window_id = window_id_from_u64(window_id)?;
    if confirm::run_confirmation(
        "🛑 Really kill this window and all contained tabs and panes?",
        &mut term,
    )? {
        promise::spawn::spawn_into_main_thread(async move {
            let mux = Mux::get();
            mux.kill_window(window_id);
        })
        .detach();
    }

    Ok(())
}

pub fn confirm_quit_program(mut term: TermWizTerminal) -> anyhow::Result<()> {
    if confirm::run_confirmation("🛑 Really Quit Chatminal?", &mut term)? {
        promise::spawn::spawn_into_main_thread(async move {
            use ::window::{Connection, ConnectionOps};
            let con = Connection::get().expect("call on gui thread");
            con.terminate_message_loop();
        })
        .detach();
    }

    Ok(())
}
