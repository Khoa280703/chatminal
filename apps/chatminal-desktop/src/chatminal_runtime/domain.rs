use std::sync::Arc;

use anyhow::{Context, bail};
use config::keyassignment::SpawnTabDomain;
use engine_term::TerminalSize;
use mux::domain::{Domain, DomainId, DomainState, alloc_domain_id};
use mux::pane::Pane;
use mux::window::WindowId;
use portable_pty::CommandBuilder;

use super::client::{ChatminalRuntimeClient, resolve_target_session_id};
use super::session_pane::ChatminalSessionPane;
use super::{CHATMINAL_RUNTIME_DOMAIN_NAME, EmbeddedRuntime, parse_proxy_session_id};

pub struct ChatminalRuntimeDomain {
    runtime: Arc<EmbeddedRuntime>,
    domain_id: DomainId,
}

impl ChatminalRuntimeDomain {
    pub fn new(runtime: Arc<EmbeddedRuntime>) -> Self {
        Self {
            runtime,
            domain_id: alloc_domain_id(),
        }
    }

    fn resolve_session_id(&self, command: Option<CommandBuilder>) -> anyhow::Result<String> {
        let explicit = command.as_ref().and_then(parse_proxy_session_id);
        let client =
            ChatminalRuntimeClient::new(Arc::clone(&self.runtime)).map_err(anyhow::Error::msg)?;
        resolve_target_session_id(&client, explicit.as_deref()).map_err(anyhow::Error::msg)
    }
}

#[async_trait::async_trait(?Send)]
impl Domain for ChatminalRuntimeDomain {
    async fn spawn_pane(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        _command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn Pane>> {
        let session_id = self.resolve_session_id(command)?;
        self.runtime
            .state
            .session_activate(&session_id, size.cols.max(20), size.rows.max(5))
            .map_err(anyhow::Error::msg)?;
        let (surface_id, leaf_id) = self
            .runtime
            .state
            .session_runtime_attachment(&session_id)
            .ok_or_else(|| anyhow::anyhow!("missing runtime attachment for session {session_id}"))?;
        ChatminalSessionPane::new(
            self.runtime.state.session_engine_shared(),
            self.domain_id,
            session_id,
            surface_id,
            leaf_id,
            size,
        )
        .map(|pane| pane as Arc<dyn Pane>)
        .context("create chatminal session pane")
    }

    fn spawnable(&self) -> bool {
        true
    }

    fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    fn domain_name(&self) -> &str {
        CHATMINAL_RUNTIME_DOMAIN_NAME
    }

    async fn attach(&self, _window_id: Option<WindowId>) -> anyhow::Result<()> {
        Ok(())
    }

    fn detachable(&self) -> bool {
        false
    }

    fn detach(&self) -> anyhow::Result<()> {
        bail!("detach not implemented for chatminal runtime domain")
    }

    fn state(&self) -> DomainState {
        DomainState::Attached
    }
}

pub fn resolve_spawn_domain() -> SpawnTabDomain {
    SpawnTabDomain::DomainName(CHATMINAL_RUNTIME_DOMAIN_NAME.to_string())
}
