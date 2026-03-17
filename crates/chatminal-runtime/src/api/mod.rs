use std::collections::HashMap;

use crate::workspace_ids::{RuntimeId, SessionViewId, TerminalInstanceId};
use serde::{Deserialize, Serialize};

macro_rules! runtime_boundary_id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            pub const fn as_u64(self) -> u64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!($prefix, "-{}"), self.0)
            }
        }
    };
}

runtime_boundary_id_type!(SessionRenderTargetId, "render-target");
runtime_boundary_id_type!(SessionTerminalHandle, "terminal-handle");
runtime_boundary_id_type!(SessionGroupId, "group");

// ─── Protocol type aliases (17 types unified with chatminal-protocol) ────────

pub type RuntimeSessionStatus = chatminal_protocol::SessionStatus;
pub type RuntimeProfile = chatminal_protocol::ProfileInfo;
pub type RuntimeSession = chatminal_protocol::SessionInfo;
pub type RuntimeWorkspace = chatminal_protocol::WorkspaceState;
pub type RuntimeCreatedSession = chatminal_protocol::CreateSessionResponse;
pub type RuntimeLifecyclePreferences = chatminal_protocol::LifecyclePreferences;
pub type RuntimeSessionSnapshot = chatminal_protocol::SessionSnapshot;
pub type RuntimeSessionExplorerState = chatminal_protocol::SessionExplorerState;
pub type RuntimeSessionExplorerEntry = chatminal_protocol::SessionExplorerEntry;
pub type RuntimeSessionExplorerFileContent = chatminal_protocol::SessionExplorerFileContent;
pub type RuntimePtyOutputEvent = chatminal_protocol::PtyOutputEvent;
pub type RuntimePtyExitedEvent = chatminal_protocol::PtyExitedEvent;
pub type RuntimePtyErrorEvent = chatminal_protocol::PtyErrorEvent;
pub type RuntimeSessionUpdatedEvent = chatminal_protocol::SessionUpdatedEvent;
pub type RuntimeWorkspaceUpdatedEvent = chatminal_protocol::WorkspaceUpdatedEvent;
pub type RuntimeDaemonHealthEvent = chatminal_protocol::DaemonHealthEvent;
pub type RuntimeEvent = chatminal_protocol::Event;

// ─── Runtime-unique types (no protocol counterpart) ──────────────────────────

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
    pub window_id: u64,
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

#[cfg(test)]
mod tests {
    use super::{
        SessionEngineCapability, SessionGroupId, SessionGroupSnapshot, SessionLayoutTarget,
        SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle,
        SessionViewBinding, SessionWindowBinding,
    };

    #[test]
    fn boundary_ids_use_stable_prefixes() {
        assert_eq!(SessionRenderTargetId::new(7).to_string(), "render-target-7");
        assert_eq!(SessionTerminalHandle::new(11).to_string(), "terminal-handle-11");
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
            window_id: 41,
            workspace_id: "desktop".to_string(),
            active_session_id: Some("session-a".to_string()),
            active_view_id: Some(view_id),
            active_render_target_id: Some(render_target.render_target_id),
        };
        let capability = SessionEngineCapability::desktop_embedded();

        assert_eq!(view_binding.group_id, Some(group.group_id));
        assert_eq!(window.active_render_target_id, Some(render_target.render_target_id));
        assert!(capability.supports_session_view_split);
    }
}
