use std::sync::Arc;

use anyhow::{bail, Context};
use engine_term::TerminalSize;
use portable_pty::CommandBuilder;

use super::session_pane::ChatminalSessionPane;
use super::{
    alloc_host_domain_id, parse_proxy_session_id, EmbeddedRuntime, HostDomain as Domain,
    HostDomainId as DomainId, HostDomainState as DomainState, HostTerminal,
    RuntimeWindowId as WindowId, CHATMINAL_RUNTIME_DOMAIN_NAME,
};
use crate::chatminal_runtime::{
    activate_runtime_session, resolve_target_session_id, runtime_session_attachment,
    ChatminalRuntimeClient,
};

pub(crate) struct ChatminalRuntimeDomain {
    runtime: Arc<EmbeddedRuntime>,
    domain_id: DomainId,
}

impl ChatminalRuntimeDomain {
    pub(crate) fn new(runtime: Arc<EmbeddedRuntime>) -> Self {
        Self {
            runtime,
            domain_id: alloc_host_domain_id(),
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
    ) -> anyhow::Result<Arc<dyn HostTerminal>> {
        let session_id = self.resolve_session_id(command)?;
        activate_runtime_session(&session_id, size.cols.max(20), size.rows.max(5))
            .map_err(anyhow::Error::msg)?;
        let (runtime_id, terminal_instance_id) = runtime_session_attachment(&session_id)
            .map_err(anyhow::Error::msg)?
            .ok_or_else(|| {
                anyhow::anyhow!("missing runtime attachment for session {session_id}")
            })?;
        ChatminalSessionPane::new(
            self.runtime.session_engine_shared(),
            self.domain_id,
            session_id,
            runtime_id,
            terminal_instance_id,
            size,
        )
        .map(|pane| pane as Arc<dyn HostTerminal>)
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
