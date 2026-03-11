use crate::{LayoutNodeId, LeafId, SessionLayoutSnapshot, SessionSurfaceSnapshot, SurfaceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSurfaceState {
    pub snapshot: SessionSurfaceSnapshot,
    pub layout: Option<SessionLayoutSnapshot>,
}

impl SessionSurfaceState {
    pub fn detached(session_id: impl Into<String>, surface_id: SurfaceId) -> Self {
        Self {
            snapshot: SessionSurfaceSnapshot::new(session_id, surface_id),
            layout: None,
        }
    }

    pub fn attach_layout(&mut self, layout: SessionLayoutSnapshot) {
        self.snapshot.active_leaf_id = Some(layout.active_leaf_id);
        self.snapshot.root_layout_node_id = Some(layout.root_layout_node_id);
        self.layout = Some(layout);
    }

    pub fn render_target_for_leaf(&self, leaf_id: LeafId) -> Option<(LayoutNodeId, LeafId)> {
        let layout = self.layout.as_ref()?;
        let layout_node_id = layout.resolve_leaf_layout_node(leaf_id)?;
        Some((layout_node_id, leaf_id))
    }

    pub fn active_render_target(&self) -> Option<(LayoutNodeId, LeafId)> {
        self.snapshot
            .active_leaf_id
            .and_then(|leaf_id| self.render_target_for_leaf(leaf_id))
    }
}

#[cfg(test)]
mod tests {
    use crate::{LeafId, SessionLayoutSnapshot, SessionSurfaceState, SurfaceId};

    #[test]
    fn active_render_target_resolves_from_layout_snapshot() {
        let mut state = SessionSurfaceState::detached("session-a", SurfaceId::new(7));
        state.attach_layout(SessionLayoutSnapshot::single_leaf(
            crate::LayoutNodeId::new(17),
            LeafId::new(19),
            None,
        ));

        assert_eq!(
            state.active_render_target(),
            Some((crate::LayoutNodeId::new(17), LeafId::new(19)))
        );
        assert_eq!(
            state.render_target_for_leaf(LeafId::new(19)),
            Some((crate::LayoutNodeId::new(17), LeafId::new(19)))
        );
        assert_eq!(state.render_target_for_leaf(LeafId::new(99)), None);
    }
}
