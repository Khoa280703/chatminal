//! Live session graph primitives for Chatminal.
//!
//! Boundary freeze for Direction B:
//! - `chatminal-runtime` owns business/workspace metadata and persisted `session_id`.
//! - `chatminal-session-runtime` owns live `surface_id`, `leaf_id`, `layout_node_id`.
//! - Desktop/UI must consume session/surface snapshots instead of inventing new ids.
//! - Terminal engine details stay behind the engine adapter layer.

mod engine_surface_adapter;
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
mod session_focus_manager;
mod session_ids;
mod session_layout_tree;
mod session_snapshot;
mod session_spawn_manager;
mod session_surface;
mod workspace_host;

pub use engine_surface_adapter::{
    ChatminalEngineSurfaceAdapter, EngineSurfaceAdapter, EngineSurfaceRef, MoveLeafTarget,
    SpawnSessionSurfaceRequest,
};
pub use leaf_runtime::{LeafRuntime, LeafRuntimeEvent, LeafRuntimeSpawn};
pub use leaf_runtime_registry::LeafRuntimeRegistry;
pub use runtime_bridge::{SessionBridgeAction, SessionRuntimeBridge};
pub use session_core_state::{
    LeafProcessState, LeafRuntimeState, SessionCoreState, SurfaceRuntimeState,
};
pub use session_engine::{ChatminalMuxSessionEngine, SessionEngine, StatefulSessionEngine};
pub use session_engine_shared::SessionEngineShared;
pub use session_event_bus::{
    SessionEventBus, SessionEventHub, SessionEventSubscription, SessionRuntimeEvent,
};
pub use session_focus_manager::SessionFocusManager;
pub use session_ids::{LayoutNodeId, LeafId, SurfaceId};
pub use session_layout_tree::{
    SessionLayoutNodeKind, SessionLayoutNodeSnapshot, SessionLayoutSnapshot, SessionLeafSnapshot,
    SessionSplitAxis, build_layout_snapshot_from_engine,
};
pub use session_snapshot::{
    SessionGraphSnapshotVersion, SessionSurfaceLookup, SessionSurfaceSnapshot,
};
pub use session_spawn_manager::{EnsureSurfaceResult, SessionSpawnManager};
pub use session_surface::SessionSurfaceState;
pub use workspace_host::SessionWorkspaceHost;
