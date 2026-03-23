#[cfg(test)]
use crate::chatminal_runtime::SessionEngineShared;
use crate::chatminal_runtime::{
    SessionViewId, WorkspaceLayoutState, WorkspaceNodeId, WorkspaceSplitAxis,
};
#[cfg(test)]
use std::sync::Arc;

pub const DEFAULT_LAYOUT_WORKSPACE_ID: &str = "desktop-main";

pub fn restore_persisted_layout(workspace_id: &str) -> Option<WorkspaceLayoutState> {
    match crate::chatminal_runtime::workspace_layout_restore_persisted(workspace_id) {
        Ok(layout) => layout,
        Err(err) => {
            log::error!("failed to restore persisted workspace layout: {err}");
            None
        }
    }
}

#[cfg(test)]
fn persist_layout_for_test(layout: &WorkspaceLayoutState) {
    if let Err(err) = crate::chatminal_runtime::store_runtime_layout(layout) {
        log::error!("failed to persist workspace layout: {err}");
    }
}

#[cfg(test)]
fn clear_persisted_layout_for_test() {
    if let Err(err) = crate::chatminal_runtime::clear_runtime_layout_state() {
        log::error!("failed to clear persisted workspace layout: {err}");
    }
}

pub fn clear_layout(workspace_id: &str) {
    if let Err(err) = crate::chatminal_runtime::workspace_layout_remove(workspace_id) {
        log::error!("failed to clear workspace layout: {err}");
    }
}

#[derive(Clone, Debug)]
pub struct DesktopWorkspaceLayoutStore {
    workspace_id: String,
    #[cfg(test)]
    shared: Option<Arc<SessionEngineShared>>,
}

impl DesktopWorkspaceLayoutStore {
    pub fn new(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            #[cfg(test)]
            shared: None,
        }
    }

    #[cfg(test)]
    pub fn with_shared(shared: Arc<SessionEngineShared>, workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            shared: Some(shared),
        }
    }

    pub fn ensure_for_session(&self, session_id: &str) -> WorkspaceLayoutState {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            let layout = shared
                .workspace_layouts()
                .lock()
                .unwrap()
                .ensure_layout(self.workspace_id.clone(), session_id.to_string());
            persist_layout_for_test(&layout);
            return layout;
        }

        match crate::chatminal_runtime::workspace_layout_ensure_session(
            &self.workspace_id,
            session_id,
        ) {
            Ok(layout) => layout,
            Err(err) => {
                log::error!("failed to ensure workspace layout session: {err}");
                WorkspaceLayoutState::new_single(session_id.to_string())
            }
        }
    }

    pub fn snapshot(&self) -> Option<WorkspaceLayoutState> {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            return shared
                .workspace_layouts()
                .lock()
                .unwrap()
                .layout(&self.workspace_id)
                .cloned();
        }

        match crate::chatminal_runtime::workspace_layout_snapshot(&self.workspace_id) {
            Ok(layout) => layout,
            Err(err) => {
                log::error!("failed to snapshot workspace layout: {err}");
                None
            }
        }
    }

    pub fn snapshot_or_restore(&self) -> Option<WorkspaceLayoutState> {
        self.snapshot()
            .or_else(|| restore_persisted_layout(&self.workspace_id))
    }

    pub fn split_view(
        &self,
        view_id: SessionViewId,
        axis: WorkspaceSplitAxis,
        session_id: &str,
    ) -> Option<WorkspaceLayoutState> {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            let layout = shared.workspace_layouts().lock().unwrap().split_view(
                &self.workspace_id,
                view_id,
                axis,
                session_id.to_string(),
            );
            if let Some(layout) = layout.as_ref() {
                persist_layout_for_test(layout);
            }
            return layout;
        }

        match crate::chatminal_runtime::workspace_layout_split_view(
            &self.workspace_id,
            view_id,
            axis,
            session_id,
        ) {
            Ok(layout) => layout,
            Err(err) => {
                log::error!("failed to split workspace layout view: {err}");
                None
            }
        }
    }

    pub fn attach_session(
        &self,
        view_id: SessionViewId,
        session_id: &str,
    ) -> Option<WorkspaceLayoutState> {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            let layout = shared.workspace_layouts().lock().unwrap().attach_session(
                &self.workspace_id,
                view_id,
                session_id.to_string(),
            );
            if let Some(layout) = layout.as_ref() {
                persist_layout_for_test(layout);
            }
            return layout;
        }

        match crate::chatminal_runtime::workspace_layout_attach_session(
            &self.workspace_id,
            view_id,
            session_id,
        ) {
            Ok(layout) => layout,
            Err(err) => {
                log::error!("failed to attach session into workspace layout: {err}");
                None
            }
        }
    }

    pub fn focus_view(&self, view_id: SessionViewId) -> Option<WorkspaceLayoutState> {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            let layout = shared
                .workspace_layouts()
                .lock()
                .unwrap()
                .focus_view(&self.workspace_id, view_id);
            if let Some(layout) = layout.as_ref() {
                persist_layout_for_test(layout);
            }
            return layout;
        }

        match crate::chatminal_runtime::workspace_layout_focus_view(&self.workspace_id, view_id) {
            Ok(layout) => layout,
            Err(err) => {
                log::error!("failed to focus workspace layout view: {err}");
                None
            }
        }
    }

    pub fn close_view(&self, view_id: SessionViewId) -> Option<WorkspaceLayoutState> {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            let layout = shared
                .workspace_layouts()
                .lock()
                .unwrap()
                .close_view(&self.workspace_id, view_id);
            if let Some(layout) = layout.as_ref() {
                persist_layout_for_test(layout);
            } else if self.snapshot().is_none() {
                clear_persisted_layout_for_test();
            }
            return layout;
        }

        match crate::chatminal_runtime::workspace_layout_close_view(&self.workspace_id, view_id) {
            Ok(layout) => layout,
            Err(err) => {
                log::error!("failed to close workspace layout view: {err}");
                None
            }
        }
    }

    pub fn resize_split(
        &self,
        node_id: WorkspaceNodeId,
        ratio: u16,
    ) -> Option<WorkspaceLayoutState> {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            let layout = shared.workspace_layouts().lock().unwrap().resize_split(
                &self.workspace_id,
                node_id,
                ratio,
            );
            if let Some(layout) = layout.as_ref() {
                persist_layout_for_test(layout);
            }
            return layout;
        }

        match crate::chatminal_runtime::workspace_layout_resize_split(
            &self.workspace_id,
            node_id,
            ratio,
        ) {
            Ok(layout) => layout,
            Err(err) => {
                log::error!("failed to resize workspace split: {err}");
                None
            }
        }
    }

    pub fn view_id_for_session(&self, session_id: &str) -> Option<SessionViewId> {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            return shared
                .workspace_layouts()
                .lock()
                .unwrap()
                .view_id_for_session(&self.workspace_id, session_id);
        }

        match crate::chatminal_runtime::workspace_layout_view_id_for_session(
            &self.workspace_id,
            session_id,
        ) {
            Ok(view_id) => view_id,
            Err(err) => {
                log::error!("failed to resolve workspace view for session: {err}");
                None
            }
        }
    }

    pub fn replace_layout(&self, layout: WorkspaceLayoutState) -> WorkspaceLayoutState {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            let layout = shared
                .workspace_layouts()
                .lock()
                .unwrap()
                .replace_layout(self.workspace_id.clone(), layout);
            persist_layout_for_test(&layout);
            return layout;
        }

        let fallback = layout.clone();
        match crate::chatminal_runtime::workspace_layout_replace(&self.workspace_id, layout) {
            Ok(layout) => layout,
            Err(err) => {
                log::error!("failed to replace workspace layout: {err}");
                fallback
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::chatminal_runtime::{
        SessionEngineShared, SessionViewId, WorkspaceLayoutNodeKind, WorkspaceNodeId,
        WorkspaceSplitAxis,
    };
    use crate::desktop_host_runtime::session_engine::SessionCoreState;

    use super::{DesktopWorkspaceLayoutStore, DEFAULT_LAYOUT_WORKSPACE_ID};

    #[test]
    fn desktop_store_reads_and_mutates_workspace_layout() {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let store = DesktopWorkspaceLayoutStore::with_shared(shared, DEFAULT_LAYOUT_WORKSPACE_ID);

        let initial = store.ensure_for_session("session-a");
        assert_eq!(initial.views.len(), 1);

        let split = store
            .split_view(
                SessionViewId::new(1),
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("split");
        assert_eq!(split.views.len(), 2);

        let focused = store.focus_view(SessionViewId::new(1)).expect("focus");
        assert_eq!(focused.active_view_id, SessionViewId::new(1));

        let resized = store
            .resize_split(WorkspaceNodeId::new(1), 700)
            .expect("resize");
        assert!(matches!(
            resized.nodes[0].kind,
            WorkspaceLayoutNodeKind::Split { ratio: 700, .. }
        ));
    }
}
