//! Business runtime for Chatminal.
//!
//! Ownership boundary:
//! - this crate owns workspace/profile/session metadata, persistence policy, and layout types
//! - live execution ids (RuntimeId, TerminalInstanceId) are product model — also live here
//! - this crate now owns the session execution engine for the active product path
//! - desktop/UI should consume runtime-owned execution state, not own another runtime layer

pub mod api;
pub mod client;
pub mod config;
pub mod execution;
pub mod host_notification;
pub mod local_spawn_target;
pub mod localpane;
pub(crate) mod localpane_hooks;
pub mod metrics;
pub mod pane;
pub(crate) mod pty_io;
pub mod renderable;
pub mod runtime_entry;
pub mod session;
pub mod state;
pub mod surface;
pub mod terminal_search;
pub mod terminal_text_utils;
pub mod termwiztermtab;
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
pub use client::{ClientId, ClientInfo};
pub use config::{RuntimeConfig, resolve_session_cwd};
pub use host_notification::HostRuntimeNotification;
pub use local_spawn_target::{LocalSpawnHooks, LocalSpawnTarget};
pub use localpane_hooks::LocalPaneHooks;
pub use metrics::{RuntimeMetrics, RuntimeMetricsSnapshot};
pub use runtime_entry::{
    FocusedPaneBinding, RuntimeEntryInfo, RuntimeEntrySplitInfo, RuntimeEntryTerminalInfo,
};
pub use session::{InputWriteStats, SessionEvent, WriteInputError};
pub use state::{RuntimeState, RuntimeSubscription};
pub use surface::{
    CachePolicy, LogicalLine, PaneEntry, PaneNode, RenderableDimensions, SerdeUrl, SplitDirection,
    SplitDirectionAndSize, SplitRequest, SplitSize, StableCursorPosition,
};
pub use terminal_search::{Pattern, PatternType, SearchResult};
pub use workspace_ids::{
    RuntimeId, SessionGroupId, SessionRenderTargetId, SessionTerminalHandle, SessionViewId,
    TerminalInstanceId, WorkspaceNodeId,
};
pub use workspace_layout::{
    SessionViewSnapshot, WorkspaceLayoutNodeKind, WorkspaceLayoutNodeSnapshot,
    WorkspaceLayoutRegistry, WorkspaceLayoutState, WorkspaceSplitAxis,
};
