use chatminal_runtime::server::run_server;
use chatminal_runtime::{DaemonConfig, DaemonState};
use chatminal_store::Store;

fn main() {
    let _ = env_logger::try_init();

    if let Err(err) = run() {
        eprintln!("chatminald error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = DaemonConfig::from_env()?;
    let store = Store::initialize_default()?;

    log::info!("chatminald database: {}", store.db_path().display());
    let state = DaemonState::new(config.clone(), store)?;
    run_server(&config.endpoint, state)
}
