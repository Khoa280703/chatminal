mod app;
mod config;
mod message;
mod session;
mod ui;

use app::AppState;
use iced::window;

fn main() -> iced::Result {
    // Ignore SIGPIPE so writes to a dead PTY do not terminate the app process.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }

    env_logger::init();

    iced::application(AppState::boot, AppState::update, AppState::view)
        .title(app_title)
        .subscription(AppState::subscription)
        .window(window::Settings {
            min_size: Some(iced::Size::new(800.0, 600.0)),
            ..Default::default()
        })
        .run()
}

fn app_title(_: &AppState) -> String {
    String::from("Chatminal")
}
