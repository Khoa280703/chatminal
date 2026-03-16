//! Business runtime for Chatminal.
//!
//! Ownership boundary:
//! - this crate owns workspace/profile/session metadata, persistence policy, and layout types
//! - live execution ids (RuntimeId, TerminalInstanceId) are product model — also live here
//! - `chatminal-session-runtime` is the execution engine; only `desktop_host_runtime` depends on it
//! - desktop/UI should not treat engine tab/pane handles as business identity

pub mod api;
pub mod config;
pub mod metrics;
pub mod server;
pub mod session;
pub mod state;
pub mod transport;
pub mod workspace_ids;
pub mod workspace_layout;

pub use api::{
    RuntimeCreatedSession, RuntimeDaemonHealthEvent, RuntimeEvent, RuntimeLifecyclePreferences,
    RuntimeProfile, RuntimePtyErrorEvent, RuntimePtyExitedEvent, RuntimePtyOutputEvent,
    RuntimeSessionBridgeAction, RuntimeSessionLookup, RuntimeSession,
    RuntimeSessionExplorerEntry, RuntimeSessionExplorerFileContent,
    RuntimeSessionExplorerState, RuntimeSessionSnapshot, RuntimeSessionStatus,
    RuntimeSessionUpdatedEvent, RuntimeWorkspace, RuntimeWorkspaceUpdatedEvent,
    SessionEngineCapability, SessionGroupId, SessionGroupSnapshot, SessionLayoutTarget,
    SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle,
    SessionViewBinding, SessionWindowBinding,
};
pub use config::{DaemonConfig, resolve_session_cwd};
pub use metrics::{RuntimeMetrics, RuntimeMetricsSnapshot};
pub use session::{InputWriteStats, SessionEvent, WriteInputError};
pub use state::{DaemonState, RuntimeSubscription, SessionExecutionStatus};
pub use state::runtime_bridge::{RuntimeExecutionAdapter, RuntimeSessionHandleTrait};
pub use workspace_ids::{RuntimeId, SessionViewId, TerminalInstanceId, WorkspaceNodeId};
pub use workspace_layout::{
    SessionViewSnapshot, WorkspaceLayoutNodeKind, WorkspaceLayoutNodeSnapshot,
    WorkspaceLayoutRegistry, WorkspaceLayoutState, WorkspaceSplitAxis,
};
