use std::sync::Arc;

use anyhow::{bail, Context};
use config::keyassignment::SpawnTabDomain;
use engine_term::TerminalSize;
use mux::domain::{alloc_domain_id, Domain, DomainId, DomainState};
use mux::pane::Pane;
use mux::window::WindowId;
use portable_pty::CommandBuilder;

use super::client::{resolve_target_session_id, ChatminalRuntimeClient};
use super::pane::ChatminalRuntimePane;
use super::{parse_proxy_session_id, EmbeddedRuntime, CHATMINAL_RUNTIME_DOMAIN_NAME};

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
        ChatminalRuntimePane::new(Arc::clone(&self.runtime), self.domain_id, session_id, size)
            .map(|pane| pane as Arc<dyn Pane>)
            .context("create chatminal runtime pane")
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
