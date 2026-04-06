use std::collections::HashMap;

use crate::workspace_ids::{RuntimeId, SessionViewId, TerminalInstanceId};
pub use crate::workspace_ids::{SessionGroupId, SessionRenderTargetId, SessionTerminalHandle};
use serde::{Deserialize, Serialize};
use store::{
    StoredProfile, StoredSessionExplorerState, StoredSessionSnapshot, StoredSessionStatus,
    StoredSessionSummary,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSessionStatus {
    Running,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeProfile {
    pub profile_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSession {
    pub session_id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub startup_command: Option<String>,
    pub status: RuntimeSessionStatus,
    pub persist_history: bool,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeWorkspace {
    pub profiles: Vec<RuntimeProfile>,
    pub active_profile_id: Option<String>,
    pub sessions: Vec<RuntimeSession>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeCreatedSession {
    pub session_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeLifecyclePreferences {
    pub keep_alive_on_close: bool,
    pub start_in_tray: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSessionSnapshot {
    pub content: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSessionExplorerState {
    pub session_id: String,
    pub root_path: Option<String>,
    pub current_dir: String,
    pub selected_path: Option<String>,
    pub open_file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSessionExplorerEntry {
    pub name: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSessionExplorerFileContent {
    pub relative_path: String,
    pub content: String,
    pub truncated: bool,
    pub byte_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimePtyOutputEvent {
    pub session_id: String,
    pub chunk: String,
    pub seq: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimePtyExitedEvent {
    pub session_id: String,
    pub exit_code: Option<i32>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimePtyErrorEvent {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSessionUpdatedEvent {
    pub session_id: String,
    pub status: RuntimeSessionStatus,
    pub seq: u64,
    pub persist_history: bool,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeWorkspaceUpdatedEvent {
    pub active_profile_id: Option<String>,
    pub active_session_id: Option<String>,
    pub profile_count: u64,
    pub session_count: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum RuntimeEvent {
    PtyOutput(RuntimePtyOutputEvent),
    PtyExited(RuntimePtyExitedEvent),
    PtyError(RuntimePtyErrorEvent),
    SessionUpdated(RuntimeSessionUpdatedEvent),
    WorkspaceUpdated(RuntimeWorkspaceUpdatedEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionLaunchSpec {
    pub session_id: String,
    pub shell: String,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRenderTargetSnapshot {
    pub render_target_id: SessionRenderTargetId,
    pub runtime_id: RuntimeId,
    pub active_terminal_instance_id: Option<TerminalInstanceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionLayoutTarget {
    View(SessionViewId),
    Group(SessionGroupId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionGroupSnapshot {
    pub group_id: SessionGroupId,
    pub layout_target: SessionLayoutTarget,
    pub view_ids: Vec<SessionViewId>,
    pub active_view_id: Option<SessionViewId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionViewBinding {
    pub workspace_id: String,
    pub session_id: String,
    pub view_id: SessionViewId,
    pub group_id: Option<SessionGroupId>,
    pub layout_target: SessionLayoutTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWindowBinding {
    pub workspace_id: String,
    pub active_session_id: Option<String>,
    pub active_view_id: Option<SessionViewId>,
    pub active_render_target_id: Option<SessionRenderTargetId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEngineCapability {
    pub supports_terminal_focus: bool,
    pub supports_directional_focus: bool,
    pub supports_session_view_split: bool,
    pub supports_terminal_detach: bool,
}

impl SessionEngineCapability {
    pub const fn desktop_embedded() -> Self {
        Self {
            supports_terminal_focus: true,
            supports_directional_focus: true,
            supports_session_view_split: true,
            supports_terminal_detach: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeSessionLookup {
    pub active_session_id: Option<String>,
    pub last_active_session_id: Option<String>,
    pub runtime_ids_by_session: HashMap<String, crate::RuntimeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSessionBridgeAction {
    Noop,
    FocusSession { session_id: String },
}

impl From<StoredSessionStatus> for RuntimeSessionStatus {
    fn from(value: StoredSessionStatus) -> Self {
        match value {
            StoredSessionStatus::Running => Self::Running,
            StoredSessionStatus::Disconnected => Self::Disconnected,
        }
    }
}

impl From<RuntimeSessionStatus> for StoredSessionStatus {
    fn from(value: RuntimeSessionStatus) -> Self {
        match value {
            RuntimeSessionStatus::Running => Self::Running,
            RuntimeSessionStatus::Disconnected => Self::Disconnected,
        }
    }
}

impl From<StoredProfile> for RuntimeProfile {
    fn from(value: StoredProfile) -> Self {
        Self {
            profile_id: value.profile_id,
            name: value.name,
        }
    }
}

impl From<StoredSessionSummary> for RuntimeSession {
    fn from(value: StoredSessionSummary) -> Self {
        Self {
            session_id: value.session_id,
            profile_id: value.profile_id,
            name: value.name,
            cwd: value.cwd,
            startup_command: value.startup_command,
            status: value.status.into(),
            persist_history: value.persist_history,
            seq: value.seq,
        }
    }
}

impl From<StoredSessionSnapshot> for RuntimeSessionSnapshot {
    fn from(value: StoredSessionSnapshot) -> Self {
        Self {
            content: value.content,
            seq: value.seq,
        }
    }
}

impl From<Option<StoredSessionExplorerState>> for RuntimeSessionExplorerState {
    fn from(value: Option<StoredSessionExplorerState>) -> Self {
        match value {
            Some(state) => Self {
                session_id: state.session_id,
                root_path: Some(state.root_path),
                current_dir: state.current_dir,
                selected_path: state.selected_path,
                open_file_path: state.open_file_path,
            },
            None => Self {
                session_id: String::new(),
                root_path: None,
                current_dir: String::new(),
                selected_path: None,
                open_file_path: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeProfile, RuntimeSession, RuntimeSessionStatus, SessionEngineCapability,
        SessionGroupId, SessionGroupSnapshot, SessionLayoutTarget, SessionRenderTargetId,
        SessionRenderTargetSnapshot, SessionTerminalHandle, SessionViewBinding,
        SessionWindowBinding,
    };
    use store::{StoredProfile, StoredSessionStatus, StoredSessionSummary};

    #[test]
    fn boundary_ids_use_stable_prefixes() {
        assert_eq!(SessionRenderTargetId::new(7).to_string(), "render-target-7");
        assert_eq!(
            SessionTerminalHandle::new(11).to_string(),
            "terminal-handle-11"
        );
        assert_eq!(SessionGroupId::new(13).to_string(), "group-13");
    }

    #[test]
    fn boundary_snapshots_capture_chatminal_terms() {
        let view_id = crate::SessionViewId::new(17);
        let group_id = SessionGroupId::new(19);
        let render_target = SessionRenderTargetSnapshot {
            render_target_id: SessionRenderTargetId::new(23),
            runtime_id: crate::RuntimeId::new(29),
            active_terminal_instance_id: Some(crate::TerminalInstanceId::new(31)),
        };
        let view_binding = SessionViewBinding {
            workspace_id: "desktop".to_string(),
            session_id: "session-a".to_string(),
            view_id,
            group_id: Some(group_id),
            layout_target: SessionLayoutTarget::View(view_id),
        };
        let group = SessionGroupSnapshot {
            group_id,
            layout_target: SessionLayoutTarget::Group(group_id),
            view_ids: vec![view_id],
            active_view_id: Some(view_id),
        };
        let window = SessionWindowBinding {
            workspace_id: "desktop".to_string(),
            active_session_id: Some("session-a".to_string()),
            active_view_id: Some(view_id),
            active_render_target_id: Some(render_target.render_target_id),
        };
        let capability = SessionEngineCapability::desktop_embedded();

        assert_eq!(view_binding.group_id, Some(group.group_id));
        assert_eq!(
            window.active_render_target_id,
            Some(render_target.render_target_id)
        );
        assert!(capability.supports_session_view_split);
    }

    #[test]
    fn store_conversions_stay_runtime_native() {
        let profile = RuntimeProfile::from(StoredProfile {
            profile_id: "profile-a".to_string(),
            name: "Default".to_string(),
        });
        let session = RuntimeSession::from(StoredSessionSummary {
            session_id: "session-a".to_string(),
            profile_id: "profile-a".to_string(),
            name: "Shell".to_string(),
            cwd: "/tmp".to_string(),
            startup_command: Some("claude".to_string()),
            status: StoredSessionStatus::Running,
            persist_history: true,
            seq: 7,
        });

        assert_eq!(profile.profile_id, "profile-a");
        assert_eq!(session.status, RuntimeSessionStatus::Running);
        assert!(session.persist_history);
        assert_eq!(session.startup_command.as_deref(), Some("claude"));
    }
}
