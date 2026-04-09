use super::confirm;
use crate::desktop_session_host::overlay_shell::OverlayTerminal;
use crate::desktop_session_host::remove_runtime_entry_scope;
#[cfg(not(target_os = "macos"))]
use crate::termwindow::TermWindowNotif;
#[cfg(not(target_os = "macos"))]
use window::{Window, WindowOps};

#[cfg(target_os = "macos")]
use cocoa::base::id;
#[cfg(target_os = "macos")]
use cocoa::base::nil;
#[cfg(target_os = "macos")]
use cocoa::foundation::{NSInteger, NSString};
#[cfg(target_os = "macos")]
use objc::*;

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

#[cfg(not(target_os = "macos"))]
pub fn confirm_clear_all_data(window: Window, mut term: OverlayTerminal) -> anyhow::Result<()> {
    if confirm::run_confirmation(
        "🛑 Clear all Chatminal data? This removes profiles, sessions, history, layouts, and local settings.",
        &mut term,
    )? {
        window.notify(TermWindowNotif::Apply(Box::new(move |term_window| {
            term_window.clear_all_chatminal_data();
        })));
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn confirm_clear_all_data_native() -> bool {
    unsafe {
        let nsstring = |value: &str| NSString::alloc(nil).init_str(value);
        let alert: id = msg_send![class!(NSAlert), alloc];
        let alert: id = msg_send![alert, init];
        let message_text = nsstring("Clear All Data?");
        let info_text = nsstring(
            "This removes all local Chatminal data, including profiles, sessions, history, layouts, and local settings.",
        );
        let cancel = nsstring("Cancel");
        let clear = nsstring("Clear All Data");

        let (): () = msg_send![alert, setMessageText: message_text];
        let (): () = msg_send![alert, setInformativeText: info_text];
        let (): () = msg_send![alert, addButtonWithTitle: cancel];
        let (): () = msg_send![alert, addButtonWithTitle: clear];

        #[allow(non_upper_case_globals)]
        const NSModalResponseCancel: NSInteger = 1000;

        let result: NSInteger = msg_send![alert, runModal];
        result != NSModalResponseCancel
    }
}
