use super::confirm;
use crate::chatminal_runtime::remove_runtime_entry_scope;
use crate::desktop_host_runtime::overlay_shell::OverlayTerminal;

pub fn confirm_close_tab(tab_id: u64, mut term: OverlayTerminal) -> anyhow::Result<()> {
    if confirm::run_confirmation(
        "🛑 Really close this session layout and all contained session views?",
        &mut term,
    )? {
        promise::spawn::spawn_into_main_thread(async move {
            remove_runtime_entry_scope(tab_id);
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
