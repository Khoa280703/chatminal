// Don't create a new standard console window when launched from the windows GUI.
#![cfg_attr(not(test), windows_subsystem = "windows")]
#![allow(unexpected_cfgs)] // objc macro expansion still emits cargo-clippy cfg checks upstream

use crate::customglyph::BlockKey;
use crate::glyphcache::GlyphCache;
use crate::utilsprites::RenderMetrics;
use ::window::*;
use anyhow::{anyhow, Context};
use clap::builder::ValueParser;
use clap::{Parser, ValueHint};
use config::keyassignment::SpawnCommand;
use config::{ConfigHandle, SerialTarget};
use gui_subcommands::*;
use portable_pty::cmdbuilder::CommandBuilder;
use std::borrow::Cow;
use std::env::current_dir;
use std::ffi::OsString;
use std::rc::Rc;
use std::sync::Arc;
use terminal_bidi::Direction;
use terminal_font::shaper::PresentationWidth;
use terminal_font::FontConfiguration;
use termwiz::cell::CellAttributes;
use termwiz::surface::{Line, SEQ_ZERO};
use toast_notification::*;
use unicode_normalization::UnicodeNormalization;

use crate::desktop_session_host::{
    activate_host_runtime_entry, host_has_panes_in_workspace, install_serial_spawn_target,
    session_id_from_pane_metadata, spawn_in_primary_target,
};

mod chatminal_layout;
mod chatminal_render;
mod chatminal_sidebar;
mod colorease;
mod commands;
mod customglyph;
mod desktop_commands;
mod desktop_mouse_actions;
mod desktop_session_host;
mod desktop_spawn;
mod desktop_termwindow_types;
mod download;
mod frontend;
mod glyphcache;
mod inputmap;
mod lucide_icons;
mod overlay;
mod quad;
mod renderstate;
mod resize_increment_calculator;
mod runtime_module;
mod scripting;
mod scrollbar;
mod selection;
mod shapecache;
mod shell_bounds;
mod spawn;
mod stats;
mod system_metrics;
mod tabbar;
mod termwindow;
mod uniforms;
mod utilsprites;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

pub use selection::SelectionMode;
pub use termwindow::{set_window_class, set_window_position, TermWindow, ICON_DATA};

#[derive(Debug, Parser)]
#[command(
    about = "Chatminal Desktop",
    version = config::terminal_version()
)]
struct Opt {
    /// Skip loading the configuration file
    #[arg(long, short = 'n')]
    skip_config: bool,

    /// Specify the configuration file to use, overrides the normal
    /// configuration file resolution
    #[arg(
        long = "config-file",
        value_parser,
        conflicts_with = "skip_config",
        value_hint=ValueHint::FilePath,
    )]
    config_file: Option<OsString>,

    /// Override specific configuration values
    #[arg(
        long = "config",
        name = "name=value",
        value_parser=ValueParser::new(name_equals_value),
        number_of_values = 1)]
    config_override: Vec<(String, String)>,

    /// On Windows, whether to attempt to attach to the parent
    /// process console to display logging output
    #[arg(long = "attach-parent-console")]
    #[allow(dead_code)]
    attach_parent_console: bool,

    #[command(subcommand)]
    cmd: Option<SubCommand>,
}

#[derive(Debug, Parser, Clone)]
enum SubCommand {
    #[command(
        name = "start",
        about = "Start the GUI, optionally running an alternative program [aliases: -e]"
    )]
    Start(StartCommand),

    /// Start the GUI in blocking mode. You shouldn't see this, but you
    /// may see it in shell completions because of this open clap issue:
    /// <https://github.com/clap-rs/clap/issues/1335>
    #[command(short_flag_alias = 'e', hide = true)]
    BlockingStart(StartCommand),

    #[command(name = "serial", about = "Open a serial port")]
    Serial(SerialCommand),

    #[command(name = "ls-fonts", about = "Display information about fonts")]
    LsFonts(LsFontsCommand),

    #[command(name = "show-keys", about = "Show key assignments")]
    ShowKeys(ShowKeysCommand),
}

async fn async_run_serial(config: ConfigHandle, opts: SerialCommand) -> anyhow::Result<()> {
    let serial_target = SerialTarget {
        name: format!("Serial Port {}", opts.port),
        port: Some(opts.port.clone()),
        baud: opts.baud,
    };

    let start_command = StartCommand {
        always_new_process: true,
        class: opts.class,
        cwd: None,
        no_auto_connect: true,
        position: opts.position,
        workspace: None,
        ..Default::default()
    };

    let cmd = None;

    install_serial_spawn_target(serial_target)?;

    let should_publish = false;
    async_run_terminal_gui(config, cmd, start_command, should_publish, true).await
}

fn run_serial(config: config::ConfigHandle, opts: SerialCommand) -> anyhow::Result<()> {
    if let Some(cls) = opts.class.as_ref() {
        crate::set_window_class(cls);
    }
    if let Some(pos) = opts.position.as_ref() {
        set_window_position(pos.clone());
    }

    crate::desktop_session_host::build_initial_host_runtime(&config, None)?;

    let gui = crate::frontend::try_new(config.clone())?;

    promise::spawn::spawn(async move {
        if let Err(err) = async_run_serial(config.clone(), opts).await {
            terminate_with_error(err);
        }
    })
    .detach();

    maybe_show_configuration_error_window();
    gui.run_forever()
}

fn have_panes_in_workspace(workspace: &Option<String>) -> bool {
    host_has_panes_in_workspace(workspace.as_deref())
}

async fn spawn_tab_in_spawn_target_if_mux_is_empty(
    config: ConfigHandle,
    cmd: Option<CommandBuilder>,
    workspace: Option<String>,
) -> anyhow::Result<()> {
    if have_panes_in_workspace(&workspace) {
        return Ok(());
    }

    config.update_ulimit()?;

    if have_panes_in_workspace(&workspace) {
        trigger_and_log_gui_attached().await;
        return Ok(());
    }

    let dpi = config.dpi.unwrap_or_else(|| ::window::default_dpi());
    let _tab = spawn_in_primary_target(
        config.initial_size(dpi as u32, Some(cell_pixel_dims(&config, dpi)?)),
        cmd,
        None,
    )
    .await?;
    trigger_and_log_gui_attached().await;
    Ok(())
}

async fn trigger_gui_startup(
    lua: Option<Rc<mlua::Lua>>,
    spawn: Option<SpawnCommand>,
) -> anyhow::Result<()> {
    if let Some(lua) = lua {
        let args = lua.pack_multi(spawn)?;
        config::lua::emit_event(&lua, ("gui-startup".to_string(), args)).await?;
    }
    Ok(())
}

async fn trigger_and_log_gui_startup(spawn_command: Option<SpawnCommand>) {
    if let Err(err) =
        config::with_lua_config_on_main_thread(move |lua| trigger_gui_startup(lua, spawn_command))
            .await
    {
        let message = format!("while processing gui-startup event: {:#}", err);
        log::error!("{}", message);
        persistent_toast_notification("Error", &message);
    }
}

async fn trigger_gui_attached(lua: Option<Rc<mlua::Lua>>) -> anyhow::Result<()> {
    if let Some(lua) = lua {
        let args = lua.pack_multi(())?;
        config::lua::emit_event(&lua, ("gui-attached".to_string(), args)).await?;
    }
    Ok(())
}

async fn trigger_and_log_gui_attached() {
    if let Err(err) = config::with_lua_config_on_main_thread(trigger_gui_attached).await {
        let message = format!("while processing gui-attached event: {:#}", err);
        log::error!("{}", message);
        persistent_toast_notification("Error", &message);
    }
}

fn request_frontend_workspace_reconcile() {
    promise::spawn::spawn_into_main_thread(async move {
        if let Some(front_end) = crate::frontend::try_front_end() {
            front_end.reconcile_workspace();
        }
    })
    .detach();
}

fn cell_pixel_dims(config: &ConfigHandle, dpi: f64) -> anyhow::Result<(usize, usize)> {
    let fontconfig = Rc::new(FontConfiguration::new(Some(config.clone()), dpi as usize)?);
    let render_metrics = RenderMetrics::new(&fontconfig)?;
    Ok((
        render_metrics.cell_size.width as usize,
        render_metrics.cell_size.height as usize,
    ))
}

async fn async_run_terminal_gui(
    config: ConfigHandle,
    cmd: Option<CommandBuilder>,
    opts: StartCommand,
    should_publish: bool,
    force_startup_spawn: bool,
) -> anyhow::Result<()> {
    let unix_socket_path =
        config::RUNTIME_DIR.join(format!("gui-sock-{}", unsafe { libc::getpid() }));
    std::env::set_var("CHATMINAL_UNIX_SOCKET", unix_socket_path.clone());
    terminal_blob_leases::register_storage(Arc::new(
        terminal_blob_leases::simple_tempdir::SimpleTempDir::new_in(&*config::CACHE_DIR)?,
    ))?;
    let _ = should_publish;

    let spawn_command = match &cmd {
        Some(cmd) => Some(SpawnCommand::from_command_builder(cmd)?),
        None => None,
    };

    let should_spawn_on_startup = force_startup_spawn || cmd.is_some();

    trigger_and_log_gui_startup(spawn_command).await;

    if should_spawn_on_startup {
        let dpi = config.dpi.unwrap_or_else(|| ::window::default_dpi());
        let initial_size = config.initial_size(dpi as u32, Some(cell_pixel_dims(&config, dpi)?));
        let spawned = spawn_in_primary_target(initial_size, cmd.clone(), None).await?;
        if let Some(session_id) = session_id_from_pane_metadata(spawned.pane.as_ref()) {
            let _ =
                crate::runtime_module::desktop_activate_session(&session_id, None, initial_size);
        } else {
            activate_host_runtime_entry(spawned.runtime_id.as_u64())?;
        }
        request_frontend_workspace_reconcile();
        trigger_and_log_gui_attached().await;
    }
    let result = spawn_tab_in_spawn_target_if_mux_is_empty(config, cmd, opts.workspace).await;
    request_frontend_workspace_reconcile();
    result
}

fn run_terminal_gui(opts: StartCommand) -> anyhow::Result<()> {
    if let Some(cls) = opts.class.as_ref() {
        crate::set_window_class(cls);
    }
    if let Some(pos) = opts.position.as_ref() {
        set_window_position(pos.clone());
    }

    let config = config::current_config_handle();
    let need_builder = !opts.prog.is_empty() || opts.cwd.is_some();

    let cmd = if need_builder {
        let prog = opts.prog.iter().map(|s| s.as_os_str()).collect::<Vec<_>>();
        let mut builder = config.build_prog(
            if prog.is_empty() { None } else { Some(prog) },
            config.default_prog.as_ref(),
            config.default_cwd.as_ref(),
        )?;
        if let Some(cwd) = &opts.cwd {
            builder.cwd(if cwd.is_relative() {
                current_dir()?.join(cwd).into_os_string().into()
            } else {
                Cow::Borrowed(cwd.as_ref())
            });
        }
        Some(builder)
    } else {
        None
    };

    crate::desktop_session_host::build_initial_host_runtime(&config, opts.workspace.as_deref())?;
    let gui = crate::frontend::try_new(config.clone())?;
    let activity = crate::desktop_session_host::start_host_activity();

    promise::spawn::spawn(async move {
        if let Err(err) = async_run_terminal_gui(config.clone(), cmd, opts, false, false).await {
            terminate_with_error(err);
        }
        drop(activity);
    })
    .detach();

    maybe_show_configuration_error_window();
    gui.run_forever()
}

fn desktop_shell_prog() -> Vec<OsString> {
    vec![
        OsString::from(crate::desktop_session_host::CHATMINAL_RUNTIME_SPAWN_TARGET_NAME),
        OsString::from(crate::desktop_session_host::DESKTOP_PROXY_COMMAND),
    ]
}

fn normalize_desktop_start_command(mut start: StartCommand) -> StartCommand {
    if start.prog.is_empty() {
        start.prog = desktop_shell_prog();
    }
    start
}

fn fatal_toast_notification(title: &str, message: &str) {
    persistent_toast_notification(title, message);
    // We need a short delay otherwise the notification
    // will not show
    #[cfg(windows)]
    std::thread::sleep(std::time::Duration::new(2, 0));
}

fn notify_on_panic() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            fatal_toast_notification("Chatminal panic", s);
        }
        default_hook(info);
    }));
}

fn terminate_with_error_message(err: &str) -> ! {
    log::error!("{}; terminating", err);
    fatal_toast_notification("Chatminal Error", &err);
    std::process::exit(1);
}

fn terminate_with_error(err: anyhow::Error) -> ! {
    let mut err_text = format!("{err:#}");

    let warnings = config::configuration_warnings_and_errors();
    if !warnings.is_empty() {
        let err = warnings.join("\n");
        err_text = format!("{err_text}\nConfiguration Error: {err}");
    }

    terminate_with_error_message(&err_text)
}

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    config::designate_this_as_the_main_thread();
    config::assign_error_callback(
        crate::desktop_session_host::show_host_configuration_error_message,
    );
    notify_on_panic();
    if let Err(e) = run() {
        terminate_with_error(e);
    }
    crate::desktop_session_host::shutdown_host_runtime();
    frontend::shutdown();
}

fn maybe_show_configuration_error_window() {
    let warnings = config::configuration_warnings_and_errors();
    if !warnings.is_empty() {
        let err = warnings.join("\n");
        crate::desktop_session_host::show_host_configuration_error_message(&err);
    }
}

fn run_show_keys(config: config::ConfigHandle, cmd: &ShowKeysCommand) -> anyhow::Result<()> {
    let map = crate::inputmap::InputMap::new(&config);
    if cmd.lua {
        map.dump_config(cmd.key_table.as_deref());
    } else {
        map.show_keys();
    }
    Ok(())
}

pub fn run_ls_fonts(config: config::ConfigHandle, cmd: &LsFontsCommand) -> anyhow::Result<()> {
    use terminal_font::parser::ParsedFont;

    if let Err(err) = config::configuration_result() {
        log::error!("{}", err);
        return Ok(());
    }

    // Disable the normal config error UI window, as we don't have
    // a fully baked GUI environment running
    config::assign_error_callback(|err| eprintln!("{}", err));

    let font_config = Rc::new(terminal_font::FontConfiguration::new(
        Some(config.clone()),
        config.dpi.unwrap_or_else(|| ::window::default_dpi()) as usize,
    )?);

    let render_metrics = crate::utilsprites::RenderMetrics::new(&font_config)?;

    let bidi_hint = if config.bidi_enabled {
        Some(config.bidi_direction)
    } else {
        None
    };

    let unicode_version = config.unicode_version();

    let text = match (&cmd.text, &cmd.codepoints) {
        (Some(text), _) => Some(text.to_string()),
        (_, Some(codepoints)) => {
            let mut s = String::new();
            for cp in codepoints.split(",") {
                let cp = u32::from_str_radix(cp, 16)
                    .with_context(|| format!("{cp} is not a hex number"))?;
                let c = char::from_u32(cp)
                    .ok_or_else(|| anyhow!("{cp} is not a valid unicode codepoint value"))?;
                s.push(c);
            }
            Some(s)
        }
        _ => None,
    };

    if let Some(text) = &text {
        // Emulate the effect of output normalization
        let text = if config.normalize_output_to_unicode_nfc {
            text.nfc().collect()
        } else {
            text.to_string()
        };

        let line = Line::from_text(
            &text,
            &CellAttributes::default(),
            SEQ_ZERO,
            Some(&unicode_version),
        );
        let cell_clusters = line.cluster(bidi_hint);
        let ft_lib = terminal_font::ftwrap::Library::new(Some(&config))?;

        let mut glyph_cache = GlyphCache::new_in_memory(&font_config, 256)?;

        for cluster in cell_clusters {
            let style = font_config.match_style(&config, &cluster.attrs);
            let font = font_config.resolve_font(style)?;
            let presentation_width = PresentationWidth::with_cluster(&cluster);
            let infos = font
                .blocking_shape(
                    &cluster.text,
                    Some(cluster.presentation),
                    cluster.direction,
                    None,
                    Some(&presentation_width),
                )
                .unwrap();

            // We must grab the handles after shaping, so that we get the
            // revised list that includes system fallbacks!
            let handles = font.clone_handles();
            let faces: Vec<_> = handles
                .iter()
                .map(|p| ft_lib.face_from_locator(&p.handle).ok())
                .collect();

            let mut iter = infos.iter().peekable();

            let mut byte_lens = vec![];
            for c in cluster.text.chars() {
                let len = c.len_utf8();
                for _ in 0..len {
                    byte_lens.push(len);
                }
            }
            println!("{:?}", cluster.direction);

            while let Some(info) = iter.next() {
                let idx = cluster.byte_to_cell_idx(info.cluster as usize);
                let followed_by_space = match line.get_cell(idx + 1) {
                    Some(cell) => cell.str() == " ",
                    None => false,
                };

                let text = if cluster.direction == Direction::LeftToRight {
                    if let Some(next) = iter.peek() {
                        line.columns_as_str(idx..cluster.byte_to_cell_idx(next.cluster as usize))
                    } else {
                        let last_idx = cluster.byte_to_cell_idx(cluster.text.len() - 1);
                        line.columns_as_str(idx..last_idx + 1)
                    }
                } else {
                    let info_len = byte_lens[info.cluster as usize];
                    let last_idx = cluster.byte_to_cell_idx(info.cluster as usize + info_len - 1);
                    line.columns_as_str(idx..last_idx + 1)
                };

                let parsed = &handles[info.font_idx];
                let escaped = format!("{}", text.escape_unicode());
                let mut is_custom = false;

                let cached_glyph = glyph_cache.cached_glyph(
                    &info,
                    &style,
                    followed_by_space,
                    &font,
                    &render_metrics,
                    info.num_cells,
                )?;

                let mut texture = cached_glyph.texture.clone();

                if config.custom_block_glyphs {
                    if let Some(block) = info.only_char.and_then(BlockKey::from_char) {
                        texture.replace(glyph_cache.cached_block(block, &render_metrics)?);
                        println!(
                            "{:2} {:4} {:12} drawn by Chatminal because custom_block_glyphs=true: {:?}",
                            info.cluster, text, escaped, block
                        );
                        is_custom = true;
                    }
                }

                if !is_custom {
                    let glyph_name = faces[info.font_idx]
                        .as_ref()
                        .and_then(|face| {
                            face.get_glyph_name(info.glyph_pos)
                                .map(|name| format!("{},", name))
                        })
                        .unwrap_or_else(String::new);

                    println!(
                        "{:2} {:4} {:12} x_adv={:<2} cells={:<2} glyph={}{:<4} {}\n{:38}{}",
                        info.cluster,
                        text,
                        escaped,
                        cached_glyph.x_advance.get(),
                        info.num_cells,
                        glyph_name,
                        info.glyph_pos,
                        parsed.lua_name(),
                        "",
                        parsed.handle.diagnostic_string()
                    );
                }

                if cmd.rasterize_ascii {
                    let mut glyph = String::new();

                    if let Some(texture) = &cached_glyph.texture {
                        use ::window::bitmaps::ImageTexture;
                        if let Some(tex) = texture.texture.downcast_ref::<ImageTexture>() {
                            for y in texture.coords.min_y()..texture.coords.max_y() {
                                for &px in tex.image.borrow().horizontal_pixel_range(
                                    texture.coords.min_x() as usize,
                                    texture.coords.max_x() as usize,
                                    y as usize,
                                ) {
                                    let px = u32::from_be(px);
                                    let (b, g, r, a) = (
                                        (px >> 8) as u8,
                                        (px >> 16) as u8,
                                        (px >> 24) as u8,
                                        (px & 0xff) as u8,
                                    );
                                    // Use regular RGB for other terminals, but then
                                    // set RGBA for Chatminal's renderer
                                    glyph.push_str(&format!(
                                "\x1b[38:2::{r}:{g}:{b}m\x1b[38:6::{r}:{g}:{b}:{a}m\u{2588}\x1b[0m"
                            ));
                                }
                                glyph.push('\n');
                            }
                        }
                    }

                    if !is_custom {
                        println!(
                            "bearing: x={} y={}, offset: x={} y={}",
                            cached_glyph.bearing_x.get(),
                            cached_glyph.bearing_y.get(),
                            cached_glyph.x_offset.get(),
                            cached_glyph.y_offset.get(),
                        );
                    }
                    println!("{glyph}");
                }
            }
        }
        return Ok(());
    }

    println!("Primary font:");
    let default_font = font_config.default_font()?;
    println!(
        "{}",
        ParsedFont::lua_fallback(&default_font.clone_handles())
    );
    println!();

    for rule in &config.font_rules {
        println!();

        let mut condition = "When".to_string();
        if let Some(intensity) = &rule.intensity {
            condition.push_str(&format!(" Intensity={:?}", intensity));
        }
        if let Some(underline) = &rule.underline {
            condition.push_str(&format!(" Underline={:?}", underline));
        }
        if let Some(italic) = &rule.italic {
            condition.push_str(&format!(" Italic={:?}", italic));
        }
        if let Some(blink) = &rule.blink {
            condition.push_str(&format!(" Blink={:?}", blink));
        }
        if let Some(rev) = &rule.reverse {
            condition.push_str(&format!(" Reverse={:?}", rev));
        }
        if let Some(strikethrough) = &rule.strikethrough {
            condition.push_str(&format!(" Strikethrough={:?}", strikethrough));
        }
        if let Some(invisible) = &rule.invisible {
            condition.push_str(&format!(" Invisible={:?}", invisible));
        }

        println!("{}:", condition);
        let font = font_config.resolve_font(&rule.font)?;
        println!("{}", ParsedFont::lua_fallback(&font.clone_handles()));
        println!();
    }

    println!("Title font:");
    let title_font = font_config.title_font()?;
    println!("{}", ParsedFont::lua_fallback(&title_font.clone_handles()));
    println!();

    if cmd.list_system {
        let font_dirs = font_config.list_fonts_in_font_dirs();
        println!(
            "{} fonts found in your font_dirs + built-in fonts:",
            font_dirs.len()
        );
        for font in font_dirs {
            let pixel_sizes = if font.pixel_sizes.is_empty() {
                "".to_string()
            } else {
                format!(" pixel_sizes={:?}", font.pixel_sizes)
            };
            println!(
                "{} -- {}{}{}",
                font.lua_name(),
                font.aka(),
                font.handle.diagnostic_string(),
                pixel_sizes
            );
        }

        match font_config.list_system_fonts() {
            Ok(sys_fonts) => {
                println!(
                    "{} system fonts found using {:?}:",
                    sys_fonts.len(),
                    config.font_locator
                );
                for font in sys_fonts {
                    let pixel_sizes = if font.pixel_sizes.is_empty() {
                        "".to_string()
                    } else {
                        format!(" pixel_sizes={:?}", font.pixel_sizes)
                    };
                    println!(
                        "{} -- {}{}{}",
                        font.lua_name(),
                        font.aka(),
                        font.handle.diagnostic_string(),
                        pixel_sizes
                    );
                }
            }
            Err(err) => log::error!("Unable to list system fonts: {}", err),
        }
    }

    Ok(())
}

fn run() -> anyhow::Result<()> {
    // Inform the system of our AppUserModelID.
    // Without this, our toast notifications won't be correctly
    // attributed to our application.
    #[cfg(windows)]
    {
        unsafe {
            ::windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(
                ::windows::core::PCWSTR(wide_string("org.chatminal.desktop").as_ptr()),
            )
            .unwrap();
        }
    }

    let opts = Opt::parse();

    // This is a bit gross.
    // In order to not to automatically open a standard windows console when
    // we run, we use the windows_subsystem attribute at the top of this
    // source file.  That comes at the cost of causing the help output
    // to disappear if we are actually invoked from a console.
    // This AttachConsole call will attach us to the console of the parent
    // in that situation, but since we were launched as a windows subsystem
    // application we will be running asynchronously from the shell in
    // the command window, which means that it will appear to the user
    // that we hung at the end, when in reality the shell is waiting for
    // input but didn't know to re-draw the prompt.
    #[cfg(windows)]
    unsafe {
        if opts.attach_parent_console {
            winapi::um::wincon::AttachConsole(winapi::um::wincon::ATTACH_PARENT_PROCESS);
        }
    };

    env_bootstrap::bootstrap();
    crate::desktop_session_host::install_lua_bridge_backend_once();
    // window_funcs is not set up by env_bootstrap as window_funcs is
    // GUI environment specific and env_bootstrap is used to setup the
    // headless mux server.
    config::lua::add_context_setup_func(window_funcs::register);
    config::lua::add_context_setup_func(crate::scripting::register);
    config::lua::add_context_setup_func(crate::stats::register);

    let initial_config = config::current_config_handle();
    stats::Stats::init(&initial_config)?;
    let _saver = umask::UmaskSaver::new();

    let mut config_overrides = opts.config_override.clone();
    if crate::chatminal_sidebar::sidebar_enabled() {
        if !config_overrides
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("show_session_index_in_session_bar"))
        {
            config_overrides.push((
                "show_session_index_in_session_bar".to_string(),
                "false".to_string(),
            ));
        }
    }

    config::common_init(
        opts.config_file.as_ref(),
        &config_overrides,
        opts.skip_config,
    )?;
    let config = config::current_config_handle();

    let sub = match opts.cmd.as_ref().cloned() {
        Some(SubCommand::BlockingStart(start)) => {
            // Act as if the normal start subcommand was used,
            // except that we always start a new instance.
            // This is needed for compatibility, because many tools assume
            // that "$TERMINAL -e $COMMAND" blocks until the command finished.
            SubCommand::Start(StartCommand {
                always_new_process: true,
                ..start
            })
        }
        Some(sub) => sub,
        None => {
            // Need to fake an argv0
            let mut argv = vec!["chatminal-desktop".to_string()];
            for a in &config.default_gui_startup_args {
                argv.push(a.clone());
            }
            let parsed = SubCommand::try_parse_from(&argv).with_context(|| {
                format!(
                    "parsing the default_gui_startup_args config: {:?}",
                    config.default_gui_startup_args
                )
            })?;
            match parsed {
                SubCommand::Start(start) => {
                    SubCommand::Start(normalize_desktop_start_command(start))
                }
                other => other,
            }
        }
    };

    match sub {
        SubCommand::Start(start) => {
            log::trace!("Using configuration: {:#?}\nopts: {:#?}", config, opts);
            let res = run_terminal_gui(normalize_desktop_start_command(start));
            terminal_blob_leases::clear_storage();
            res
        }
        SubCommand::BlockingStart(_) => unreachable!(),
        SubCommand::Serial(serial) => run_serial(config, serial),
        SubCommand::LsFonts(cmd) => run_ls_fonts(config, &cmd),
        SubCommand::ShowKeys(cmd) => run_show_keys(config, &cmd),
    }
}
