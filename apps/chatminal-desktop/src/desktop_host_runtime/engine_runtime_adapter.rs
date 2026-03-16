use std::sync::Arc;
use std::convert::TryFrom;

use super::session_engine::{
    build_layout_snapshot_from_tree, EngineRuntimeAdapter, EngineRuntimeRef, TerminalInstanceId,
    MoveTerminalInstanceTarget, SessionLayoutTreeNode, SessionRuntimeState,
    SpawnSessionRuntimeRequest, RuntimeId,
};
use config::keyassignment::SessionDirection;
use engine_dynamic::Value;
use window::{Window, WindowOps};

use super::{
    host_render_scope_capability, HostLayoutNode, HostMux, HostRenderScope,
    HostRuntimeEntryId as RuntimeEntryId,
    HostSplitDirection as SplitDirection, HostTerminal,
    RuntimeWindowId as EngineWindowId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DesktopEngineRuntimeAdapter {
    window_id: EngineWindowId,
}

impl DesktopEngineRuntimeAdapter {
    pub(crate) const fn new(window_id: EngineWindowId) -> Self {
        Self { window_id }
    }

    fn render_scope_id_for_session(&self, session_id: &str) -> Option<RuntimeEntryId> {
        let mux = HostMux::get();
        mux.get_window(self.window_id).and_then(|window| {
            window.iter().find_map(|render_scope| {
                (render_scope_session_id(render_scope).as_deref() == Some(session_id))
                    .then(|| render_scope.tab_id())
            })
        })
    }

    fn render_scope_id_for_runtime(&self, runtime_id: RuntimeId) -> Option<RuntimeEntryId> {
        let mux = HostMux::get();
        mux.get_window(self.window_id).and_then(|window| {
            window
                .iter()
                .find(|render_scope| runtime_id_for_render_scope(render_scope) == runtime_id)
                .map(|render_scope| render_scope.tab_id())
        })
    }

    fn spawn_runtime_inner(&self, request: SpawnSessionRuntimeRequest, window: Option<Window>) {
        let window_id = self.window_id;
        promise::spawn::spawn(async move {
            match HostMux::get()
                .spawn_tab_or_window(
                    Some(window_id),
                    request.domain,
                    Some(request.command),
                    None,
                    request.terminal_size,
                    request
                        .current_host_handle
                        .and_then(|handle| usize::try_from(handle).ok()),
                    request.workspace,
                    None,
                )
                .await
            {
                Ok(_) => {
                    if let Some(window) = window.as_ref() {
                        window.invalidate();
                    }
                }
                Err(err) => {
                    log::error!(
                        "failed to switch chatminal sidebar session {}: {:#}",
                        request.session_id,
                        err
                    );
                }
            }
        })
        .detach();
    }
}

impl EngineRuntimeAdapter for DesktopEngineRuntimeAdapter {
    type Error = &'static str;

    fn attach_runtime(&self, session_id: &str) -> Result<EngineRuntimeRef, Self::Error> {
        self.render_scope_id_for_session(session_id)
            .and_then(|render_scope_id| host_render_scope_capability(render_scope_id as u64))
            .map(|render_scope| EngineRuntimeRef {
                runtime_id: runtime_id_for_render_scope(&render_scope),
                session_id: session_id.to_string(),
            })
            .ok_or("session runtime not found")
    }

    fn focus_runtime(&self, runtime_id: RuntimeId) -> Result<(), Self::Error> {
        let render_scope_id = self
            .render_scope_id_for_runtime(runtime_id)
            .or_else(|| render_scope_id_from_runtime(runtime_id).ok())
            .ok_or("render scope missing")?;
        let mux = HostMux::get();
        let pane = host_render_scope_capability(render_scope_id as u64)
            .and_then(|render_scope| render_scope.get_active_pane())
            .ok_or("runtime has no active pane")?;
        let render_scope =
            host_render_scope_capability(render_scope_id as u64).ok_or("render scope missing")?;
        render_scope.set_active_pane(&pane);
        let Some(mut window) = mux.get_window_mut(self.window_id) else {
            return Err("window missing");
        };
        let Some(idx) = window.idx_by_id(render_scope_id) else {
            return Err("host tab not attached to window");
        };
        window.save_and_then_set_active(idx);
        Ok(())
    }

    fn focus_terminal_instance(&self, runtime_id: RuntimeId, terminal_instance_id: TerminalInstanceId) -> Result<(), Self::Error> {
        let render_scope_id = self
            .render_scope_id_for_runtime(runtime_id)
            .or_else(|| render_scope_id_from_runtime(runtime_id).ok())
            .ok_or("render scope missing")?;
        let render_scope =
            host_render_scope_capability(render_scope_id as u64).ok_or("render scope missing")?;
        let pane = render_scope
            .iter_panes()
            .into_iter()
            .find(|pos| {
                pane_terminal_instance_id(&pos.pane).unwrap_or_else(|| TerminalInstanceId::new(pos.pane.pane_id() as u64))
                    == terminal_instance_id
            })
            .map(|pos| pos.pane)
            .ok_or("leaf not found in runtime")?;
        render_scope.set_active_pane(&pane);
        HostMux::get()
            .focus_pane_and_containing_tab(pane.pane_id())
            .map_err(|_| "focus leaf failed")
    }

    fn adjacent_active_terminal_instance(
        &self,
        runtime_id: RuntimeId,
        direction: SessionDirection,
    ) -> Result<Option<TerminalInstanceId>, Self::Error> {
        let render_scope_id = self
            .render_scope_id_for_runtime(runtime_id)
            .or_else(|| render_scope_id_from_runtime(runtime_id).ok())
            .ok_or("render scope missing")?;
        let render_scope =
            host_render_scope_capability(render_scope_id as u64).ok_or("render scope missing")?;
        let Some(target_index) = render_scope.get_pane_direction(direction, false) else {
            return Ok(None);
        };
        Ok(render_scope
            .iter_panes()
            .into_iter()
            .find(|pos| pos.index == target_index)
            .map(|pos| {
                pane_terminal_instance_id(&pos.pane).unwrap_or_else(|| TerminalInstanceId::new(pos.pane.pane_id() as u64))
            }))
    }

    fn swap_active_terminal_instance(
        &self,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        keep_focus: bool,
    ) -> Result<(), Self::Error> {
        let render_scope_id = self
            .render_scope_id_for_runtime(runtime_id)
            .or_else(|| render_scope_id_from_runtime(runtime_id).ok())
            .ok_or("render scope missing")?;
        let render_scope =
            host_render_scope_capability(render_scope_id as u64).ok_or("render scope missing")?;
        let target_index = render_scope
            .iter_panes()
            .into_iter()
            .find(|pos| {
                pane_terminal_instance_id(&pos.pane).unwrap_or_else(|| TerminalInstanceId::new(pos.pane.pane_id() as u64))
                    == terminal_instance_id
            })
            .map(|pos| pos.index)
            .ok_or("leaf not found in runtime")?;
        render_scope
            .swap_active_with_index(target_index, keep_focus)
            .ok_or("swap active leaf failed")
    }

    fn move_terminal_instance(
        &self,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        target: MoveTerminalInstanceTarget,
    ) -> Result<(), Self::Error> {
        let render_scope_id = self
            .render_scope_id_for_runtime(runtime_id)
            .or_else(|| render_scope_id_from_runtime(runtime_id).ok())
            .ok_or("render scope missing")?;
        let render_scope =
            host_render_scope_capability(render_scope_id as u64).ok_or("render scope missing")?;
        let terminal_handle = render_scope
            .iter_panes()
            .into_iter()
            .find(|pos| {
                pane_terminal_instance_id(&pos.pane).unwrap_or_else(|| TerminalInstanceId::new(pos.pane.pane_id() as u64))
                    == terminal_instance_id
            })
            .map(|pos| pos.pane.pane_id())
            .ok_or("leaf not found in runtime")?;
        let window_id = self.window_id;
        promise::spawn::spawn(async move {
            let mux = HostMux::get();
            let result = match target {
                MoveTerminalInstanceTarget::NewWindow => mux.move_pane_to_new_tab(terminal_handle, None, None).await,
                MoveTerminalInstanceTarget::NewRuntimeInWindow => {
                    mux.move_pane_to_new_tab(terminal_handle, Some(window_id), None).await
                }
            };
            if let Err(err) = result {
                log::error!("failed to move leaf {terminal_instance_id}: {err:#}");
                return;
            }
            if matches!(target, MoveTerminalInstanceTarget::NewRuntimeInWindow) {
                let _ = mux.focus_pane_and_containing_tab(terminal_handle);
            }
        })
        .detach();
        Ok(())
    }

    fn close_runtime(&self, runtime_id: RuntimeId) -> Result<(), Self::Error> {
        let render_scope_id = self
            .render_scope_id_for_runtime(runtime_id)
            .or_else(|| render_scope_id_from_runtime(runtime_id).ok())
            .ok_or("render scope missing")?;
        HostMux::get().remove_tab(render_scope_id);
        Ok(())
    }

    fn spawn_runtime(
        &self,
        request: SpawnSessionRuntimeRequest,
        window: Option<Window>,
    ) -> Result<(), Self::Error> {
        self.spawn_runtime_inner(request, window);
        Ok(())
    }

    fn snapshot_runtime(&self, runtime_id: RuntimeId) -> Result<SessionRuntimeState, Self::Error> {
        let render_scope_id = self
            .render_scope_id_for_runtime(runtime_id)
            .or_else(|| render_scope_id_from_runtime(runtime_id).ok())
            .ok_or("render scope missing")?;
        let render_scope =
            host_render_scope_capability(render_scope_id as u64).ok_or("render scope missing")?;
        let session_id = render_scope_session_id(&render_scope).ok_or("runtime session id missing")?;
        let active_terminal_pane = render_scope
            .get_active_pane()
            .ok_or("runtime has no active pane")?;
        let pane_tree = render_scope.codec_pane_tree();
        let mut state = SessionRuntimeState::detached(session_id, runtime_id);
        let active_terminal_instance_id = pane_terminal_instance_id(&active_terminal_pane)
            .unwrap_or_else(|| TerminalInstanceId::new(active_terminal_pane.pane_id() as u64));
        if let Some(layout) = Some(active_terminal_instance_id)
            .filter(|terminal_instance_id| active_terminal_pane.pane_id() as u64 == terminal_instance_id.as_u64())
            .map(|terminal_instance_id| {
                super::session_engine::SessionLayoutSnapshot::single_terminal_instance(
                    metadata_layout_node_id(runtime_id, terminal_instance_id),
                    terminal_instance_id,
                    Some(active_terminal_pane.get_title()),
                )
            })
            .or_else(|| {
                let layout_tree = build_layout_tree_from_mux(&pane_tree);
                build_layout_snapshot_from_tree(render_scope_id as u64, active_terminal_instance_id, &layout_tree)
            })
        {
            state.attach_layout(layout);
        }
        Ok(state)
    }
}

fn pane_session_id(pane: &Arc<dyn HostTerminal>) -> Option<String> {
    pane_metadata_string(pane, "chatminal_session_id")
}

fn render_scope_session_id(render_scope: &Arc<HostRenderScope>) -> Option<String> {
    for positioned in render_scope.iter_panes() {
        if let Some(session_id) = pane_session_id(&positioned.pane) {
            return Some(session_id);
        }
    }
    None
}

fn runtime_id_for_render_scope(render_scope: &Arc<HostRenderScope>) -> RuntimeId {
    for positioned in render_scope.iter_panes() {
        if let Some(runtime_id) = pane_runtime_id(&positioned.pane) {
            return runtime_id;
        }
    }
    RuntimeId::new(render_scope.tab_id() as u64)
}

fn render_scope_id_from_runtime(runtime_id: RuntimeId) -> Result<RuntimeEntryId, &'static str> {
    usize::try_from(runtime_id.as_u64())
        .map(|value| value as RuntimeEntryId)
        .map_err(|_| "runtime id out of range")
}

fn pane_runtime_id(pane: &Arc<dyn HostTerminal>) -> Option<RuntimeId> {
    pane_metadata_u64(pane, "chatminal_runtime_id").map(RuntimeId::new)
}

fn pane_terminal_instance_id(pane: &Arc<dyn HostTerminal>) -> Option<TerminalInstanceId> {
    pane_metadata_u64(pane, "chatminal_terminal_instance_id").map(TerminalInstanceId::new)
}

fn pane_metadata_string(pane: &Arc<dyn HostTerminal>, key: &str) -> Option<String> {
    match pane.get_metadata() {
        Value::Object(obj) => match obj.get(&Value::String(key.to_string())) {
            Some(Value::String(value)) if !value.trim().is_empty() => Some(value.to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn pane_metadata_u64(pane: &Arc<dyn HostTerminal>, key: &str) -> Option<u64> {
    match pane.get_metadata() {
        Value::Object(obj) => obj
            .get(&Value::String(key.to_string()))
            .and_then(Value::coerce_unsigned),
        _ => None,
    }
}

fn metadata_layout_node_id(runtime_id: RuntimeId, terminal_instance_id: TerminalInstanceId) -> super::session_engine::LayoutNodeId {
    super::session_engine::LayoutNodeId::new(
        (runtime_id.as_u64() << 32) | (terminal_instance_id.as_u64() & 0xffff_ffff),
    )
}

fn build_layout_tree_from_mux(node: &HostLayoutNode) -> SessionLayoutTreeNode {
    match node {
        HostLayoutNode::Empty => SessionLayoutTreeNode::Empty,
        HostLayoutNode::Leaf(entry) => SessionLayoutTreeNode::TerminalInstance {
            terminal_instance_id: TerminalInstanceId::new(entry.pane_id as u64),
            title: Some(entry.title.clone()),
        },
        HostLayoutNode::Split { left, right, node } => SessionLayoutTreeNode::Split {
            axis: split_axis(node.direction),
            first: Box::new(build_layout_tree_from_mux(left)),
            second: Box::new(build_layout_tree_from_mux(right)),
        },
    }
}

fn split_axis(direction: SplitDirection) -> super::session_engine::SessionSplitAxis {
    match direction {
        SplitDirection::Horizontal => super::session_engine::SessionSplitAxis::Horizontal,
        SplitDirection::Vertical => super::session_engine::SessionSplitAxis::Vertical,
    }
}
