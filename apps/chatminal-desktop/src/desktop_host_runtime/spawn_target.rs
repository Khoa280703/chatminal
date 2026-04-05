use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static LOCAL_PANE_RUNTIME_ID_COUNTER: AtomicU64 = AtomicU64::new(0x_ffff_0000_0000_0001);

use anyhow::Context;
use config::SerialTarget;
use engine_term::TerminalSize;
use chatminal_runtime::local_spawn_target::{LocalSpawnHooks, LocalSpawnTarget};
use portable_pty::CommandBuilder;

use super::session_pane::ChatminalSessionPane;
use super::{
    CHATMINAL_RUNTIME_SPAWN_TARGET_NAME, DesktopSpawnTargetBackend, EmbeddedRuntime,
    HostSpawnedRuntimeEntry, HostTerminal, parse_proxy_session_id,
};
use crate::chatminal_runtime::{
    ChatminalRuntimeClient, activate_runtime_session, resolve_target_session_id,
    runtime_session_attachment,
};
use crate::frontend::current_frontend_config;

pub(crate) struct DesktopSpawnTarget {
    local: LocalSpawnTarget,
}

impl DesktopSpawnTarget {
    pub(crate) fn new_local() -> anyhow::Result<Self> {
        Ok(Self {
            local: LocalSpawnTarget::new_with_hooks("local", LocalSpawnHooks::noop())?,
        })
    }

    pub(crate) fn new_serial(serial_target: SerialTarget) -> anyhow::Result<Self> {
        Ok(Self {
            local: LocalSpawnTarget::new_serial_target_with_hooks(
                serial_target,
                LocalSpawnHooks::noop(),
            )?,
        })
    }

    fn runtime_session_id(
        &self,
        command: Option<CommandBuilder>,
    ) -> anyhow::Result<Option<String>> {
        let Some(explicit) = command.as_ref().and_then(parse_proxy_session_id) else {
            return Ok(None);
        };
        let runtime = EmbeddedRuntime::global().map_err(anyhow::Error::msg)?;
        let client =
            ChatminalRuntimeClient::new(Arc::clone(runtime)).map_err(anyhow::Error::msg)?;
        let session_id = resolve_target_session_id(&client, Some(explicit.as_str()))
            .map_err(anyhow::Error::msg)?;
        Ok(Some(session_id))
    }
}

#[async_trait::async_trait(?Send)]
impl DesktopSpawnTargetBackend for DesktopSpawnTarget {
    async fn spawn(
        &self,
        size: TerminalSize,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
    ) -> anyhow::Result<HostSpawnedRuntimeEntry> {
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
                current_frontend_config(),
            )
            .map(|pane| pane as Arc<dyn HostTerminal>)
            .map(|pane| HostSpawnedRuntimeEntry { runtime_id, pane })
            .context("create chatminal session pane");
        }

        let pane: Arc<dyn HostTerminal> = self.local.spawn_pane(size, command, command_dir).await?;
        Ok(HostSpawnedRuntimeEntry {
            runtime_id: chatminal_runtime::RuntimeId::new(
                LOCAL_PANE_RUNTIME_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            ),
            pane,
        })
    }

    fn spawn_target_name(&self) -> &str {
        CHATMINAL_RUNTIME_SPAWN_TARGET_NAME
    }
}
