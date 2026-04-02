// Workspace layout product model — moved from `desktop_host_runtime::session_engine` (Phase 07).
//
// These types are UI/product model (split views, workspace tree) — NOT execution engine
// internals. They live here so `chatminal-runtime` is the single source of truth for
// workspace state without depending on `desktop_host_runtime::session_engine`.

use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::workspace_ids::{SessionViewId, WorkspaceNodeId};

const DEFAULT_SPLIT_RATIO: u16 = 500;
const MIN_SPLIT_RATIO: u16 = 100;
const MAX_SPLIT_RATIO: u16 = 900;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceSplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionViewSnapshot {
    pub view_id: SessionViewId,
    pub session_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceLayoutNodeKind {
    View {
        view_id: SessionViewId,
    },
    Split {
        axis: WorkspaceSplitAxis,
        first: WorkspaceNodeId,
        second: WorkspaceNodeId,
        ratio: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceLayoutNodeSnapshot {
    pub node_id: WorkspaceNodeId,
    pub kind: WorkspaceLayoutNodeKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceLayoutState {
    pub root_node_id: WorkspaceNodeId,
    pub active_view_id: SessionViewId,
    pub nodes: Vec<WorkspaceLayoutNodeSnapshot>,
    pub views: Vec<SessionViewSnapshot>,
    next_node_id: u64,
    next_view_id: u64,
}

impl WorkspaceLayoutState {
    pub fn new_single(session_id: impl Into<String>) -> Self {
        let view_id = SessionViewId::new(1);
        let root_node_id = WorkspaceNodeId::new(1);
        Self {
            root_node_id,
            active_view_id: view_id,
            nodes: vec![WorkspaceLayoutNodeSnapshot {
                node_id: root_node_id,
                kind: WorkspaceLayoutNodeKind::View { view_id },
            }],
            views: vec![SessionViewSnapshot {
                view_id,
                session_id: session_id.into(),
            }],
            next_node_id: 2,
            next_view_id: 2,
        }
    }

    pub fn split_view(
        &mut self,
        view_id: SessionViewId,
        axis: WorkspaceSplitAxis,
        session_id: impl Into<String>,
    ) -> Option<SessionViewId> {
        let target_index = self
            .nodes
            .iter()
            .position(|node| matches!(node.kind, WorkspaceLayoutNodeKind::View { view_id: id } if id == view_id))?;
        let existing_child = WorkspaceNodeId::new(self.alloc_node_id());
        let new_child = WorkspaceNodeId::new(self.alloc_node_id());
        let new_view_id = SessionViewId::new(self.alloc_view_id());

        self.nodes[target_index].kind = WorkspaceLayoutNodeKind::Split {
            axis,
            first: existing_child,
            second: new_child,
            ratio: DEFAULT_SPLIT_RATIO,
        };
        self.nodes.push(WorkspaceLayoutNodeSnapshot {
            node_id: existing_child,
            kind: WorkspaceLayoutNodeKind::View { view_id },
        });
        self.nodes.push(WorkspaceLayoutNodeSnapshot {
            node_id: new_child,
            kind: WorkspaceLayoutNodeKind::View {
                view_id: new_view_id,
            },
        });
        self.views.push(SessionViewSnapshot {
            view_id: new_view_id,
            session_id: session_id.into(),
        });
        self.active_view_id = new_view_id;
        Some(new_view_id)
    }

    pub fn attach_session(
        &mut self,
        view_id: SessionViewId,
        session_id: impl Into<String>,
    ) -> bool {
        let Some(view) = self.views.iter_mut().find(|view| view.view_id == view_id) else {
            return false;
        };
        view.session_id = session_id.into();
        true
    }

    pub fn focus_view(&mut self, view_id: SessionViewId) -> bool {
        if self.views.iter().any(|view| view.view_id == view_id) {
            self.active_view_id = view_id;
            true
        } else {
            false
        }
    }

    pub fn close_view(&mut self, view_id: SessionViewId) -> bool {
        if self.views.len() <= 1 || !self.contains_view(view_id) {
            return false;
        }
        let rebuilt = rebuild_node(self, self.root_node_id, view_id);
        let Some(rebuilt) = rebuilt else {
            return false;
        };
        let mut nodes = Vec::new();
        let mut views = Vec::new();
        collect_rebuilt(&rebuilt, &mut nodes, &mut views);
        self.root_node_id = rebuilt.node_id();
        self.active_view_id = if views.iter().any(|view| view.view_id == self.active_view_id) {
            self.active_view_id
        } else {
            views[0].view_id
        };
        self.nodes = nodes;
        self.views = views;
        true
    }

    pub fn resize_split(&mut self, node_id: WorkspaceNodeId, ratio: u16) -> bool {
        let Some(node) = self.nodes.iter_mut().find(|node| node.node_id == node_id) else {
            return false;
        };
        let WorkspaceLayoutNodeKind::Split {
            ratio: current_ratio,
            ..
        } = &mut node.kind
        else {
            return false;
        };
        *current_ratio = clamp_split_ratio(ratio);
        true
    }

    pub fn contains_view(&self, view_id: SessionViewId) -> bool {
        self.views.iter().any(|view| view.view_id == view_id)
    }

    pub fn view(&self, view_id: SessionViewId) -> Option<&SessionViewSnapshot> {
        self.views.iter().find(|view| view.view_id == view_id)
    }

    pub fn view_for_session(&self, session_id: &str) -> Option<SessionViewId> {
        self.views
            .iter()
            .find(|view| view.session_id == session_id)
            .map(|view| view.view_id)
    }

    pub fn ensure_session_view(&mut self, session_id: impl Into<String>) -> SessionViewId {
        let session_id = session_id.into();
        if let Some(view_id) = self.view_for_session(&session_id) {
            self.active_view_id = view_id;
            return view_id;
        }

        self.split_view(
            self.active_view_id,
            WorkspaceSplitAxis::Vertical,
            session_id,
        )
        .expect("active view must exist before ensuring session view")
    }

    pub fn grouped_sessions(session_ids: &[String]) -> Option<Self> {
        let (anchor_session_id, rest) = session_ids.split_first()?;
        let mut layout = Self::new_single(anchor_session_id.clone());
        let anchor_view_id = layout.active_view_id;
        let mut pending = VecDeque::from([(anchor_view_id, 0usize)]);

        for session_id in rest {
            let (target_view_id, depth) = pending.pop_front()?;
            let axis = if depth % 2 == 0 {
                WorkspaceSplitAxis::Vertical
            } else {
                WorkspaceSplitAxis::Horizontal
            };
            let new_view_id = layout.split_view(target_view_id, axis, session_id.clone())?;
            pending.push_back((target_view_id, depth + 1));
            pending.push_back((new_view_id, depth + 1));
        }

        let _ = layout.focus_view(anchor_view_id);
        Some(layout)
    }

    pub fn retain_sessions<'a, I>(&mut self, valid_session_ids: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        let valid_session_ids = valid_session_ids
            .into_iter()
            .map(str::to_owned)
            .collect::<HashSet<_>>();
        if !self
            .views
            .iter()
            .any(|view| valid_session_ids.contains(&view.session_id))
        {
            return false;
        }

        let removed_view_ids = self
            .views
            .iter()
            .filter(|view| !valid_session_ids.contains(&view.session_id))
            .map(|view| view.view_id)
            .collect::<Vec<_>>();

        for view_id in removed_view_ids {
            if self.views.len() <= 1 {
                break;
            }
            let _ = self.close_view(view_id);
        }

        if !self.contains_view(self.active_view_id) {
            if let Some(view_id) = self
                .views
                .iter()
                .find(|view| valid_session_ids.contains(&view.session_id))
                .map(|view| view.view_id)
            {
                self.active_view_id = view_id;
            }
        }

        true
    }

    fn alloc_node_id(&mut self) -> u64 {
        let next = self.next_node_id;
        self.next_node_id += 1;
        next
    }

    fn alloc_view_id(&mut self) -> u64 {
        let next = self.next_view_id;
        self.next_view_id += 1;
        next
    }
}

pub(crate) fn clamp_split_ratio(ratio: u16) -> u16 {
    ratio.clamp(MIN_SPLIT_RATIO, MAX_SPLIT_RATIO)
}

// --- rebuild helpers (inlined from workspace_layout_rebuild) ---

#[derive(Clone)]
enum RebuiltNode {
    View(SessionViewSnapshot, WorkspaceNodeId),
    Split(
        WorkspaceLayoutNodeSnapshot,
        Box<RebuiltNode>,
        Box<RebuiltNode>,
    ),
}

impl RebuiltNode {
    fn node_id(&self) -> WorkspaceNodeId {
        match self {
            Self::View(_, node_id) => *node_id,
            Self::Split(node, _, _) => node.node_id,
        }
    }
}

fn rebuild_node(
    layout: &WorkspaceLayoutState,
    node_id: WorkspaceNodeId,
    removed_view_id: SessionViewId,
) -> Option<RebuiltNode> {
    let node = layout.nodes.iter().find(|node| node.node_id == node_id)?;
    match &node.kind {
        WorkspaceLayoutNodeKind::View { view_id } => {
            if *view_id == removed_view_id {
                None
            } else {
                Some(RebuiltNode::View(layout.view(*view_id)?.clone(), node_id))
            }
        }
        WorkspaceLayoutNodeKind::Split {
            axis,
            first,
            second,
            ratio,
        } => {
            let first = rebuild_node(layout, *first, removed_view_id);
            let second = rebuild_node(layout, *second, removed_view_id);
            match (first, second) {
                (Some(first), Some(second)) => Some(RebuiltNode::Split(
                    WorkspaceLayoutNodeSnapshot {
                        node_id,
                        kind: WorkspaceLayoutNodeKind::Split {
                            axis: *axis,
                            first: first.node_id(),
                            second: second.node_id(),
                            ratio: *ratio,
                        },
                    },
                    Box::new(first),
                    Box::new(second),
                )),
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            }
        }
    }
}

fn collect_rebuilt(
    rebuilt: &RebuiltNode,
    nodes: &mut Vec<WorkspaceLayoutNodeSnapshot>,
    views: &mut Vec<SessionViewSnapshot>,
) {
    match rebuilt {
        RebuiltNode::View(view, node_id) => {
            nodes.push(WorkspaceLayoutNodeSnapshot {
                node_id: *node_id,
                kind: WorkspaceLayoutNodeKind::View {
                    view_id: view.view_id,
                },
            });
            views.push(view.clone());
        }
        RebuiltNode::Split(node, first, second) => {
            nodes.push(node.clone());
            collect_rebuilt(first, nodes, views);
            collect_rebuilt(second, nodes, views);
        }
    }
}

// --- registry (inlined from workspace_layout_registry) ---

use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceLayoutRegistry {
    layouts: HashMap<String, WorkspaceLayoutState>,
}

impl WorkspaceLayoutRegistry {
    pub fn ensure_layout(
        &mut self,
        workspace_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> WorkspaceLayoutState {
        let workspace_id = workspace_id.into();
        let session_id = session_id.into();
        self.layouts
            .entry(workspace_id)
            .or_insert_with(|| WorkspaceLayoutState::new_single(session_id))
            .clone()
    }

    pub fn layout(&self, workspace_id: &str) -> Option<&WorkspaceLayoutState> {
        self.layouts.get(workspace_id)
    }

    pub fn replace_layout(
        &mut self,
        workspace_id: impl Into<String>,
        layout: WorkspaceLayoutState,
    ) -> WorkspaceLayoutState {
        let workspace_id = workspace_id.into();
        self.layouts.insert(workspace_id, layout.clone());
        layout
    }

    pub fn remove_layout(&mut self, workspace_id: &str) -> Option<WorkspaceLayoutState> {
        self.layouts.remove(workspace_id)
    }

    pub fn ensure_session_view(
        &mut self,
        workspace_id: &str,
        session_id: impl Into<String>,
    ) -> Option<WorkspaceLayoutState> {
        let layout = self.layouts.get_mut(workspace_id)?;
        layout.ensure_session_view(session_id);
        Some(layout.clone())
    }

    pub fn split_view(
        &mut self,
        workspace_id: &str,
        view_id: SessionViewId,
        axis: WorkspaceSplitAxis,
        session_id: impl Into<String>,
    ) -> Option<WorkspaceLayoutState> {
        let layout = self.layouts.get_mut(workspace_id)?;
        layout.split_view(view_id, axis, session_id)?;
        Some(layout.clone())
    }

    pub fn attach_session(
        &mut self,
        workspace_id: &str,
        view_id: SessionViewId,
        session_id: impl Into<String>,
    ) -> Option<WorkspaceLayoutState> {
        let layout = self.layouts.get_mut(workspace_id)?;
        if !layout.attach_session(view_id, session_id) {
            return None;
        }
        Some(layout.clone())
    }

    pub fn focus_view(
        &mut self,
        workspace_id: &str,
        view_id: SessionViewId,
    ) -> Option<WorkspaceLayoutState> {
        let layout = self.layouts.get_mut(workspace_id)?;
        if !layout.focus_view(view_id) {
            return None;
        }
        Some(layout.clone())
    }

    pub fn close_view(
        &mut self,
        workspace_id: &str,
        view_id: SessionViewId,
    ) -> Option<WorkspaceLayoutState> {
        let layout = self.layouts.get_mut(workspace_id)?;
        if !layout.close_view(view_id) {
            return None;
        }
        Some(layout.clone())
    }

    pub fn resize_split(
        &mut self,
        workspace_id: &str,
        node_id: WorkspaceNodeId,
        ratio: u16,
    ) -> Option<WorkspaceLayoutState> {
        let layout = self.layouts.get_mut(workspace_id)?;
        if !layout.resize_split(node_id, ratio) {
            return None;
        }
        Some(layout.clone())
    }

    pub fn view_id_for_session(
        &self,
        workspace_id: &str,
        session_id: &str,
    ) -> Option<SessionViewId> {
        self.layouts.get(workspace_id)?.view_for_session(session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceLayoutRegistry, WorkspaceLayoutState, WorkspaceSplitAxis};
    use crate::workspace_ids::{SessionViewId, WorkspaceNodeId};

    #[test]
    fn new_single_creates_one_view_layout() {
        let layout = WorkspaceLayoutState::new_single("session-a");
        assert_eq!(layout.views.len(), 1);
        assert_eq!(layout.views[0].session_id, "session-a");
    }

    #[test]
    fn ensure_layout_is_idempotent_for_existing_workspace() {
        let mut registry = WorkspaceLayoutRegistry::default();
        let first = registry.ensure_layout("main", "session-a");
        let second = registry.ensure_layout("main", "session-b");

        assert_eq!(first, second);
        assert_eq!(second.views[0].session_id, "session-a");
    }

    #[test]
    fn registry_mutations_return_updated_layout_snapshots() {
        let mut registry = WorkspaceLayoutRegistry::default();
        registry.ensure_layout("main", "session-a");

        let split = registry
            .split_view(
                "main",
                SessionViewId::new(1),
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("split");
        assert_eq!(split.views.len(), 2);

        let rebound = registry
            .attach_session("main", SessionViewId::new(2), "session-c")
            .expect("attach");
        assert_eq!(
            rebound.view(SessionViewId::new(2)).unwrap().session_id,
            "session-c"
        );

        let focused = registry
            .focus_view("main", SessionViewId::new(1))
            .expect("focus");
        assert_eq!(focused.active_view_id, SessionViewId::new(1));
    }

    #[test]
    fn ensure_session_view_tracks_session_identity() {
        let mut registry = WorkspaceLayoutRegistry::default();
        registry.ensure_layout("main", "session-a");

        let layout = registry
            .ensure_session_view("main", "session-b")
            .expect("ensure session view");

        assert_eq!(layout.views.len(), 2);
        assert_eq!(
            registry.view_id_for_session("main", "session-b"),
            Some(SessionViewId::new(2))
        );
    }

    #[test]
    fn resize_split_updates_layout_snapshot() {
        let mut registry = WorkspaceLayoutRegistry::default();
        registry.ensure_layout("main", "session-a");
        registry
            .split_view(
                "main",
                SessionViewId::new(1),
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("split");

        let resized = registry
            .resize_split("main", WorkspaceNodeId::new(1), 700)
            .expect("resize");

        assert!(matches!(
            resized.nodes[0].kind,
            crate::WorkspaceLayoutNodeKind::Split { ratio: 700, .. }
        ));
    }

    #[test]
    fn grouped_sessions_builds_balanced_join_layout() {
        let sessions = vec![
            "session-a".to_string(),
            "session-b".to_string(),
            "session-c".to_string(),
            "session-d".to_string(),
        ];
        let layout = WorkspaceLayoutState::grouped_sessions(&sessions).expect("grouped layout");

        assert_eq!(layout.views.len(), 4);
        assert_eq!(
            layout
                .view(layout.active_view_id)
                .expect("active view")
                .session_id,
            "session-a"
        );
        assert!(layout.view_for_session("session-b").is_some());
        assert!(layout.view_for_session("session-c").is_some());
        assert!(layout.view_for_session("session-d").is_some());
    }
}
