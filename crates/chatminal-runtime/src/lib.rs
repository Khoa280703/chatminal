//! Business runtime for Chatminal.
//!
//! Ownership boundary:
//! - this crate owns workspace/profile/session metadata and persistence policy
//! - live surface/layout ids belong to `chatminal-session-runtime`
//! - desktop/UI should not treat engine tab/pane handles as business identity

pub mod api;
pub mod config;
pub mod metrics;
pub mod server;
pub mod session;
pub mod state;
pub mod transport;

pub use api::{
    RuntimeCreatedSession, RuntimeDaemonHealthEvent, RuntimeEvent, RuntimeLifecyclePreferences,
    RuntimeProfile, RuntimePtyErrorEvent, RuntimePtyExitedEvent, RuntimePtyOutputEvent,
    RuntimeSession, RuntimeSessionExplorerEntry, RuntimeSessionExplorerFileContent,
    RuntimeSessionExplorerState, RuntimeSessionSnapshot, RuntimeSessionStatus,
    RuntimeSessionUpdatedEvent, RuntimeWorkspace, RuntimeWorkspaceUpdatedEvent,
};
pub use config::{DaemonConfig, resolve_session_cwd};
pub use metrics::{RuntimeMetrics, RuntimeMetricsSnapshot};
pub use session::{InputWriteStats, SessionEvent, WriteInputError};
pub use state::{DaemonState, RuntimeSubscription};
