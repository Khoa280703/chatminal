use clap::{builder::ValueParser, Parser, ValueHint};
use config::configuration;
use engine_gui_subcommands::name_equals_value;
use engine_mux_server_impl::update_mux_domains_for_server;
use mux::activity::Activity;
use mux::domain::{Domain, LocalDomain};
use mux::Mux;
use portable_pty::cmdbuilder::CommandBuilder;
use std::ffi::OsString;
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

mod daemonize;
mod ossl;

#[derive(Debug, Parser)]
#[command(
    about = "Chatminal headless mux host",
    version = config::engine_version(),
    trailing_var_arg = true,
)]
struct Opt {
    #[arg(long, short = 'n')]
    skip_config: bool,

    #[arg(
        long = "config-file",
        value_parser,
        conflicts_with = "skip_config",
        value_hint = ValueHint::FilePath,
    )]
    config_file: Option<OsString>,

    #[arg(
        long = "config",
        name = "name=value",
        value_parser = ValueParser::new(name_equals_value),
        number_of_values = 1
    )]
    config_override: Vec<(String, String)>,

    #[arg(long = "daemonize")]
    daemonize: bool,

    #[arg(long = "cwd", value_parser, value_hint = ValueHint::DirPath)]
    cwd: Option<OsString>,

    #[cfg(unix)]
    #[arg(long, hide = true)]
    pid_file_fd: Option<i32>,

    #[arg(value_parser, value_hint = ValueHint::CommandWithArguments, num_args = 1..)]
    prog: Vec<OsString>,
}

fn main() {
    if let Err(err) = run() {
        engine_blob_leases::clear_storage();
        log::error!("{:#}", err);
        std::process::exit(1);
    }
    engine_blob_leases::clear_storage();
}

fn run() -> anyhow::Result<()> {
    env_bootstrap::bootstrap();
    config::designate_this_as_the_main_thread();
    let _saver = umask::UmaskSaver::new();
    let opts = Opt::parse();

    #[cfg(unix)]
    if let Some(fd) = opts.pid_file_fd {
        daemonize::set_cloexec(fd, true);
    }

    config::common_init(
        opts.config_file.as_ref(),
        &opts.config_override,
        opts.skip_config,
    )?;

    let config = config::configuration();
    config.update_ulimit()?;
    if let Some(value) = &config.default_ssh_auth_sock {
        std::env::set_var("SSH_AUTH_SOCK", value);
    }

    #[cfg(unix)]
    let mut pid_file = None;

    #[cfg(unix)]
    if opts.daemonize {
        pid_file = daemonize::daemonize(&config)?;
    }

    if opts.daemonize {
        #[cfg(unix)]
        {
            return reexec_daemonized(&opts, pid_file);
        }
        #[cfg(windows)]
        {
            return reexec_daemonized(&opts, &config);
        }
    }

    sanitize_mux_environment();

    engine_blob_leases::register_storage(Arc::new(
        engine_blob_leases::simple_tempdir::SimpleTempDir::new_in(&*config::CACHE_DIR)?,
    ))?;

    let command = build_spawn_command(&opts);
    let domain: Arc<dyn Domain> = Arc::new(LocalDomain::new("local")?);
    let mux = Arc::new(Mux::new(Some(domain)));
    Mux::set_mux(&mux);
    spawn_listeners()?;

    let executor = promise::spawn::SimpleExecutor::new();
    let activity = Activity::new();
    promise::spawn::spawn(async move {
        if let Err(err) = async_run(command).await {
            terminate_with_error(err);
        }
        drop(activity);
    })
    .detach();

    loop {
        executor.tick()?;
    }
}

fn build_spawn_command(opts: &Opt) -> Option<CommandBuilder> {
    let need_builder = !opts.prog.is_empty() || opts.cwd.is_some();
    if !need_builder {
        return None;
    }

    let mut builder = if opts.prog.is_empty() {
        CommandBuilder::new_default_prog()
    } else {
        CommandBuilder::from_argv(opts.prog.clone())
    };
    if let Some(cwd) = &opts.cwd {
        builder.cwd(cwd.clone());
    }
    Some(builder)
}

fn sanitize_mux_environment() {
    for name in [
        "OLDPWD",
        "PWD",
        "SHLVL",
        "CHATMINAL_PANE",
        "CHATMINAL_UNIX_SOCKET",
        "_",
    ] {
        std::env::remove_var(name);
    }
    for name in &configuration().mux_env_remove {
        std::env::remove_var(name);
    }
}

fn reexec_daemonized(
    opts: &Opt,
    #[cfg(unix)] pid_file: Option<i32>,
    #[cfg(windows)] config: &config::ConfigHandle,
) -> anyhow::Result<()> {
    let mut cmd = Command::new(std::env::current_exe()?);

    #[cfg(unix)]
    if let Some(fd) = pid_file {
        cmd.arg("--pid-file-fd").arg(fd.to_string());
    }
    if opts.skip_config {
        cmd.arg("-n");
    }
    if let Some(path) = &opts.config_file {
        cmd.arg("--config-file").arg(path);
    }
    for (name, value) in &opts.config_override {
        cmd.arg("--config").arg(format!("{name}={value}"));
    }
    if let Some(cwd) = &opts.cwd {
        cmd.arg("--cwd").arg(cwd);
    }
    if !opts.prog.is_empty() {
        cmd.arg("--");
        for arg in &opts.prog {
            cmd.arg(arg);
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.stdout(config.daemon_options.open_stdout()?);
        cmd.stderr(config.daemon_options.open_stderr()?);
        cmd.creation_flags(winapi::um::winbase::DETACHED_PROCESS);
        drop(cmd.spawn()?);
        return Ok(());
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        if let Some(mask) = umask::UmaskSaver::saved_umask() {
            unsafe {
                cmd.pre_exec(move || {
                    libc::umask(mask);
                    Ok(())
                });
            }
        }
        Err(anyhow::anyhow!("failed to re-exec: {:?}", cmd.exec()))
    }
}

async fn async_run(command: Option<CommandBuilder>) -> anyhow::Result<()> {
    let mux = Mux::get();
    let config = configuration();

    update_mux_domains_for_server(&config)?;
    let _config_subscription = config::subscribe_to_config_reload(move || {
        promise::spawn::spawn_into_main_thread(async move {
            if let Err(err) = update_mux_domains_for_server(&config::configuration()) {
                log::error!("Error updating mux domains: {:#}", err);
            }
        })
        .detach();
        true
    });

    if let Err(err) = config::with_lua_config_on_main_thread(trigger_mux_startup).await {
        log::error!("while processing mux-startup event: {:#}", err);
    }

    let domain = mux.default_domain();
    let have_panes = mux
        .iter_panes()
        .iter()
        .any(|pane| pane.domain_id() == domain.domain_id());
    if !have_panes {
        let window_id = mux.new_empty_window(None, None);
        domain.attach(Some(*window_id)).await?;
        mux.default_domain()
            .spawn(config.initial_size(0, None), command, None, *window_id)
            .await?;
    }
    Ok(())
}

async fn trigger_mux_startup(lua: Option<Rc<mlua::Lua>>) -> anyhow::Result<()> {
    if let Some(lua) = lua {
        let args = lua.pack_multi(())?;
        config::lua::emit_event(&lua, ("mux-startup".to_string(), args)).await?;
    }
    Ok(())
}

fn spawn_listeners() -> anyhow::Result<()> {
    let config = configuration();
    for unix_dom in &config.unix_domains {
        std::env::set_var("CHATMINAL_UNIX_SOCKET", unix_dom.socket_path());
        let mut listener = engine_mux_server_impl::local::LocalListener::with_domain(unix_dom)?;
        thread::spawn(move || listener.run());
    }
    for tls_server in &config.tls_servers {
        ossl::spawn_tls_listener(tls_server)?;
    }
    Ok(())
}

fn terminate_with_error(err: anyhow::Error) -> ! {
    log::error!("{:#}; terminating", err);
    std::process::exit(1);
}
