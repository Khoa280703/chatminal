use super::confirm;
use crate::chatminal_runtime::overlay_compat::OverlayTerminal;

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
