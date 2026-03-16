use super::{LayoutNodeId, TerminalInstanceId, SessionLayoutSnapshot, SessionRuntimeSnapshot, RuntimeId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRuntimeState {
    pub snapshot: SessionRuntimeSnapshot,
    pub layout: Option<SessionLayoutSnapshot>,
}

impl SessionRuntimeState {
    pub fn detached(session_id: impl Into<String>, runtime_id: RuntimeId) -> Self {
        Self {
            snapshot: SessionRuntimeSnapshot::new(session_id, runtime_id),
            layout: None,
        }
    }

    pub fn attach_layout(&mut self, layout: SessionLayoutSnapshot) {
        self.snapshot.active_terminal_instance_id = Some(layout.active_terminal_instance_id);
        self.snapshot.root_layout_node_id = Some(layout.root_layout_node_id);
        self.layout = Some(layout);
    }

    pub fn render_target_for_terminal_instance(&self, terminal_instance_id: TerminalInstanceId) -> Option<(LayoutNodeId, TerminalInstanceId)> {
        let layout = self.layout.as_ref()?;
        let layout_node_id = layout.resolve_terminal_instance_layout_node(terminal_instance_id)?;
        Some((layout_node_id, terminal_instance_id))
    }

    pub fn active_render_target(&self) -> Option<(LayoutNodeId, TerminalInstanceId)> {
        self.snapshot
            .active_terminal_instance_id
            .and_then(|terminal_instance_id| self.render_target_for_terminal_instance(terminal_instance_id))
    }
}

#[cfg(test)]
mod tests {
    use super::{TerminalInstanceId, SessionLayoutSnapshot, SessionRuntimeState, RuntimeId};

    #[test]
    fn active_render_target_resolves_from_layout_snapshot() {
        let mut state = SessionRuntimeState::detached("session-a", RuntimeId::new(7));
        state.attach_layout(SessionLayoutSnapshot::single_terminal_instance(
            super::LayoutNodeId::new(17),
            TerminalInstanceId::new(19),
            None,
        ));

        assert_eq!(
            state.active_render_target(),
            Some((super::LayoutNodeId::new(17), TerminalInstanceId::new(19)))
        );
        assert_eq!(
            state.render_target_for_terminal_instance(TerminalInstanceId::new(19)),
            Some((super::LayoutNodeId::new(17), TerminalInstanceId::new(19)))
        );
        assert_eq!(state.render_target_for_terminal_instance(TerminalInstanceId::new(99)), None);
    }
}
