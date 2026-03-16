use std::collections::{BTreeSet, HashMap};

use super::{TerminalInstanceId, SessionLayoutSnapshot, RuntimeId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalInstanceProcessState {
    pub process_id: Option<u32>,
    pub command_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalInstanceRuntimeState {
    pub terminal_instance_id: TerminalInstanceId,
    pub process: Option<TerminalInstanceProcessState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRuntimeRecord {
    pub session_id: String,
    pub runtime_id: RuntimeId,
    pub root_layout_node_id: Option<super::LayoutNodeId>,
    pub active_terminal_instance_id: Option<TerminalInstanceId>,
    pub layout: Option<SessionLayoutSnapshot>,
    pub leaves: HashMap<TerminalInstanceId, TerminalInstanceRuntimeState>,
}

impl SessionRuntimeRecord {
    pub fn new(session_id: impl Into<String>, runtime_id: RuntimeId) -> Self {
        Self {
            session_id: session_id.into(),
            runtime_id,
            root_layout_node_id: None,
            active_terminal_instance_id: None,
            layout: None,
            leaves: HashMap::new(),
        }
    }

    pub fn sync_layout(&mut self, layout: &SessionLayoutSnapshot) {
        self.root_layout_node_id = Some(layout.root_layout_node_id);
        self.active_terminal_instance_id = Some(layout.active_terminal_instance_id);
        self.layout = Some(layout.clone());

        let live_terminal_instance_ids: BTreeSet<_> =
            layout.leaves.iter().map(|leaf| leaf.terminal_instance_id).collect();
        self.leaves
            .retain(|terminal_instance_id, _| live_terminal_instance_ids.contains(terminal_instance_id));
        for leaf in &layout.leaves {
            self.leaves
                .entry(leaf.terminal_instance_id)
                .or_insert_with(|| TerminalInstanceRuntimeState {
                    terminal_instance_id: leaf.terminal_instance_id,
                    process: None,
                });
        }
    }

    pub fn set_leaf_process(&mut self, terminal_instance_id: TerminalInstanceId, process: TerminalInstanceProcessState) {
        self.leaves
            .entry(terminal_instance_id)
            .or_insert_with(|| TerminalInstanceRuntimeState {
                terminal_instance_id,
                process: None,
            })
            .process = Some(process);
    }

}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionCoreState {
    session_to_runtime: HashMap<String, RuntimeId>,
    runtimes: HashMap<RuntimeId, SessionRuntimeRecord>,
}

impl SessionCoreState {
    pub fn register_runtime(
        &mut self,
        session_id: impl Into<String>,
        runtime_id: RuntimeId,
    ) -> &mut SessionRuntimeRecord {
        let session_id = session_id.into();
        self.session_to_runtime
            .insert(session_id.clone(), runtime_id);
        self.runtimes
            .entry(runtime_id)
            .or_insert_with(|| SessionRuntimeRecord::new(session_id, runtime_id))
    }

    pub fn remove_runtime(&mut self, runtime_id: RuntimeId) -> Option<SessionRuntimeRecord> {
        let removed = self.runtimes.remove(&runtime_id)?;
        self.session_to_runtime
            .retain(|_, mapped_runtime_id| *mapped_runtime_id != runtime_id);
        Some(removed)
    }

    pub fn runtime_id_for_session(&self, session_id: &str) -> Option<RuntimeId> {
        self.session_to_runtime.get(session_id).copied()
    }

    /// Iterate all (session_id, runtime_id) mappings.
    pub fn session_runtime_map(&self) -> impl Iterator<Item = (&String, &RuntimeId)> {
        self.session_to_runtime.iter()
    }

    pub fn runtime(&self, runtime_id: RuntimeId) -> Option<&SessionRuntimeRecord> {
        self.runtimes.get(&runtime_id)
    }

    pub fn runtime_mut(
        &mut self,
        runtime_id: RuntimeId,
    ) -> Option<&mut SessionRuntimeRecord> {
        self.runtimes.get_mut(&runtime_id)
    }

    pub fn sync_runtime_layout(
        &mut self,
        session_id: impl Into<String>,
        runtime_id: RuntimeId,
        layout: &SessionLayoutSnapshot,
    ) -> &mut SessionRuntimeRecord {
        let runtime = self.register_runtime(session_id, runtime_id);
        runtime.sync_layout(layout);
        runtime
    }
}

#[cfg(test)]
mod tests {
    use super::super::{LayoutNodeId, RuntimeId, SessionLayoutSnapshot, TerminalInstanceId};

    use super::{TerminalInstanceProcessState, SessionCoreState, SessionRuntimeRecord};

    #[test]
    fn sync_layout_tracks_active_leaf_and_prunes_stale_leaves() {
        let mut runtime = SessionRuntimeRecord::new("session-a", RuntimeId::new(7));
        runtime.set_leaf_process(
            TerminalInstanceId::new(22),
            TerminalInstanceProcessState {
                process_id: Some(99),
                command_label: Some("shell".into()),
            },
        );
        runtime.sync_layout(&SessionLayoutSnapshot::single_terminal_instance(
            LayoutNodeId::new(11),
            TerminalInstanceId::new(33),
            None,
        ));

        assert_eq!(runtime.root_layout_node_id, Some(LayoutNodeId::new(11)));
        assert_eq!(runtime.active_terminal_instance_id, Some(TerminalInstanceId::new(33)));
        assert_eq!(
            runtime
                .layout
                .as_ref()
                .map(|layout| layout.root_layout_node_id),
            Some(LayoutNodeId::new(11))
        );
        assert!(runtime.leaves.contains_key(&TerminalInstanceId::new(33)));
        assert!(!runtime.leaves.contains_key(&TerminalInstanceId::new(22)));
    }

    #[test]
    fn session_core_state_maps_session_to_runtime_and_removes_reverse_index() {
        let mut state = SessionCoreState::default();
        state.register_runtime("session-a", RuntimeId::new(7));
        assert_eq!(
            state.runtime_id_for_session("session-a"),
            Some(RuntimeId::new(7))
        );
        assert!(state.remove_runtime(RuntimeId::new(7)).is_some());
        assert_eq!(state.runtime_id_for_session("session-a"), None);
    }
}
