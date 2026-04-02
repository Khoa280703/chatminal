//! Business runtime for Chatminal.
//!
//! Ownership boundary:
//! - this crate owns workspace/profile/session metadata, persistence policy, and layout types
//! - live execution ids (RuntimeId, TerminalInstanceId) are product model — also live here
//! - `desktop_host_runtime::session_engine` is the execution engine; only `desktop_host_runtime` depends on it
//! - desktop/UI should not treat engine tab/pane handles as business identity

pub mod api;
pub mod config;
pub mod metrics;
pub mod runtime_host;
pub mod session;
pub mod state;
pub mod terminal_text_utils;
pub mod workspace_ids;
pub mod workspace_layout;

pub use api::{
    RuntimeCreatedSession, RuntimeEvent, RuntimeLifecyclePreferences, RuntimeProfile,
    RuntimePtyErrorEvent, RuntimePtyExitedEvent, RuntimePtyOutputEvent, RuntimeSession,
    RuntimeSessionBridgeAction, RuntimeSessionExplorerEntry, RuntimeSessionExplorerFileContent,
    RuntimeSessionExplorerState, RuntimeSessionLaunchSpec, RuntimeSessionLookup,
    RuntimeSessionSnapshot, RuntimeSessionStatus, RuntimeSessionUpdatedEvent, RuntimeWorkspace,
    RuntimeWorkspaceUpdatedEvent, SessionEngineCapability, SessionGroupSnapshot,
    SessionLayoutTarget, SessionRenderTargetSnapshot, SessionViewBinding, SessionWindowBinding,
};
pub use config::{RuntimeConfig, resolve_session_cwd};
pub use metrics::{RuntimeMetrics, RuntimeMetricsSnapshot};
pub use runtime_host::{
    RuntimeHost, RuntimeHostSessionState, RuntimeHostTerminalBinding, RuntimeTerminalSize,
};
pub use session::{InputWriteStats, SessionEvent, WriteInputError};
pub use state::runtime_bridge::{RuntimeExecutionAdapter, RuntimeSessionHandleTrait};
pub use state::{RuntimeState, RuntimeSubscription};
pub use workspace_ids::{
    RuntimeId, SessionGroupId, SessionRenderTargetId, SessionTerminalHandle, SessionViewId,
    TerminalInstanceId, WorkspaceNodeId,
};
pub use workspace_layout::{
    SessionViewSnapshot, WorkspaceLayoutNodeKind, WorkspaceLayoutNodeSnapshot,
    WorkspaceLayoutRegistry, WorkspaceLayoutState, WorkspaceSplitAxis,
};
