// Workspace + session execution identity types.
//
// These are "product model" primitive IDs — they cross crate boundaries and must
// live in `chatminal-runtime` so callers only need one dependency.
//
// `LayoutNodeId` stays in `desktop_host_runtime::session_engine` (engine-internal only).
// `SessionViewId` / `WorkspaceNodeId` are workspace product model → live here.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! workspace_id_type {
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

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($prefix, "-{}"), self.0)
            }
        }
    };
}

workspace_id_type!(RuntimeId, "runtime");
workspace_id_type!(TerminalInstanceId, "terminal-instance");
workspace_id_type!(SessionViewId, "view");
workspace_id_type!(WorkspaceNodeId, "workspace-node");
workspace_id_type!(SessionRenderTargetId, "render-target");
workspace_id_type!(SessionTerminalHandle, "terminal-handle");
workspace_id_type!(SessionGroupId, "group");

#[cfg(test)]
mod tests {
    use super::{
        RuntimeId, SessionGroupId, SessionRenderTargetId, SessionTerminalHandle, SessionViewId,
        TerminalInstanceId, WorkspaceNodeId,
    };

    #[test]
    fn ids_format_with_stable_prefixes() {
        assert_eq!(RuntimeId::new(7).to_string(), "runtime-7");
        assert_eq!(
            TerminalInstanceId::new(11).to_string(),
            "terminal-instance-11"
        );
        assert_eq!(SessionViewId::new(17).to_string(), "view-17");
        assert_eq!(WorkspaceNodeId::new(19).to_string(), "workspace-node-19");
        assert_eq!(
            SessionRenderTargetId::new(23).to_string(),
            "render-target-23"
        );
        assert_eq!(
            SessionTerminalHandle::new(29).to_string(),
            "terminal-handle-29"
        );
        assert_eq!(SessionGroupId::new(31).to_string(), "group-31");
    }
}
