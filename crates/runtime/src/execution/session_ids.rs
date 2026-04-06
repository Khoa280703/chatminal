// Phase 07: RuntimeId and TerminalInstanceId moved to
// `chatminal-runtime`. Re-export here so internal code keeps compiling without
// mass-rewriting all the `use super::` imports inside session-runtime.
//
// LayoutNodeId is engine-internal and stays here.

use std::fmt;

use serde::{Deserialize, Serialize};

// Re-export product-model IDs from chatminal-runtime (the new canonical location).
pub use crate::{RuntimeId, TerminalInstanceId};

macro_rules! session_id_type {
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
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($prefix, "-{}"), self.0)
            }
        }
    };
}

// Engine-internal only — not promoted to chatminal-runtime.
session_id_type!(LayoutNodeId, "layout");

#[cfg(test)]
mod tests {
    use crate::{SessionViewId, WorkspaceNodeId};

    use super::{LayoutNodeId, RuntimeId, TerminalInstanceId};

    #[test]
    fn ids_format_with_stable_prefixes() {
        assert_eq!(RuntimeId::new(7).to_string(), "runtime-7");
        assert_eq!(
            TerminalInstanceId::new(11).to_string(),
            "terminal-instance-11"
        );
        assert_eq!(LayoutNodeId::new(13).to_string(), "layout-13");
        assert_eq!(SessionViewId::new(17).to_string(), "view-17");
        assert_eq!(WorkspaceNodeId::new(19).to_string(), "workspace-node-19");
    }
}
