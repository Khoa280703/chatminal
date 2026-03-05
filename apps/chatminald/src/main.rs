mod config;
mod metrics;
mod server;
mod session;
mod state;
mod transport;

use chatminal_store::Store;
use config::DaemonConfig;
use state::DaemonState;

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
    server::run_server(&config.endpoint, state)
}
