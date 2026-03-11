use crate::ICON_DATA;
use anyhow::anyhow;
use config::{configuration, engine_version};
use engine_toast_notification::*;
use http_req::request::{HttpVersion, Request};
use http_req::uri::Uri;
use mux::connui::ConnectionUI;
use serde::*;
use std::convert::TryFrom;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use termwiz::cell::{Hyperlink, Underline};
use termwiz::color::AnsiColor;
use termwiz::escape::csi::{Cursor, Sgr};
use termwiz::escape::osc::{ITermDimension, ITermFileData, ITermProprietary};
use termwiz::escape::{CSI, OneBased, OperatingSystemCommand};

const CHATMINAL_ENABLE_DESKTOP_UPDATE_CHECK: &str = "CHATMINAL_ENABLE_DESKTOP_UPDATE_CHECK";
const CHATMINAL_ALWAYS_SHOW_UPDATE_UI: &str = "CHATMINAL_ALWAYS_SHOW_UPDATE_UI";
const CHATMINAL_DESKTOP_UPDATE_LATEST_URL: &str = "CHATMINAL_DESKTOP_UPDATE_LATEST_URL";
const CHATMINAL_DESKTOP_UPDATE_NIGHTLY_URL: &str = "CHATMINAL_DESKTOP_UPDATE_NIGHTLY_URL";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Release {
    pub url: String,
    pub body: String,
    pub html_url: String,
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Asset {
    pub name: String,
    pub size: usize,
    pub url: String,
    pub browser_download_url: String,
}

fn get_release_info(uri: &str) -> anyhow::Result<Release> {
    let uri = Uri::try_from(uri)?;

    let mut latest = Vec::new();
    let _res = Request::new(&uri)
        .version(HttpVersion::Http10)
        .header(
            "User-Agent",
            &format!("chatminal-desktop/{}", engine_version()),
        )
        .send(&mut latest)
        .map_err(|e| anyhow!("failed to query desktop release feed: {}", e))?;

    let latest: Release = serde_json::from_slice(&latest)?;
    Ok(latest)
}

fn configured_release_url(env_name: &str) -> anyhow::Result<String> {
    std::env::var(env_name)
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("desktop update feed is not configured: {env_name}"))
}

pub fn get_latest_release_info() -> anyhow::Result<Release> {
    get_release_info(&configured_release_url(
        CHATMINAL_DESKTOP_UPDATE_LATEST_URL,
    )?)
}

#[allow(unused)]
pub fn get_nightly_release_info() -> anyhow::Result<Release> {
    get_release_info(&configured_release_url(
        CHATMINAL_DESKTOP_UPDATE_NIGHTLY_URL,
    )?)
}

lazy_static::lazy_static! {
    static ref UPDATER_WINDOW: Mutex<Option<ConnectionUI>> = Mutex::new(None);
}

pub fn load_last_release_info_and_set_banner() {
    if !desktop_update_check_enabled() || !configuration().check_for_updates {
        return;
    }

    let update_file_name = config::DATA_DIR.join("check_update");
    if let Ok(data) = std::fs::read(update_file_name) {
        let latest: Release = match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(_) => return,
        };

        let current = engine_version();
        let force_ui = always_show_update_ui();
        if latest.tag_name.as_str() <= current && !force_ui {
            return;
        }

        set_banner_from_release_info(&latest);
    }
}

fn release_notes_url(latest: &Release) -> String {
    let trimmed = latest.html_url.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    latest.url.clone()
}

fn set_banner_from_release_info(latest: &Release) {
    let mux = crate::Mux::get();
    let url = release_notes_url(latest);

    let icon = ITermFileData {
        name: None,
        size: Some(ICON_DATA.len()),
        width: ITermDimension::Automatic,
        height: ITermDimension::Cells(2),
        preserve_aspect_ratio: true,
        inline: true,
        do_not_move_cursor: false,
        data: ICON_DATA.to_vec(),
    };
    let icon = OperatingSystemCommand::ITermProprietary(ITermProprietary::File(Box::new(icon)));
    let top_line_pos = CSI::Cursor(Cursor::CharacterAndLinePosition {
        line: OneBased::new(1),
        col: OneBased::new(6),
    });
    let second_line_pos = CSI::Cursor(Cursor::CharacterAndLinePosition {
        line: OneBased::new(2),
        col: OneBased::new(6),
    });
    let link_on = OperatingSystemCommand::SetHyperlink(Some(Hyperlink::new(url)));
    let underline_color = CSI::Sgr(Sgr::UnderlineColor(AnsiColor::Blue.into()));
    let underline_on = CSI::Sgr(Sgr::Underline(Underline::Single));
    let reset = CSI::Sgr(Sgr::Reset);
    let link_off = OperatingSystemCommand::SetHyperlink(None);
    mux.set_banner(Some(format!(
        "{}{}Chatminal Update Available\r\n{}{}{}{}Click to see what's new{}{}\r\n",
        icon,
        top_line_pos,
        second_line_pos,
        link_on,
        underline_color,
        underline_on,
        link_off,
        reset,
    )));
}

fn schedule_set_banner_from_release_info(latest: &Release) {
    let current = engine_version();
    if latest.tag_name.as_str() <= current {
        return;
    }
    promise::spawn::spawn_into_main_thread({
        let latest = latest.clone();
        async move {
            set_banner_from_release_info(&latest);
        }
    })
    .detach();
}

fn update_checker() {
    if !desktop_update_check_enabled() {
        return;
    }

    let update_interval = Duration::from_secs(configuration().check_for_updates_interval_seconds);
    let initial_interval = Duration::from_secs(10);

    let force_ui = always_show_update_ui();

    let update_file_name = config::DATA_DIR.join("check_update");
    let delay = update_file_name
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(|_| ())
        .and_then(|systime| {
            let elapsed = systime.elapsed().unwrap_or(Duration::new(0, 0));
            update_interval.checked_sub(elapsed).ok_or(())
        })
        .unwrap_or(initial_interval);

    std::thread::sleep(if force_ui { initial_interval } else { delay });

    let my_sock = config::RUNTIME_DIR.join(format!("gui-sock-{}", unsafe { libc::getpid() }));

    loop {
        let socks = engine_client::discovery::discover_gui_socks();

        if configuration().check_for_updates {
            if let Ok(latest) = get_latest_release_info() {
                schedule_set_banner_from_release_info(&latest);
                let current = engine_version();
                if latest.tag_name.as_str() > current || force_ui {
                    log::info!(
                        "latest release {} is newer than current build {}",
                        latest.tag_name,
                        current
                    );

                    let url = release_notes_url(&latest);

                    if force_ui || socks.is_empty() || socks[0] == my_sock {
                        persistent_toast_notification_with_click_to_open_url(
                            "Chatminal Update Available",
                            "Click to see what's new",
                            &url,
                        );
                    }
                }

                config::create_user_owned_dirs(update_file_name.parent().unwrap()).ok();

                if let Ok(f) = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&update_file_name)
                {
                    serde_json::to_writer_pretty(f, &latest).ok();
                }
            }
        }

        std::thread::sleep(Duration::from_secs(
            configuration().check_for_updates_interval_seconds,
        ));
    }
}

pub fn start_update_checker() {
    if !desktop_update_check_enabled() {
        return;
    }

    static CHECKER_STARTED: AtomicBool = AtomicBool::new(false);
    if let Ok(false) =
        CHECKER_STARTED.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
    {
        std::thread::Builder::new()
            .name("update_checker".into())
            .spawn(update_checker)
            .expect("failed to spawn update checker thread");
    }
}

fn desktop_update_check_enabled() -> bool {
    std::env::var_os(CHATMINAL_ENABLE_DESKTOP_UPDATE_CHECK).is_some_and(|value| value == "1")
}

fn always_show_update_ui() -> bool {
    std::env::var_os(CHATMINAL_ALWAYS_SHOW_UPDATE_UI).is_some_and(|value| value == "1")
}
