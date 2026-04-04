use crate::chatminal_runtime::{
    desktop_create_split_session_view, desktop_current_active_session_id,
    desktop_current_active_terminal_handle, DesktopSplitRequest,
};
use crate::desktop_host_runtime::{
    host_workspace_name, spawn_host_runtime_entry, start_host_activity,
};
use anyhow::{anyhow, Context};
use config::keyassignment::SpawnCommand;
use config::TermConfig;
use engine_term::TerminalSize;
use portable_pty::CommandBuilder;
use std::sync::Arc;

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub enum SpawnWhere {
    InitialWindow,
    NewSession,
    SplitSession(DesktopSplitRequest),
}

pub fn spawn_command_impl(
    spawn: &SpawnCommand,
    spawn_where: SpawnWhere,
    size: TerminalSize,
    term_config: Arc<TermConfig>,
) {
    let spawn = spawn.clone();

    promise::spawn::spawn(async move {
        if let Err(err) = spawn_command_internal(spawn, spawn_where, size, term_config).await {
            log::error!("Failed to spawn: {:#}", err);
        }
    })
    .detach();
}

pub async fn spawn_command_internal(
    spawn: SpawnCommand,
    spawn_where: SpawnWhere,
    size: TerminalSize,
    term_config: Arc<TermConfig>,
) -> anyhow::Result<()> {
    let current_pane_id = desktop_current_active_terminal_handle();

    let cwd = if let Some(cwd) = spawn.cwd.as_ref() {
        Some(cwd.to_str().map(|s| s.to_owned()).ok_or_else(|| {
            anyhow!(
                "SpawnTarget::spawn requires that the cwd be unicode in {:?}",
                cwd
            )
        })?)
    } else {
        None
    };

    let cmd_builder = match (
        spawn.args.as_ref(),
        spawn.cwd.as_ref(),
        spawn.set_environment_variables.is_empty(),
    ) {
        (None, None, true) => None,
        _ => {
            let mut builder = spawn
                .args
                .as_ref()
                .map(|args| CommandBuilder::from_argv(args.iter().map(Into::into).collect()))
                .unwrap_or_else(CommandBuilder::new_default_prog);
            for (k, v) in spawn.set_environment_variables.iter() {
                builder.env(k, v);
            }
            if let Some(cwd) = &spawn.cwd {
                builder.cwd(cwd);
            }
            Some(builder)
        }
    };

    let can_use_session_view_split =
        spawn.args.is_none() && spawn.set_environment_variables.is_empty();

    match spawn_where {
        SpawnWhere::SplitSession(direction) => {
            if let Some(_session_id) = desktop_current_active_session_id() {
                if can_use_session_view_split {
                    desktop_create_split_session_view(direction, size, cwd.clone())
                        .context("create_split_session_view")?;
                    return Ok(());
                }
                anyhow::bail!(
                    "session-native terminal split has been removed; use session view split instead"
                );
            }
            anyhow::bail!("engine split fallback reached — session_id is None; this is a bug");
        }
        _ => {
            let activity = start_host_activity();
            let workspace = host_workspace_name();
            let pane = spawn_host_runtime_entry(
                cmd_builder,
                cwd,
                size,
                current_pane_id.map(|handle| handle.as_u64()),
                workspace,
                spawn.position,
            )
            .await
            .context("spawn_tab_or_window")?;

            pane.set_config(term_config);
            drop(activity);
        }
    };

    Ok(())
}
