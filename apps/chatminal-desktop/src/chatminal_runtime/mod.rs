mod client;
mod domain;
mod pane;

use std::ffi::{OsStr, OsString};
use std::sync::Arc;
use std::sync::OnceLock;

use chatminal_runtime::DaemonState;
use mux::domain::Domain;
use mux::Mux;
use portable_pty::CommandBuilder;

pub use client::ChatminalRuntimeClient;
pub use domain::resolve_spawn_domain;

pub const CHATMINAL_RUNTIME_DOMAIN_NAME: &str = "chatminal-runtime";
const DESKTOP_PROXY_COMMAND: &str = "proxy-desktop-session";

pub struct EmbeddedRuntime {
    pub state: DaemonState,
}

static EMBEDDED_RUNTIME: OnceLock<Arc<EmbeddedRuntime>> = OnceLock::new();
static CHATMINAL_DOMAIN_ID: OnceLock<usize> = OnceLock::new();

impl EmbeddedRuntime {
    pub fn global() -> Result<&'static Arc<Self>, String> {
        if let Some(runtime) = EMBEDDED_RUNTIME.get() {
            return Ok(runtime);
        }

        let (state, _config) = DaemonState::initialize_default()?;
        let runtime = Arc::new(Self { state });
        let _ = EMBEDDED_RUNTIME.set(runtime);
        EMBEDDED_RUNTIME
            .get()
            .ok_or_else(|| "failed to initialize embedded chatminal runtime".to_string())
    }
}

pub fn ensure_chatminal_domain_for_command(
    cmd: &Option<CommandBuilder>,
) -> anyhow::Result<Option<Arc<dyn Domain>>> {
    let Some(cmd) = cmd.as_ref() else {
        return Ok(None);
    };
    if parse_proxy_session_id(cmd).is_none() {
        return Ok(None);
    }

    let runtime = Arc::clone(EmbeddedRuntime::global().map_err(anyhow::Error::msg)?);
    let mux = Mux::get();
    if let Some(domain_id) = CHATMINAL_DOMAIN_ID.get().copied() {
        if let Some(domain) = mux.get_domain(domain_id) {
            return Ok(Some(domain));
        }
    }

    let domain: Arc<dyn Domain> = Arc::new(domain::ChatminalRuntimeDomain::new(runtime));
    CHATMINAL_DOMAIN_ID.get_or_init(|| domain.domain_id());
    mux.add_domain(&domain);
    Ok(Some(domain))
}

pub fn runtime_client() -> Result<ChatminalRuntimeClient, String> {
    let runtime = EmbeddedRuntime::global().map(Arc::clone)?;
    client::ChatminalRuntimeClient::new(runtime)
}

pub fn parse_proxy_session_id(builder: &CommandBuilder) -> Option<String> {
    let argv = builder.get_argv();
    if argv.len() < 2 {
        return None;
    }
    if argv
        .get(1)
        .and_then(|value| os_str_to_str(value.as_os_str()))
        .is_none_or(|value| value != DESKTOP_PROXY_COMMAND)
    {
        return None;
    }
    argv.get(2)
        .and_then(|value| os_str_to_str(value.as_os_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or(Some(String::new()))
}

pub fn clamp_preview_lines(value: usize) -> usize {
    value.clamp(50, 20_000)
}

pub fn runtime_proxy_command(session_id: Option<&str>) -> CommandBuilder {
    let mut argv = vec![
        OsString::from("chatminal-runtime"),
        OsString::from(DESKTOP_PROXY_COMMAND),
    ];
    if let Some(session_id) = session_id.map(str::trim).filter(|value| !value.is_empty()) {
        argv.push(OsString::from(session_id));
    }
    CommandBuilder::from_argv(argv)
}

fn os_str_to_str(value: &OsStr) -> Option<&str> {
    value.to_str()
}
