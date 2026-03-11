use anyhow::{Context, anyhow, bail};
use crate::scripting::guiwin::DesktopWindowId;
use config::TermConfig;
use config::keyassignment::SpawnCommand;
use engine_term::TerminalSize;
use mux::Mux;
use mux::activity::Activity;
use mux::domain::SplitSource;
use mux::tab::SplitRequest;
use mux::window::WindowId as EngineWindowId;
use portable_pty::CommandBuilder;
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub enum SpawnWhere {
    NewWindow,
    NewTab,
    SplitPane(SplitRequest),
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
    let mux = Mux::get();
    let activity = Activity::new();
    let engine_src_window_id = src_window_id
        .map(EngineWindowId::try_from)
        .transpose()
        .map_err(|_| anyhow!("invalid desktop window id"))?;

    let current_pane_id = match engine_src_window_id {
        Some(window_id) => mux.get_active_tab_for_window(window_id).and_then(|tab| {
            crate::chatminal_session_surface::active_session_id(window_id as DesktopWindowId)
                .and_then(|session_id| {
                    crate::chatminal_session_surface::active_leaf_id(
                        window_id as DesktopWindowId,
                        &session_id,
                    )
                })
                .and_then(|leaf_id| usize::try_from(leaf_id.as_u64()).ok())
                .or_else(|| tab.get_active_pane().map(|p| p.pane_id()))
        }),
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

    let workspace = mux.active_workspace().clone();

    match spawn_where {
        SpawnWhere::SplitPane(direction) => {
            let _src_window_id = match engine_src_window_id {
                Some(id) => id,
                None => anyhow::bail!("no src window when splitting a pane?"),
            };
            if let Some(pane_id) = current_pane_id {
                log::trace!("doing split_pane");
                let (pane, _size) = mux
                    .split_pane(
                        pane_id,
                        direction,
                        SplitSource::Spawn {
                            command: cmd_builder,
                            command_dir: cwd,
                        },
                        spawn.domain,
                    )
                    .await
                    .context("split_pane")?;
                pane.set_config(term_config);
            } else {
                bail!("there is no active pane while splitting pane!?");
            }
        }
        _ => {
            let (_tab, pane, window_id) = mux
                .spawn_tab_or_window(
                    match spawn_where {
                        SpawnWhere::NewWindow => None,
                        _ => engine_src_window_id,
                    },
                    spawn.domain,
                    cmd_builder,
                    cwd,
                    size,
                    current_pane_id,
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
        }
    };

    drop(activity);

    Ok(())
}
