use std::sync::Arc;

use anyhow::Context;
use config::SerialTarget;
use engine_term::TerminalSize;
use host_runtime::spawn_target::LocalSpawnTarget;
use portable_pty::CommandBuilder;

use super::session_pane::ChatminalSessionPane;
use super::{
    parse_proxy_session_id, EmbeddedRuntime, HostSpawnTarget as SpawnTarget, HostTerminal,
    CHATMINAL_RUNTIME_SPAWN_TARGET_NAME,
};
use crate::chatminal_runtime::{
    activate_runtime_session, resolve_target_session_id, runtime_session_attachment,
    ChatminalRuntimeClient,
};

pub(crate) struct DesktopSpawnTarget {
    local: LocalSpawnTarget,
}

impl DesktopSpawnTarget {
    pub(crate) fn new_local() -> anyhow::Result<Self> {
        Ok(Self {
            local: LocalSpawnTarget::new("local")?,
        })
    }

    pub(crate) fn new_serial(serial_target: SerialTarget) -> anyhow::Result<Self> {
        Ok(Self {
            local: LocalSpawnTarget::new_serial_target(serial_target)?,
        })
    }

    fn runtime_session_id(&self, command: Option<CommandBuilder>) -> anyhow::Result<Option<String>> {
        let Some(explicit) = command.as_ref().and_then(parse_proxy_session_id) else {
            return Ok(None);
        };
        let runtime = EmbeddedRuntime::global().map_err(anyhow::Error::msg)?;
        let client = ChatminalRuntimeClient::new(Arc::clone(runtime)).map_err(anyhow::Error::msg)?;
        let session_id =
            resolve_target_session_id(&client, Some(explicit.as_str())).map_err(anyhow::Error::msg)?;
        Ok(Some(session_id))
    }
}

#[async_trait::async_trait(?Send)]
impl SpawnTarget for DesktopSpawnTarget {
    async fn spawn_pane(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<Arc<dyn HostTerminal>> {
        if let Some(session_id) = self.runtime_session_id(command.clone())? {
            let runtime = EmbeddedRuntime::global()
                .map(Arc::clone)
                .map_err(anyhow::Error::msg)?;
            activate_runtime_session(&session_id, size.cols.max(20), size.rows.max(5))
                .map_err(anyhow::Error::msg)?;
            let (runtime_id, terminal_instance_id) = runtime_session_attachment(&session_id)
                .map_err(anyhow::Error::msg)?
                .ok_or_else(|| {
                    anyhow::anyhow!("missing runtime attachment for session {session_id}")
                })?;
            return ChatminalSessionPane::new(
                runtime.session_engine_shared(),
                session_id,
                runtime_id,
                terminal_instance_id,
                size,
            )
            .map(|pane| pane as Arc<dyn HostTerminal>)
            .context("create chatminal session pane");
        }

        self.local.spawn_pane(size, command, command_dir).await
    }
    fn spawn_target_name(&self) -> &str {
        CHATMINAL_RUNTIME_SPAWN_TARGET_NAME
    }
}
