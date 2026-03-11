use std::collections::{BTreeSet, HashMap};

use crate::{LeafId, SessionLayoutSnapshot, SessionSurfaceLookup, SurfaceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafProcessState {
    pub process_id: Option<u32>,
    pub command_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafRuntimeState {
    pub leaf_id: LeafId,
    pub process: Option<LeafProcessState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceRuntimeState {
    pub session_id: String,
    pub surface_id: SurfaceId,
    pub root_layout_node_id: Option<crate::LayoutNodeId>,
    pub active_leaf_id: Option<LeafId>,
    pub layout: Option<SessionLayoutSnapshot>,
    pub leaves: HashMap<LeafId, LeafRuntimeState>,
}

impl SurfaceRuntimeState {
    pub fn new(session_id: impl Into<String>, surface_id: SurfaceId) -> Self {
        Self {
            session_id: session_id.into(),
            surface_id,
            root_layout_node_id: None,
            active_leaf_id: None,
            layout: None,
            leaves: HashMap::new(),
        }
    }

    pub fn sync_layout(&mut self, layout: &SessionLayoutSnapshot) {
        self.root_layout_node_id = Some(layout.root_layout_node_id);
        self.active_leaf_id = Some(layout.active_leaf_id);
        self.layout = Some(layout.clone());

        let live_leaf_ids: BTreeSet<_> = layout.leaves.iter().map(|leaf| leaf.leaf_id).collect();
        self.leaves
            .retain(|leaf_id, _| live_leaf_ids.contains(leaf_id));
        for leaf in &layout.leaves {
            self.leaves
                .entry(leaf.leaf_id)
                .or_insert_with(|| LeafRuntimeState {
                    leaf_id: leaf.leaf_id,
                    process: None,
                });
        }
    }

    pub fn set_leaf_process(&mut self, leaf_id: LeafId, process: LeafProcessState) {
        self.leaves
            .entry(leaf_id)
            .or_insert_with(|| LeafRuntimeState {
                leaf_id,
                process: None,
            })
            .process = Some(process);
    }

    pub fn clear_leaf_process(&mut self, leaf_id: LeafId) {
        if let Some(leaf) = self.leaves.get_mut(&leaf_id) {
            leaf.process = None;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionCoreState {
    session_to_surface: HashMap<String, SurfaceId>,
    surfaces: HashMap<SurfaceId, SurfaceRuntimeState>,
}

impl SessionCoreState {
    pub fn register_surface(
        &mut self,
        session_id: impl Into<String>,
        surface_id: SurfaceId,
    ) -> &mut SurfaceRuntimeState {
        let session_id = session_id.into();
        self.session_to_surface
            .insert(session_id.clone(), surface_id);
        self.surfaces
            .entry(surface_id)
            .or_insert_with(|| SurfaceRuntimeState::new(session_id, surface_id))
    }

    pub fn remove_surface(&mut self, surface_id: SurfaceId) -> Option<SurfaceRuntimeState> {
        let removed = self.surfaces.remove(&surface_id)?;
        self.session_to_surface
            .retain(|_, mapped_surface_id| *mapped_surface_id != surface_id);
        Some(removed)
    }

    pub fn surface_id_for_session(&self, session_id: &str) -> Option<SurfaceId> {
        self.session_to_surface.get(session_id).copied()
    }

    pub fn surface(&self, surface_id: SurfaceId) -> Option<&SurfaceRuntimeState> {
        self.surfaces.get(&surface_id)
    }

    pub fn surface_mut(&mut self, surface_id: SurfaceId) -> Option<&mut SurfaceRuntimeState> {
        self.surfaces.get_mut(&surface_id)
    }

    pub fn reconcile_lookup(&mut self, lookup: &SessionSurfaceLookup) {
        let live_surface_ids: BTreeSet<_> =
            lookup.surface_ids_by_session.values().copied().collect();
        self.surfaces
            .retain(|surface_id, _| live_surface_ids.contains(surface_id));
        self.session_to_surface.retain(|session_id, surface_id| {
            lookup.surface_ids_by_session.get(session_id) == Some(surface_id)
        });

        for (session_id, surface_id) in &lookup.surface_ids_by_session {
            self.register_surface(session_id.clone(), *surface_id);
        }
    }

    pub fn sync_surface_layout(
        &mut self,
        session_id: impl Into<String>,
        surface_id: SurfaceId,
        layout: &SessionLayoutSnapshot,
    ) -> &mut SurfaceRuntimeState {
        let surface = self.register_surface(session_id, surface_id);
        surface.sync_layout(layout);
        surface
    }
}

#[cfg(test)]
mod tests {
    use crate::{LayoutNodeId, SessionLayoutSnapshot, SessionSurfaceLookup};

    use super::{LeafProcessState, SessionCoreState, SurfaceRuntimeState};

    #[test]
    fn sync_layout_tracks_active_leaf_and_prunes_stale_leaves() {
        let mut surface = SurfaceRuntimeState::new("session-a", crate::SurfaceId::new(7));
        surface.set_leaf_process(
            crate::LeafId::new(22),
            LeafProcessState {
                process_id: Some(99),
                command_label: Some("shell".into()),
            },
        );
        surface.sync_layout(&SessionLayoutSnapshot::single_leaf(
            LayoutNodeId::new(11),
            crate::LeafId::new(33),
            None,
        ));

        assert_eq!(surface.root_layout_node_id, Some(LayoutNodeId::new(11)));
        assert_eq!(surface.active_leaf_id, Some(crate::LeafId::new(33)));
        assert_eq!(
            surface
                .layout
                .as_ref()
                .map(|layout| layout.root_layout_node_id),
            Some(LayoutNodeId::new(11))
        );
        assert!(surface.leaves.contains_key(&crate::LeafId::new(33)));
        assert!(!surface.leaves.contains_key(&crate::LeafId::new(22)));
    }

    #[test]
    fn session_core_state_maps_session_to_surface_and_removes_reverse_index() {
        let mut state = SessionCoreState::default();
        state.register_surface("session-a", crate::SurfaceId::new(7));
        assert_eq!(
            state.surface_id_for_session("session-a"),
            Some(crate::SurfaceId::new(7))
        );
        assert!(state.remove_surface(crate::SurfaceId::new(7)).is_some());
        assert_eq!(state.surface_id_for_session("session-a"), None);
    }

    #[test]
    fn reconcile_lookup_prunes_stale_surfaces_and_primes_missing_mappings() {
        let mut state = SessionCoreState::default();
        state.register_surface("session-a", crate::SurfaceId::new(7));
        state.register_surface("session-stale", crate::SurfaceId::new(8));

        state.reconcile_lookup(&SessionSurfaceLookup {
            surface_ids_by_session: [("session-b".to_string(), crate::SurfaceId::new(9))].into(),
            ..SessionSurfaceLookup::default()
        });

        assert_eq!(state.surface_id_for_session("session-a"), None);
        assert_eq!(state.surface_id_for_session("session-stale"), None);
        assert_eq!(
            state.surface_id_for_session("session-b"),
            Some(crate::SurfaceId::new(9))
        );
        assert!(state.surface(crate::SurfaceId::new(8)).is_none());
        assert!(state.surface(crate::SurfaceId::new(9)).is_some());
    }
}
