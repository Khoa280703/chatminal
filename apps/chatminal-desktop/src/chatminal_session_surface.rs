use std::convert::TryFrom;
use std::sync::Arc;

use crate::scripting::guiwin::DesktopWindowId;
use chatminal_session_runtime::{
    ChatminalEngineSurfaceAdapter, ChatminalMuxSessionEngine, LeafId, MoveLeafTarget,
    SessionEngine, SessionEngineShared, SessionSurfaceLookup, SessionSurfaceState,
    SpawnSessionSurfaceRequest, SurfaceId,
};
use config::keyassignment::PaneDirection;
use engine_term::TerminalSize;
use mux::pane::PaneId;
use mux::tab::Tab;
use mux::window::WindowId as EngineWindowId;
use window::Window;

fn engine_window_id(window_id: DesktopWindowId) -> Option<EngineWindowId> {
    EngineWindowId::try_from(window_id).ok()
}

fn session_engine_shared() -> Option<Arc<SessionEngineShared>> {
    crate::chatminal_runtime::EmbeddedRuntime::global()
        .ok()
        .map(|runtime| runtime.state.session_engine_shared())
}

fn session_engine(window_id: DesktopWindowId) -> Option<ChatminalMuxSessionEngine> {
    Some(ChatminalMuxSessionEngine::with_shared(
        ChatminalEngineSurfaceAdapter::new(engine_window_id(window_id)?),
        session_engine_shared()?,
    ))
}

pub fn focus_session_surface_state(
    window_id: DesktopWindowId,
    session_id: &str,
) -> Option<SessionSurfaceState> {
    session_engine(window_id)?
        .focus_session_state(session_id)
        .ok()
}

pub fn focus_surface_state(
    window_id: DesktopWindowId,
    surface_id: SurfaceId,
) -> Option<SessionSurfaceState> {
    session_engine(window_id)?
        .focus_surface_state(surface_id)
        .ok()
}

pub fn remove_session_surface(window_id: DesktopWindowId, session_id: &str) -> bool {
    session_engine(window_id)
        .map(|engine| engine.remove_session_surface(session_id).is_ok())
        .unwrap_or(false)
}

pub fn collect_session_surface_lookup(window_id: DesktopWindowId) -> SessionSurfaceLookup {
    session_engine(window_id)
        .map(|engine| engine.collect_session_surface_lookup())
        .unwrap_or_default()
}

pub fn active_session_id(window_id: DesktopWindowId) -> Option<String> {
    collect_session_surface_lookup(window_id).active_session_id
}

pub fn session_id_for_host_surface(
    window_id: DesktopWindowId,
    host_surface_id: usize,
) -> Option<String> {
    let lookup = collect_session_surface_lookup(window_id);
    lookup.surface_ids_by_session.keys().find_map(|session_id| {
        host_surface_for_session(window_id, session_id)
            .filter(|tab| tab.tab_id() == host_surface_id)
            .map(|_| session_id.clone())
    })
}

pub fn host_surface_for_session(window_id: DesktopWindowId, session_id: &str) -> Option<Arc<Tab>> {
    session_engine(window_id)?.host_surface_for_session(session_id)
}

pub fn host_surface_for_public_surface(
    window_id: DesktopWindowId,
    surface_id: SurfaceId,
) -> Option<Arc<Tab>> {
    let lookup = collect_session_surface_lookup(window_id);
    lookup
        .surface_ids_by_session
        .iter()
        .find_map(|(session_id, mapped_surface_id)| {
            (*mapped_surface_id == surface_id).then(|| host_surface_for_session(window_id, session_id))?
        })
}

pub fn host_surface_id_for_public_surface(
    window_id: DesktopWindowId,
    surface_id: SurfaceId,
) -> Option<usize> {
    host_surface_for_public_surface(window_id, surface_id).map(|tab| tab.tab_id())
}

pub fn surface_id_for_session(window_id: DesktopWindowId, session_id: &str) -> Option<SurfaceId> {
    session_engine(window_id)?.surface_id_for_session(session_id)
}

pub fn active_leaf_id(window_id: DesktopWindowId, session_id: &str) -> Option<LeafId> {
    session_engine(window_id)?.active_leaf_id(session_id)
}

pub fn focus_session_leaf(
    window_id: DesktopWindowId,
    session_id: &str,
    leaf_id: LeafId,
) -> Option<SessionSurfaceState> {
    session_engine(window_id)?
        .focus_session_leaf(session_id, leaf_id)
        .ok()
}

pub fn swap_active_with_session_leaf(
    window_id: DesktopWindowId,
    session_id: &str,
    leaf_id: LeafId,
    keep_focus: bool,
) -> bool {
    session_engine(window_id)
        .map(|engine| {
            engine
                .swap_active_with_session_leaf(session_id, leaf_id, keep_focus)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn move_session_leaf_to_new_window(
    window_id: DesktopWindowId,
    session_id: &str,
    leaf_id: LeafId,
) -> bool {
    session_engine(window_id)
        .map(|engine| {
            engine
                .move_session_leaf(session_id, leaf_id, MoveLeafTarget::NewWindow)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn move_session_leaf_to_new_surface(
    window_id: DesktopWindowId,
    session_id: &str,
    leaf_id: LeafId,
) -> bool {
    session_engine(window_id)
        .map(|engine| {
            engine
                .move_session_leaf(session_id, leaf_id, MoveLeafTarget::NewSurfaceInWindow)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn activate_session_leaf_direction(
    window_id: DesktopWindowId,
    session_id: &str,
    direction: PaneDirection,
) -> Option<SessionSurfaceState> {
    session_engine(window_id)?
        .activate_session_direction(session_id, direction)
        .ok()?
}

pub fn spawn_session_surface(
    window_id: DesktopWindowId,
    session_id: String,
    terminal_size: TerminalSize,
    current_host_leaf_id: Option<PaneId>,
    workspace: String,
    window: Option<Window>,
) {
    let request = SpawnSessionSurfaceRequest {
        session_id: session_id.clone(),
        terminal_size,
        current_host_leaf_id,
        workspace,
        domain: crate::chatminal_runtime::resolve_spawn_domain(),
        command: crate::chatminal_runtime::runtime_proxy_command(Some(&session_id)),
    };
    let Some(engine) = session_engine(window_id) else {
        log::error!("failed to ensure session surface {session_id}: invalid desktop window id");
        return;
    };
    if let Err(err) = engine.ensure_session_surface(&session_id, request, window) {
        log::error!("failed to ensure session surface {session_id}: {err}");
    }
}
