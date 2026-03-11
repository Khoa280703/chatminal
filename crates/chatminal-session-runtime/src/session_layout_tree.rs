use mux::tab::{PaneNode, SplitDirection, TabId};

use crate::{LayoutNodeId, LeafId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionSplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionLeafSnapshot {
    pub leaf_id: LeafId,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionLayoutNodeKind {
    Leaf {
        leaf_id: LeafId,
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
    pub active_leaf_id: LeafId,
    pub nodes: Vec<SessionLayoutNodeSnapshot>,
    pub leaves: Vec<SessionLeafSnapshot>,
}

impl SessionLayoutSnapshot {
    pub fn single_leaf(
        root_layout_node_id: LayoutNodeId,
        active_leaf_id: LeafId,
        title: Option<String>,
    ) -> Self {
        Self {
            root_layout_node_id,
            active_leaf_id,
            nodes: vec![SessionLayoutNodeSnapshot {
                layout_node_id: root_layout_node_id,
                kind: SessionLayoutNodeKind::Leaf {
                    leaf_id: active_leaf_id,
                },
            }],
            leaves: vec![SessionLeafSnapshot {
                leaf_id: active_leaf_id,
                title,
            }],
        }
    }

    pub fn leaf(&self, leaf_id: LeafId) -> Option<&SessionLeafSnapshot> {
        self.leaves.iter().find(|leaf| leaf.leaf_id == leaf_id)
    }

    pub fn active_leaf(&self) -> Option<&SessionLeafSnapshot> {
        self.leaf(self.active_leaf_id)
    }

    pub fn node(&self, layout_node_id: LayoutNodeId) -> Option<&SessionLayoutNodeSnapshot> {
        self.nodes
            .iter()
            .find(|node| node.layout_node_id == layout_node_id)
    }

    pub fn contains_leaf(&self, leaf_id: LeafId) -> bool {
        self.leaf(leaf_id).is_some()
    }

    pub fn resolve_leaf_layout_node(&self, leaf_id: LeafId) -> Option<LayoutNodeId> {
        self.nodes.iter().find_map(|node| match node.kind {
            SessionLayoutNodeKind::Leaf {
                leaf_id: node_leaf_id,
            } if node_leaf_id == leaf_id => Some(node.layout_node_id),
            _ => None,
        })
    }

    pub fn child_layout_nodes(
        &self,
        layout_node_id: LayoutNodeId,
    ) -> Option<(LayoutNodeId, LayoutNodeId)> {
        match self.node(layout_node_id)?.kind {
            SessionLayoutNodeKind::Split { first, second, .. } => Some((first, second)),
            SessionLayoutNodeKind::Leaf { .. } => None,
        }
    }

    pub fn is_stale_leaf(&self, leaf_id: LeafId) -> bool {
        !self.contains_leaf(leaf_id)
    }

    pub fn is_stale_layout_node(&self, layout_node_id: LayoutNodeId) -> bool {
        self.node(layout_node_id).is_none()
    }
}

pub fn build_layout_snapshot_from_engine(
    host_surface_id: TabId,
    active_host_leaf_id: u64,
    pane_tree: &PaneNode,
) -> Option<SessionLayoutSnapshot> {
    let active_leaf_id = LeafId::new(active_host_leaf_id);
    let mut nodes = Vec::new();
    let mut leaves = Vec::new();
    let mut split_ordinal = 0usize;
    let root_layout_node_id = build_layout_node(
        host_surface_id,
        pane_tree,
        &mut split_ordinal,
        &mut nodes,
        &mut leaves,
    )?;
    if !leaves.iter().any(|leaf| leaf.leaf_id == active_leaf_id) {
        return None;
    }
    Some(SessionLayoutSnapshot {
        root_layout_node_id,
        active_leaf_id,
        nodes,
        leaves,
    })
}
fn build_layout_node(
    host_surface_id: TabId,
    node: &PaneNode,
    split_ordinal: &mut usize,
    nodes: &mut Vec<SessionLayoutNodeSnapshot>,
    leaves: &mut Vec<SessionLeafSnapshot>,
) -> Option<LayoutNodeId> {
    match node {
        PaneNode::Empty => None,
        PaneNode::Leaf(entry) => {
            let leaf_id = LeafId::new(entry.pane_id as u64);
            let layout_node_id = leaf_layout_node_id_for_host_surface(host_surface_id, leaf_id);
            leaves.push(SessionLeafSnapshot {
                leaf_id,
                title: Some(entry.title.clone()),
            });
            nodes.push(SessionLayoutNodeSnapshot {
                layout_node_id,
                kind: SessionLayoutNodeKind::Leaf { leaf_id },
            });
            Some(layout_node_id)
        }
        PaneNode::Split { left, right, node } => {
            let first = build_layout_node(host_surface_id, left, split_ordinal, nodes, leaves)?;
            let second = build_layout_node(host_surface_id, right, split_ordinal, nodes, leaves)?;
            let layout_node_id =
                split_layout_node_id_for_host_surface(host_surface_id, *split_ordinal);
            *split_ordinal += 1;
            nodes.push(SessionLayoutNodeSnapshot {
                layout_node_id,
                kind: SessionLayoutNodeKind::Split {
                    axis: split_axis(node.direction),
                    first,
                    second,
                },
            });
            Some(layout_node_id)
        }
    }
}

fn split_axis(direction: SplitDirection) -> SessionSplitAxis {
    match direction {
        SplitDirection::Horizontal => SessionSplitAxis::Horizontal,
        SplitDirection::Vertical => SessionSplitAxis::Vertical,
    }
}

fn leaf_layout_node_id_for_host_surface(host_surface_id: TabId, leaf_id: LeafId) -> LayoutNodeId {
    LayoutNodeId::new(((host_surface_id as u64) << 32) | leaf_id.as_u64())
}

fn split_layout_node_id_for_host_surface(
    host_surface_id: TabId,
    split_index: usize,
) -> LayoutNodeId {
    LayoutNodeId::new(((host_surface_id as u64) << 32) | 0x8000_0000 | split_index as u64)
}

#[cfg(test)]
mod tests {
    use engine_term::TerminalSize;
    use mux::renderable::StableCursorPosition;
    use mux::tab::{PaneEntry, PaneNode, SplitDirection, SplitDirectionAndSize};

    use super::{SessionLayoutNodeKind, SessionLayoutSnapshot, build_layout_snapshot_from_engine};
    use crate::{LayoutNodeId, LeafId};

    fn leaf(host_leaf_id: usize, title: &str) -> PaneNode {
        PaneNode::Leaf(PaneEntry {
            window_id: 1,
            tab_id: 7,
            pane_id: host_leaf_id,
            title: title.to_string(),
            size: TerminalSize::default(),
            working_dir: None,
            is_active_pane: false,
            is_zoomed_pane: false,
            workspace: "default".to_string(),
            cursor_pos: StableCursorPosition::default(),
            physical_top: 0,
            top_row: 0,
            left_col: 0,
            tty_name: None,
        })
    }

    fn split(direction: SplitDirection, left: PaneNode, right: PaneNode) -> PaneNode {
        PaneNode::Split {
            left: Box::new(left),
            right: Box::new(right),
            node: SplitDirectionAndSize {
                direction,
                first: TerminalSize::default(),
                second: TerminalSize::default(),
            },
        }
    }

    #[test]
    fn single_leaf_layout_uses_stable_ids() {
        let layout =
            SessionLayoutSnapshot::single_leaf(LayoutNodeId::new(11), LeafId::new(22), None);

        assert_eq!(layout.root_layout_node_id, LayoutNodeId::new(11));
        assert_eq!(layout.active_leaf_id, LeafId::new(22));
        assert_eq!(layout.nodes.len(), 1);
        assert_eq!(
            layout.nodes[0].kind,
            SessionLayoutNodeKind::Leaf {
                leaf_id: LeafId::new(22)
            }
        );
        assert_eq!(layout.leaves.len(), 1);
        assert_eq!(layout.leaves[0].leaf_id, LeafId::new(22));
        assert!(layout.contains_leaf(LeafId::new(22)));
        assert_eq!(
            layout.active_leaf().expect("active leaf").leaf_id,
            LeafId::new(22)
        );
        assert_eq!(
            layout
                .node(LayoutNodeId::new(11))
                .expect("root node")
                .layout_node_id,
            LayoutNodeId::new(11)
        );
    }

    #[test]
    fn pane_tree_builds_exact_split_layout() {
        let pane_tree = split(
            SplitDirection::Vertical,
            leaf(41, "left"),
            split(
                SplitDirection::Horizontal,
                leaf(42, "top-right"),
                leaf(43, "bottom-right"),
            ),
        );

        let layout =
            build_layout_snapshot_from_engine(7, 43, &pane_tree).expect("layout from pane tree");

        assert_eq!(layout.active_leaf_id, LeafId::new(43));
        assert_eq!(layout.leaves.len(), 3);
        assert_eq!(layout.nodes.len(), 5);
        assert_eq!(
            layout
                .resolve_leaf_layout_node(LeafId::new(42))
                .expect("leaf node"),
            LayoutNodeId::new((7u64 << 32) | 42)
        );
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
                assert_eq!(
                    layout.child_layout_nodes(layout.root_layout_node_id),
                    Some((*first, *second))
                );
            }
            other => panic!("unexpected root node: {other:?}"),
        }
        assert!(!layout.is_stale_leaf(LeafId::new(41)));
        assert!(layout.is_stale_leaf(LeafId::new(99)));
        assert!(!layout.is_stale_layout_node(layout.root_layout_node_id));
        assert!(layout.is_stale_layout_node(LayoutNodeId::new(999)));
    }

    #[test]
    fn layout_snapshot_rejects_stale_active_leaf() {
        let pane_tree = split(SplitDirection::Horizontal, leaf(1, "one"), leaf(2, "two"));

        assert!(build_layout_snapshot_from_engine(9, 99, &pane_tree).is_none());
    }
}
