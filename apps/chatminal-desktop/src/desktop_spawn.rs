use anyhow::{Context, anyhow, bail};
use crate::chatminal_runtime::DesktopSplitRequest;
use crate::scripting::guiwin::DesktopWindowId;
use config::TermConfig;
use config::keyassignment::SpawnCommand;
use engine_term::TerminalSize;
use portable_pty::CommandBuilder;
use std::sync::Arc;

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub enum SpawnWhere {
    NewWindow,
    NewSession,
    SplitSession(DesktopSplitRequest),
}

pub fn spawn_command_impl(
    spawn: &SpawnCommand,
    spawn_where: SpawnWhere,
    size: TerminalSize,
    src_window_id: Option<DesktopWindowId>,
    term_config: Arc<TermConfig>,
) {
    let spawn = spawn.clone();

    promise::spawn::spawn(async move {
        if let Err(err) =
            spawn_command_internal(spawn, spawn_where, size, src_window_id, term_config).await
        {
            log::error!("Failed to spawn: {:#}", err);
        }
    })
    .detach();
}

pub async fn spawn_command_internal(
    spawn: SpawnCommand,
    spawn_where: SpawnWhere,
    size: TerminalSize,
    src_window_id: Option<DesktopWindowId>,
    term_config: Arc<TermConfig>,
) -> anyhow::Result<()> {
    let engine_src_window_id = src_window_id.map(|window_id| window_id as u64);

    let current_pane_id = match engine_src_window_id {
        Some(window_id) => crate::chatminal_runtime::desktop_current_active_terminal_handle(
            window_id as DesktopWindowId,
        ),
        None => None,
    };

    let cwd = if let Some(cwd) = spawn.cwd.as_ref() {
        Some(cwd.to_str().map(|s| s.to_owned()).ok_or_else(|| {
            anyhow!(
                "Domain::spawn requires that the cwd be unicode in {:?}",
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
            let src_window_id = match engine_src_window_id {
                Some(id) => id,
                None => anyhow::bail!("no src window when splitting a pane?"),
            };
            if let Some(_session_id) =
                crate::chatminal_runtime::desktop_current_active_session_id(src_window_id as DesktopWindowId)
            {
                if can_use_session_view_split {
                    crate::chatminal_runtime::desktop_create_split_session_view(
                        src_window_id as DesktopWindowId,
                        direction,
                        size,
                        cwd.clone(),
                    )
                    .context("create_split_session_view")?;
                    return Ok(());
                }
                anyhow::bail!(
                    "session-native terminal split has been removed; use session view split instead"
                );
            }
            log::warn!(
                "engine split fallback triggered for pane_id={:?}",
                current_pane_id
            );
            let activity = crate::chatminal_runtime::start_host_activity();
            if let Some(pane_id) = current_pane_id {
                log::trace!("doing split_pane");
                let (pane, _size) = crate::chatminal_runtime::split_terminal_handle_by_public_id(
                    pane_id.as_u64(),
                    direction.into_host_request(),
                    cmd_builder,
                    cwd,
                    spawn.domain,
                )
                    .await
                    .context("split_pane")?;
                pane.set_config(term_config);
            } else {
                bail!("there is no active pane while splitting pane!?");
            }
            drop(activity);
        }
        _ => {
            let activity = crate::chatminal_runtime::start_host_activity();
            let workspace = crate::chatminal_runtime::host_workspace_name();
            let (pane, window_id) = crate::chatminal_runtime::spawn_host_runtime_entry(
                match spawn_where {
                    SpawnWhere::NewWindow => None,
                    _ => engine_src_window_id,
                },
                spawn.domain,
                cmd_builder,
                cwd,
                size,
                current_pane_id.map(|handle| handle.as_u64()),
                workspace,
                spawn.position,
            )
                .await
                .context("spawn_tab_or_window")?;

            // If it was created in this window, it copies our handlers.
            // Otherwise, we'll pick them up when we later respond to
            // the new window being created.
            if Some(window_id) == engine_src_window_id {
                pane.set_config(term_config);
            }
            drop(activity);
        }
    };

    Ok(())
}
