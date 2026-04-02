#[cfg(test)]
use crate::chatminal_runtime::SessionEngineShared;
use crate::chatminal_runtime::{
    SessionViewId, WorkspaceLayoutState, WorkspaceNodeId, WorkspaceSplitAxis,
};
#[cfg(test)]
use std::sync::Arc;

pub const DEFAULT_LAYOUT_WORKSPACE_ID: &str = "desktop-main";
const PROFILE_LAYOUT_WORKSPACE_PREFIX: &str = "desktop-profile:";
const PROFILE_GROUP_LAYOUT_WORKSPACE_PREFIX: &str = "desktop-profile-group:";

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

    fn with_workspace_id(&self, workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            #[cfg(test)]
            shared: self.shared.clone(),
        }
    }

    pub fn profile_workspace_id(profile_id: &str) -> String {
        format!("{PROFILE_LAYOUT_WORKSPACE_PREFIX}{profile_id}")
    }

    pub fn profile_group_workspace_id(profile_id: &str, session_id: &str) -> String {
        format!("{PROFILE_GROUP_LAYOUT_WORKSPACE_PREFIX}{profile_id}:{session_id}")
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

    pub fn clear(&self) {
        #[cfg(test)]
        if let Some(shared) = &self.shared {
            shared
                .workspace_layouts()
                .lock()
                .unwrap()
                .remove_layout(&self.workspace_id);
            return;
        }

        clear_layout(&self.workspace_id);
    }

    pub fn save_as_profile_layout(&self, profile_id: &str) -> Option<WorkspaceLayoutState> {
        let profile_store = self.with_workspace_id(Self::profile_workspace_id(profile_id));
        let layout = self.snapshot_or_restore()?;
        let _ = profile_store.replace_layout(layout.clone());
        if layout.views.len() > 1 {
            for view in &layout.views {
                let group_store = self.with_workspace_id(Self::profile_group_workspace_id(
                    profile_id,
                    &view.session_id,
                ));
                let _ = group_store.replace_layout(layout.clone());
            }
        }
        Some(layout)
    }

    pub fn restore_profile_layout(
        &self,
        profile_id: &str,
        fallback_session_id: Option<&str>,
    ) -> Option<WorkspaceLayoutState> {
        if let Some(session_id) = fallback_session_id {
            if let Some(layout) = self.restore_profile_group_layout(profile_id, session_id) {
                return Some(layout);
            }
        }
        let profile_store = self.with_workspace_id(Self::profile_workspace_id(profile_id));
        let persisted = profile_store.snapshot_or_restore();
        let layout = match (persisted, fallback_session_id) {
            (Some(layout), Some(session_id))
                if !layout
                    .views
                    .iter()
                    .any(|view| view.session_id == session_id) =>
            {
                Some(WorkspaceLayoutState::new_single(session_id.to_string()))
            }
            (Some(layout), _) => Some(layout),
            (None, Some(session_id)) => {
                Some(WorkspaceLayoutState::new_single(session_id.to_string()))
            }
            (None, None) => None,
        };
        match layout {
            Some(layout) => Some(self.replace_layout(layout)),
            None => {
                self.clear();
                None
            }
        }
    }

    pub fn restore_profile_layout_if_contains(
        &self,
        profile_id: &str,
        session_id: &str,
    ) -> Option<WorkspaceLayoutState> {
        if let Some(layout) = self.restore_profile_group_layout(profile_id, session_id) {
            return Some(layout);
        }
        let profile_store = self.with_workspace_id(Self::profile_workspace_id(profile_id));
        let layout = profile_store.snapshot_or_restore()?;
        if !layout
            .views
            .iter()
            .any(|view| view.session_id == session_id)
        {
            return None;
        }
        Some(self.replace_layout(layout))
    }

    pub fn clear_profile_group_layouts<'a, I>(&self, profile_id: &str, session_ids: I)
    where
        I: IntoIterator<Item = &'a str>,
    {
        for session_id in session_ids {
            self.with_workspace_id(Self::profile_group_workspace_id(profile_id, session_id))
                .clear();
        }
    }

    pub fn profile_group_session_ids(&self, profile_id: &str, session_id: &str) -> Vec<String> {
        self.with_workspace_id(Self::profile_group_workspace_id(profile_id, session_id))
            .snapshot_or_restore()
            .filter(|layout| {
                layout.views.len() > 1
                    && layout
                        .views
                        .iter()
                        .any(|view| view.session_id == session_id)
            })
            .map(|layout| {
                layout
                    .views
                    .into_iter()
                    .map(|view| view.session_id)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn restore_profile_group_layout(
        &self,
        profile_id: &str,
        session_id: &str,
    ) -> Option<WorkspaceLayoutState> {
        let group_store =
            self.with_workspace_id(Self::profile_group_workspace_id(profile_id, session_id));
        let layout = group_store.snapshot_or_restore()?;
        if !layout
            .views
            .iter()
            .any(|view| view.session_id == session_id)
        {
            return None;
        }
        Some(self.replace_layout(layout))
    }

    pub fn swap_profile_layout(
        &self,
        previous_profile_id: Option<&str>,
        next_profile_id: &str,
        fallback_session_id: Option<&str>,
    ) -> Option<WorkspaceLayoutState> {
        if previous_profile_id == Some(next_profile_id) {
            return self.snapshot_or_restore().or_else(|| {
                fallback_session_id
                    .map(|session_id| WorkspaceLayoutState::new_single(session_id.to_string()))
            });
        }
        if let Some(previous_profile_id) = previous_profile_id {
            let _ = self.save_as_profile_layout(previous_profile_id);
        }
        self.restore_profile_layout(next_profile_id, fallback_session_id)
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
        SessionEngineShared, SessionViewId, WorkspaceLayoutNodeKind, WorkspaceLayoutState,
        WorkspaceNodeId, WorkspaceSplitAxis,
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

    #[test]
    fn desktop_store_swaps_layouts_per_profile_without_losing_join_state() {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let store = DesktopWorkspaceLayoutStore::with_shared(shared, DEFAULT_LAYOUT_WORKSPACE_ID);

        let initial = store.ensure_for_session("session-a");
        let joined = store
            .split_view(
                initial.active_view_id,
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("join session-b");
        assert_eq!(joined.views.len(), 2);
        let _ = store.save_as_profile_layout("profile-a");

        let work_layout =
            store.replace_layout(WorkspaceLayoutState::new_single("session-c".to_string()));
        assert_eq!(work_layout.views.len(), 1);

        let restored = store
            .swap_profile_layout(Some("profile-b"), "profile-a", Some("session-a"))
            .expect("restore profile-a layout");
        assert_eq!(restored.views.len(), 2);
        assert!(restored
            .views
            .iter()
            .any(|view| view.session_id == "session-a"));
        assert!(restored
            .views
            .iter()
            .any(|view| view.session_id == "session-b"));

        let saved_profile_b = store
            .with_workspace_id(DesktopWorkspaceLayoutStore::profile_workspace_id(
                "profile-b",
            ))
            .snapshot()
            .expect("saved profile-b layout");
        assert_eq!(saved_profile_b.views.len(), 1);
        assert_eq!(saved_profile_b.views[0].session_id, "session-c");
    }

    #[test]
    fn save_as_profile_layout_keeps_session_group_alias_when_active_becomes_single_view() {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let store = DesktopWorkspaceLayoutStore::with_shared(shared, DEFAULT_LAYOUT_WORKSPACE_ID);

        let initial = store.ensure_for_session("session-a");
        let joined = store
            .split_view(
                initial.active_view_id,
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("join session-b");
        assert_eq!(joined.views.len(), 2);
        let _ = store.save_as_profile_layout("profile-a");

        let _ = store.replace_layout(WorkspaceLayoutState::new_single("session-c".to_string()));
        let saved = store
            .save_as_profile_layout("profile-a")
            .expect("save single-view profile layout");
        assert_eq!(saved.views.len(), 1);
        assert_eq!(saved.views[0].session_id, "session-c");

        let profile_layout = store
            .with_workspace_id(DesktopWorkspaceLayoutStore::profile_workspace_id(
                "profile-a",
            ))
            .snapshot()
            .expect("profile-a layout");
        assert_eq!(profile_layout.views.len(), 1);
        assert_eq!(profile_layout.views[0].session_id, "session-c");

        let restored_group = store
            .restore_profile_layout_if_contains("profile-a", "session-a")
            .expect("restore preserved group alias");
        assert_eq!(restored_group.views.len(), 2);
        assert!(restored_group
            .views
            .iter()
            .any(|view| view.session_id == "session-a"));
        assert!(restored_group
            .views
            .iter()
            .any(|view| view.session_id == "session-b"));
    }

    #[test]
    fn restore_profile_layout_if_contains_only_restores_matching_session_layout() {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let store = DesktopWorkspaceLayoutStore::with_shared(shared, DEFAULT_LAYOUT_WORKSPACE_ID);

        let initial = store.ensure_for_session("session-a");
        store
            .split_view(
                initial.active_view_id,
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("join session-b");
        let _ = store.save_as_profile_layout("profile-a");

        let _ = store.replace_layout(WorkspaceLayoutState::new_single("session-c".to_string()));
        assert!(store
            .restore_profile_layout_if_contains("profile-a", "session-a")
            .is_some());
        let restored = store.snapshot().expect("restored layout");
        assert_eq!(restored.views.len(), 2);

        let _ = store.replace_layout(WorkspaceLayoutState::new_single("session-c".to_string()));
        assert!(store
            .restore_profile_layout_if_contains("profile-a", "session-z")
            .is_none());
        let unchanged = store.snapshot().expect("unchanged layout");
        assert_eq!(unchanged.views.len(), 1);
        assert_eq!(unchanged.views[0].session_id, "session-c");
    }

    #[test]
    fn restore_profile_layout_prefers_target_session_when_saved_layout_excludes_it() {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let store = DesktopWorkspaceLayoutStore::with_shared(shared, DEFAULT_LAYOUT_WORKSPACE_ID);

        let initial = store.ensure_for_session("session-a");
        store
            .split_view(
                initial.active_view_id,
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("join session-b");
        let _ = store.save_as_profile_layout("profile-a");

        let restored = store
            .restore_profile_layout("profile-a", Some("session-c"))
            .expect("restore target session-c");

        assert_eq!(restored.views.len(), 1);
        assert_eq!(restored.views[0].session_id, "session-c");
        assert_eq!(restored.active_view_id, restored.views[0].view_id);

        let saved_profile_layout = store
            .with_workspace_id(DesktopWorkspaceLayoutStore::profile_workspace_id(
                "profile-a",
            ))
            .snapshot()
            .expect("saved profile-a layout");
        assert_eq!(saved_profile_layout.views.len(), 2);
        assert!(saved_profile_layout
            .views
            .iter()
            .any(|view| view.session_id == "session-a"));
        assert!(saved_profile_layout
            .views
            .iter()
            .any(|view| view.session_id == "session-b"));
    }

    #[test]
    fn restore_profile_layout_if_contains_prefers_session_group_alias() {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let store = DesktopWorkspaceLayoutStore::with_shared(shared, DEFAULT_LAYOUT_WORKSPACE_ID);

        let initial = store.ensure_for_session("session-a");
        store
            .split_view(
                initial.active_view_id,
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("join session-b");
        let _ = store.save_as_profile_layout("profile-a");

        let _ = store.replace_layout(WorkspaceLayoutState::new_single("session-c".to_string()));
        let restored = store
            .restore_profile_layout_if_contains("profile-a", "session-b")
            .expect("restore session-b group layout");

        assert_eq!(restored.views.len(), 2);
        assert!(restored
            .views
            .iter()
            .any(|view| view.session_id == "session-a"));
        assert!(restored
            .views
            .iter()
            .any(|view| view.session_id == "session-b"));
    }

    #[test]
    fn profile_can_persist_multiple_join_groups_without_overwriting_each_other() {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let store = DesktopWorkspaceLayoutStore::with_shared(shared, DEFAULT_LAYOUT_WORKSPACE_ID);

        let initial = store.ensure_for_session("session-a");
        store
            .split_view(
                initial.active_view_id,
                WorkspaceSplitAxis::Vertical,
                "session-b",
            )
            .expect("join session-b");
        let _ = store.save_as_profile_layout("profile-a");

        let second_group = WorkspaceLayoutState::grouped_sessions(&[
            "session-c".to_string(),
            "session-d".to_string(),
        ])
        .expect("second joined group");
        let _ = store.replace_layout(second_group);
        let _ = store.save_as_profile_layout("profile-a");

        let restored_ab = store
            .restore_profile_layout_if_contains("profile-a", "session-a")
            .expect("restore session-a group");
        assert_eq!(restored_ab.views.len(), 2);
        assert!(restored_ab
            .views
            .iter()
            .any(|view| view.session_id == "session-a"));
        assert!(restored_ab
            .views
            .iter()
            .any(|view| view.session_id == "session-b"));

        let restored_cd = store
            .restore_profile_layout_if_contains("profile-a", "session-c")
            .expect("restore session-c group");
        assert_eq!(restored_cd.views.len(), 2);
        assert!(restored_cd
            .views
            .iter()
            .any(|view| view.session_id == "session-c"));
        assert!(restored_cd
            .views
            .iter()
            .any(|view| view.session_id == "session-d"));
    }

    #[test]
    fn profile_group_session_ids_returns_full_join_group_for_member_session() {
        let shared = Arc::new(SessionEngineShared::new(Arc::new(Mutex::new(
            SessionCoreState::default(),
        ))));
        let store = DesktopWorkspaceLayoutStore::with_shared(shared, DEFAULT_LAYOUT_WORKSPACE_ID);

        let joined = WorkspaceLayoutState::grouped_sessions(&[
            "session-a".to_string(),
            "session-b".to_string(),
            "session-c".to_string(),
        ])
        .expect("joined group");
        let _ = store.replace_layout(joined);
        let _ = store.save_as_profile_layout("profile-a");

        assert_eq!(
            store.profile_group_session_ids("profile-a", "session-b"),
            vec![
                "session-a".to_string(),
                "session-b".to_string(),
                "session-c".to_string()
            ]
        );
        assert!(store
            .profile_group_session_ids("profile-a", "session-z")
            .is_empty());
    }
}
