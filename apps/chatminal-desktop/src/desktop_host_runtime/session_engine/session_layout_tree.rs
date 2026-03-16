use super::{LayoutNodeId, TerminalInstanceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionSplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionTerminalInstanceSnapshot {
    pub terminal_instance_id: TerminalInstanceId,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionLayoutTreeNode {
    Empty,
    TerminalInstance {
        terminal_instance_id: TerminalInstanceId,
        title: Option<String>,
    },
    Split {
        axis: SessionSplitAxis,
        first: Box<SessionLayoutTreeNode>,
        second: Box<SessionLayoutTreeNode>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionLayoutNodeKind {
    Leaf {
        terminal_instance_id: TerminalInstanceId,
    },
    Split {
        axis: SessionSplitAxis,
        first: LayoutNodeId,
        second: LayoutNodeId,
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

    pub fn leaf(&self, terminal_instance_id: TerminalInstanceId) -> Option<&SessionTerminalInstanceSnapshot> {
        self.leaves.iter().find(|leaf| leaf.terminal_instance_id == terminal_instance_id)
    }

    pub fn node(&self, layout_node_id: LayoutNodeId) -> Option<&SessionLayoutNodeSnapshot> {
        self.nodes
            .iter()
            .find(|node| node.layout_node_id == layout_node_id)
    }

    pub fn remove_terminal_instance(&self, terminal_instance_id: TerminalInstanceId) -> Option<Self> {
        let rebuilt = rebuild_layout_node(self, self.root_layout_node_id, terminal_instance_id)?;
        let mut nodes = Vec::new();
        let mut leaves = Vec::new();
        collect_layout_node(&rebuilt, &mut nodes, &mut leaves);
        let active_terminal_instance_id = if leaves.iter().any(|leaf| leaf.terminal_instance_id == self.active_terminal_instance_id) {
            self.active_terminal_instance_id
        } else {
            leaves.first()?.terminal_instance_id
        };
        Some(Self {
            root_layout_node_id: rebuilt.layout_node_id(),
            active_terminal_instance_id,
            nodes,
            leaves,
        })
    }
}

#[derive(Clone, Debug)]
enum RebuiltLayoutNode {
    Leaf {
        layout_node_id: LayoutNodeId,
        leaf: SessionTerminalInstanceSnapshot,
    },
    Split {
        layout_node_id: LayoutNodeId,
        axis: SessionSplitAxis,
        first: Box<RebuiltLayoutNode>,
        second: Box<RebuiltLayoutNode>,
    },
}

impl RebuiltLayoutNode {
    fn layout_node_id(&self) -> LayoutNodeId {
        match self {
            Self::Leaf { layout_node_id, .. } | Self::Split { layout_node_id, .. } => {
                *layout_node_id
            }
        }
    }
}

fn rebuild_layout_node(
    layout: &SessionLayoutSnapshot,
    layout_node_id: LayoutNodeId,
    removed_terminal_instance_id: TerminalInstanceId,
) -> Option<RebuiltLayoutNode> {
    let node = layout.node(layout_node_id)?;
    match node.kind {
        SessionLayoutNodeKind::Leaf { terminal_instance_id } => {
            if terminal_instance_id == removed_terminal_instance_id {
                None
            } else {
                Some(RebuiltLayoutNode::Leaf {
                    layout_node_id,
                    leaf: layout.leaf(terminal_instance_id)?.clone(),
                })
            }
        }
        SessionLayoutNodeKind::Split {
            axis,
            first,
            second,
        } => {
            let first = rebuild_layout_node(layout, first, removed_terminal_instance_id);
            let second = rebuild_layout_node(layout, second, removed_terminal_instance_id);
            match (first, second) {
                (Some(first), Some(second)) => Some(RebuiltLayoutNode::Split {
                    layout_node_id,
                    axis,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Some(node), None) | (None, Some(node)) => Some(node),
                (None, None) => None,
            }
        }
    }
}

fn collect_layout_node(
    node: &RebuiltLayoutNode,
    nodes: &mut Vec<SessionLayoutNodeSnapshot>,
    leaves: &mut Vec<SessionTerminalInstanceSnapshot>,
) {
    match node {
        RebuiltLayoutNode::Leaf {
            layout_node_id,
            leaf,
        } => {
            leaves.push(leaf.clone());
            nodes.push(SessionLayoutNodeSnapshot {
                layout_node_id: *layout_node_id,
                kind: SessionLayoutNodeKind::Leaf {
                    terminal_instance_id: leaf.terminal_instance_id,
                },
            });
        }
        RebuiltLayoutNode::Split {
            layout_node_id,
            axis,
            first,
            second,
        } => {
            collect_layout_node(first, nodes, leaves);
            collect_layout_node(second, nodes, leaves);
            nodes.push(SessionLayoutNodeSnapshot {
                layout_node_id: *layout_node_id,
                kind: SessionLayoutNodeKind::Split {
                    axis: *axis,
                    first: first.layout_node_id(),
                    second: second.layout_node_id(),
                },
            });
        }
    }
}

pub fn build_layout_snapshot_from_tree(
    layout_seed: u64,
    active_terminal_instance_id: TerminalInstanceId,
    root: &SessionLayoutTreeNode,
) -> Option<SessionLayoutSnapshot> {
    let mut nodes = Vec::new();
    let mut leaves = Vec::new();
    let mut split_ordinal = 0usize;
    let root_layout_node_id = build_layout_node(
        layout_seed,
        root,
        &mut split_ordinal,
        &mut nodes,
        &mut leaves,
    )?;
    if !leaves.iter().any(|leaf| leaf.terminal_instance_id == active_terminal_instance_id) {
        return None;
    }
    Some(SessionLayoutSnapshot {
        root_layout_node_id,
        active_terminal_instance_id,
        nodes,
        leaves,
    })
}
fn build_layout_node(
    layout_seed: u64,
    node: &SessionLayoutTreeNode,
    split_ordinal: &mut usize,
    nodes: &mut Vec<SessionLayoutNodeSnapshot>,
    leaves: &mut Vec<SessionTerminalInstanceSnapshot>,
) -> Option<LayoutNodeId> {
    match node {
        SessionLayoutTreeNode::Empty => None,
        SessionLayoutTreeNode::TerminalInstance {
            terminal_instance_id,
            title,
        } => {
            let terminal_instance_id = *terminal_instance_id;
            let layout_node_id = leaf_layout_node_id(layout_seed, terminal_instance_id);
            leaves.push(SessionTerminalInstanceSnapshot {
                terminal_instance_id,
                title: title.clone(),
            });
            nodes.push(SessionLayoutNodeSnapshot {
                layout_node_id,
                kind: SessionLayoutNodeKind::Leaf { terminal_instance_id },
            });
            Some(layout_node_id)
        }
        SessionLayoutTreeNode::Split {
            axis,
            first,
            second,
        } => {
            let first = build_layout_node(layout_seed, first, split_ordinal, nodes, leaves)?;
            let second = build_layout_node(layout_seed, second, split_ordinal, nodes, leaves)?;
            let layout_node_id = split_layout_node_id(layout_seed, *split_ordinal);
            *split_ordinal += 1;
            nodes.push(SessionLayoutNodeSnapshot {
                layout_node_id,
                kind: SessionLayoutNodeKind::Split {
                    axis: *axis,
                    first,
                    second,
                },
            });
            Some(layout_node_id)
        }
    }
}

fn leaf_layout_node_id(layout_seed: u64, terminal_instance_id: TerminalInstanceId) -> LayoutNodeId {
    LayoutNodeId::new((layout_seed << 32) | terminal_instance_id.as_u64())
}

fn split_layout_node_id(layout_seed: u64, split_index: usize) -> LayoutNodeId {
    LayoutNodeId::new((layout_seed << 32) | 0x8000_0000 | split_index as u64)
}

#[cfg(test)]
mod tests {
    use super::{
        SessionLayoutNodeKind, SessionLayoutSnapshot, SessionLayoutTreeNode,
        build_layout_snapshot_from_tree,
    };
    use super::{LayoutNodeId, TerminalInstanceId};

    fn terminal_instance(id: u64, title: &str) -> SessionLayoutTreeNode {
        SessionLayoutTreeNode::TerminalInstance {
            terminal_instance_id: TerminalInstanceId::new(id),
            title: Some(title.to_string()),
        }
    }

    fn split(
        axis: super::SessionSplitAxis,
        first: SessionLayoutTreeNode,
        second: SessionLayoutTreeNode,
    ) -> SessionLayoutTreeNode {
        SessionLayoutTreeNode::Split {
            axis,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    #[test]
    fn single_terminal_instance_layout_uses_stable_ids() {
        let layout =
            SessionLayoutSnapshot::single_terminal_instance(LayoutNodeId::new(11), TerminalInstanceId::new(22), None);

        assert_eq!(layout.root_layout_node_id, LayoutNodeId::new(11));
        assert_eq!(layout.active_terminal_instance_id, TerminalInstanceId::new(22));
        assert_eq!(layout.nodes.len(), 1);
        assert_eq!(
            layout.nodes[0].kind,
            SessionLayoutNodeKind::Leaf {
                terminal_instance_id: TerminalInstanceId::new(22)
            }
        );
        assert_eq!(layout.leaves.len(), 1);
        assert_eq!(layout.leaves[0].terminal_instance_id, TerminalInstanceId::new(22));
        assert!(layout.leaf(TerminalInstanceId::new(22)).is_some());
        assert_eq!(
            layout
                .node(LayoutNodeId::new(11))
                .expect("root node")
                .layout_node_id,
            LayoutNodeId::new(11)
        );
    }

    #[test]
    fn layout_tree_builds_exact_split_layout() {
        let layout_tree = split(
            super::SessionSplitAxis::Vertical,
            terminal_instance(41, "left"),
            split(
                super::SessionSplitAxis::Horizontal,
                terminal_instance(42, "top-right"),
                terminal_instance(43, "bottom-right"),
            ),
        );

        let layout = build_layout_snapshot_from_tree(7, TerminalInstanceId::new(43), &layout_tree)
            .expect("layout from tree");

        assert_eq!(layout.active_terminal_instance_id, TerminalInstanceId::new(43));
        assert_eq!(layout.leaves.len(), 3);
        assert_eq!(layout.nodes.len(), 5);
        match &layout
            .node(layout.root_layout_node_id)
            .expect("root node")
            .kind
        {
            SessionLayoutNodeKind::Split {
                axis,
                first,
                second,
            } => {
                assert_eq!(*axis, super::SessionSplitAxis::Vertical);
                assert!(layout.node(*first).is_some());
                assert!(layout.node(*second).is_some());
            }
            other => panic!("unexpected root node: {:?}", other),
        }
        assert!(layout.leaf(TerminalInstanceId::new(41)).is_some());
        assert!(layout.leaf(TerminalInstanceId::new(99)).is_none());
        assert!(layout.node(layout.root_layout_node_id).is_some());
        assert!(layout.node(LayoutNodeId::new(999)).is_none());
    }

    #[test]
    fn layout_snapshot_rejects_stale_active_terminal_instance() {
        let layout_tree = split(
            super::SessionSplitAxis::Horizontal,
            terminal_instance(1, "one"),
            terminal_instance(2, "two"),
        );

        assert!(build_layout_snapshot_from_tree(9, TerminalInstanceId::new(99), &layout_tree).is_none());
    }

    #[test]
    fn remove_terminal_instance_collapses_parent_split_and_preserves_valid_tree() {
        let layout_tree = split(
            super::SessionSplitAxis::Vertical,
            terminal_instance(41, "left"),
            split(
                super::SessionSplitAxis::Horizontal,
                terminal_instance(42, "top-right"),
                terminal_instance(43, "bottom-right"),
            ),
        );
        let layout = build_layout_snapshot_from_tree(7, TerminalInstanceId::new(43), &layout_tree)
            .expect("layout from tree");

        let updated = layout.remove_terminal_instance(TerminalInstanceId::new(42)).expect("updated layout");

        assert_eq!(updated.active_terminal_instance_id, TerminalInstanceId::new(43));
        assert_eq!(updated.leaves.len(), 2);
        assert!(updated.leaf(TerminalInstanceId::new(41)).is_some());
        assert!(updated.leaf(TerminalInstanceId::new(43)).is_some());
        assert!(updated.leaf(TerminalInstanceId::new(42)).is_none());
        assert_eq!(updated.nodes.len(), 3);

        match &updated.node(updated.root_layout_node_id).expect("root node").kind {
            SessionLayoutNodeKind::Split { axis, first, second } => {
                assert_eq!(*axis, super::SessionSplitAxis::Vertical);
                assert!(updated.node(*first).is_some());
                assert!(updated.node(*second).is_some());
            }
            other => panic!("unexpected collapsed root: {:?}", other),
        }
    }
}
