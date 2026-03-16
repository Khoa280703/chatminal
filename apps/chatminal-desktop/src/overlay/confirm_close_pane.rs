use super::confirm;
use crate::termwindow::TermWindowNotif;
use crate::TermWindow;
use crate::chatminal_runtime::overlay_compat::OverlayTerminal;
use window::WindowOps;

pub fn confirm_close_pane(
    pane_id: u64,
    mut term: OverlayTerminal,
    window: ::window::Window,
) -> anyhow::Result<()> {
    if confirm::run_confirmation("🛑 Really kill this pane?", &mut term)? {
        promise::spawn::spawn_into_main_thread(async move {
            if let Err(err) = crate::chatminal_runtime::remove_terminal_handle_by_public_id(pane_id) {
                log::error!("failed to remove host pane {pane_id}: {err:#}");
            }
        })
        .detach();
    }
    TermWindow::schedule_cancel_overlay_for_terminal_handle(window, pane_id);

    Ok(())
}

pub fn confirm_close_chatminal_session_leaf_or_session(
    session_id: String,
    host_terminal_handle: u64,
    mut term: OverlayTerminal,
    window: ::window::Window,
) -> anyhow::Result<()> {
    if confirm::run_confirmation("🛑 Really kill this pane?", &mut term)? {
        let window_for_apply = window.clone();
        promise::spawn::spawn_into_main_thread(async move {
            window_for_apply.notify(TermWindowNotif::Apply(Box::new(move |term_window| {
                term_window.close_chatminal_terminal_handle_or_session(
                    &session_id,
                    host_terminal_handle,
                );
            })));
        })
        .detach();
    }
    TermWindow::schedule_cancel_overlay_for_terminal_handle(window, host_terminal_handle);
    Ok(())
}

pub fn confirm_close_tab(tab_id: u64, mut term: OverlayTerminal) -> anyhow::Result<()> {
    if confirm::run_confirmation(
        "🛑 Really kill this tab and all contained panes?",
        &mut term,
    )? {
        promise::spawn::spawn_into_main_thread(async move {
            crate::chatminal_runtime::remove_runtime_entry_scope(tab_id);
        })
        .detach();
    }

    Ok(())
}

pub fn confirm_close_window(mut term: OverlayTerminal, window_id: u64) -> anyhow::Result<()> {
    if confirm::run_confirmation(
        "🛑 Really kill this window and all contained tabs and panes?",
        &mut term,
    )? {
        promise::spawn::spawn_into_main_thread(async move {
            if let Err(err) = crate::chatminal_runtime::kill_host_window_by_public_id(window_id) {
                log::error!("failed to kill host window {window_id}: {err:#}");
            }
        })
        .detach();
    }

    Ok(())
}

pub fn confirm_quit_program(mut term: OverlayTerminal) -> anyhow::Result<()> {
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
