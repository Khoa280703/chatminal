// Session execution engine (inlined from former chatminal-session-runtime crate, Phase 08).
// All execution engine code lives here under desktop_host_runtime::session_engine.
// Only desktop_host_runtime and chatminald may depend on this module.

mod leaf_runtime;
mod leaf_runtime_command;
mod leaf_runtime_registry;
mod leaf_runtime_threads;
mod runtime_bridge;
mod session_core_ids;
mod session_core_state;
mod session_engine;
mod session_engine_core;
mod session_engine_shared;
mod session_event_bus;
mod session_ids;
mod session_layout_tree;
mod session_runtime_state;
mod session_snapshot;
mod workspace_host;

pub use runtime_bridge::{SessionBridgeAction, SessionRuntimeBridge};
pub use session_core_state::{
    SessionCoreState, TerminalInstanceProcessState,
};
pub use session_engine::StatefulSessionEngine;
pub use session_engine_shared::SessionEngineShared;
pub use session_event_bus::{SessionEventBus, SessionEventSubscription, SessionRuntimeEvent};
pub use session_ids::{LayoutNodeId, RuntimeId, TerminalInstanceId};
pub use session_layout_tree::SessionLayoutSnapshot;
pub use session_runtime_state::SessionRuntimeState;
pub use session_snapshot::{SessionRuntimeLookup, SessionRuntimeSnapshot};
pub use workspace_host::SessionWorkspaceHost;
