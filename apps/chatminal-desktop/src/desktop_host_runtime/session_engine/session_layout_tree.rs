use super::{LayoutNodeId, TerminalInstanceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionTerminalInstanceSnapshot {
    pub terminal_instance_id: TerminalInstanceId,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionLayoutNodeKind {
    Leaf {
        terminal_instance_id: TerminalInstanceId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionLayoutNodeSnapshot {
    pub layout_node_id: LayoutNodeId,
    pub kind: SessionLayoutNodeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionLayoutSnapshot {
    pub root_layout_node_id: LayoutNodeId,
    pub active_terminal_instance_id: TerminalInstanceId,
    pub nodes: Vec<SessionLayoutNodeSnapshot>,
    pub leaves: Vec<SessionTerminalInstanceSnapshot>,
}

impl SessionLayoutSnapshot {
    pub fn single_terminal_instance(
        root_layout_node_id: LayoutNodeId,
        active_terminal_instance_id: TerminalInstanceId,
        title: Option<String>,
    ) -> Self {
        Self {
            root_layout_node_id,
            active_terminal_instance_id,
            nodes: vec![SessionLayoutNodeSnapshot {
                layout_node_id: root_layout_node_id,
                kind: SessionLayoutNodeKind::Leaf {
                    terminal_instance_id: active_terminal_instance_id,
                },
            }],
            leaves: vec![SessionTerminalInstanceSnapshot {
                terminal_instance_id: active_terminal_instance_id,
                title,
            }],
        }
    }

    #[cfg(test)]
    pub fn leaf(
        &self,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<&SessionTerminalInstanceSnapshot> {
        self.leaves
            .iter()
            .find(|leaf| leaf.terminal_instance_id == terminal_instance_id)
    }

    #[cfg(test)]
    pub fn node(&self, layout_node_id: LayoutNodeId) -> Option<&SessionLayoutNodeSnapshot> {
        self.nodes
            .iter()
            .find(|node| node.layout_node_id == layout_node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{LayoutNodeId, TerminalInstanceId};
    use super::{SessionLayoutNodeKind, SessionLayoutSnapshot};

    #[test]
    fn single_terminal_instance_layout_uses_stable_ids() {
        let layout = SessionLayoutSnapshot::single_terminal_instance(
            LayoutNodeId::new(11),
            TerminalInstanceId::new(22),
            None,
        );

        assert_eq!(layout.root_layout_node_id, LayoutNodeId::new(11));
        assert_eq!(layout.active_terminal_instance_id, TerminalInstanceId::new(22));
        assert_eq!(layout.leaves.len(), 1);
        assert_eq!(layout.nodes.len(), 1);
        assert!(layout.leaf(TerminalInstanceId::new(22)).is_some());
        assert!(layout.leaf(TerminalInstanceId::new(99)).is_none());
        assert_eq!(
            layout.node(LayoutNodeId::new(11)).expect("node").kind,
            SessionLayoutNodeKind::Leaf {
                terminal_instance_id: TerminalInstanceId::new(22),
            }
        );
    }
}
