use std::sync::Arc;

use config::keyassignment::PaneDirection;
use config::keyassignment::SpawnTabDomain;
use engine_dynamic::Value;
use engine_term::TerminalSize;
use mux::Mux;
use mux::pane::{Pane, PaneId};
use mux::tab::{Tab, TabId};
use mux::window::WindowId as EngineWindowId;
use portable_pty::CommandBuilder;
use window::{Window, WindowOps};

use crate::{
    LeafId, SessionSurfaceLookup, SessionSurfaceState, SurfaceId, build_layout_snapshot_from_engine,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineSurfaceRef {
    pub surface_id: SurfaceId,
    pub session_id: String,
}

#[derive(Clone, Debug)]
pub struct SpawnSessionSurfaceRequest {
    pub session_id: String,
    pub terminal_size: TerminalSize,
    pub current_host_leaf_id: Option<PaneId>,
    pub workspace: String,
    pub domain: SpawnTabDomain,
    pub command: CommandBuilder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveLeafTarget {
    NewWindow,
    NewSurfaceInWindow,
}

/// Temporary engine boundary.
///
/// From Phase 03 onward, engine-specific mux calls are only allowed behind an
/// implementation of this trait.
pub trait EngineSurfaceAdapter: Send + Sync {
    type Error;

    fn collect_session_surface_lookup(&self) -> SessionSurfaceLookup {
        SessionSurfaceLookup::default()
    }

    fn attach_surface(&self, session_id: &str) -> Result<EngineSurfaceRef, Self::Error>;
    fn focus_surface(&self, surface_id: SurfaceId) -> Result<(), Self::Error>;
    fn focus_leaf(&self, surface_id: SurfaceId, leaf_id: LeafId) -> Result<(), Self::Error>;
    fn adjacent_active_leaf(
        &self,
        surface_id: SurfaceId,
        direction: PaneDirection,
    ) -> Result<Option<LeafId>, Self::Error>;
    fn swap_active_leaf(
        &self,
        surface_id: SurfaceId,
        leaf_id: LeafId,
        keep_focus: bool,
    ) -> Result<(), Self::Error>;
    fn move_leaf(
        &self,
        surface_id: SurfaceId,
        leaf_id: LeafId,
        target: MoveLeafTarget,
    ) -> Result<(), Self::Error>;
    fn close_surface(&self, surface_id: SurfaceId) -> Result<(), Self::Error>;
    fn spawn_surface(
        &self,
        request: SpawnSessionSurfaceRequest,
        window: Option<Window>,
    ) -> Result<(), Self::Error>;
    fn snapshot_surface(&self, surface_id: SurfaceId) -> Result<SessionSurfaceState, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatminalEngineSurfaceAdapter {
    window_id: EngineWindowId,
}

impl ChatminalEngineSurfaceAdapter {
    pub const fn new(window_id: EngineWindowId) -> Self {
        Self { window_id }
    }

    pub fn active_session_id(&self) -> Option<String> {
        Mux::get()
            .get_active_tab_for_window(self.window_id)
            .and_then(|host_surface| host_surface_session_id(&host_surface))
    }

    fn collect_session_surface_lookup_impl(&self) -> SessionSurfaceLookup {
        let mux = Mux::get();
        let Some(window) = mux.get_window(self.window_id) else {
            return SessionSurfaceLookup::default();
        };

        let active_session_id = mux
            .get_active_tab_for_window(self.window_id)
            .and_then(|host_surface| host_surface_session_id(&host_surface));
        let last_active_session_id = window
            .get_last_active_idx()
            .and_then(|idx| window.get_by_idx(idx))
            .and_then(|host_surface| host_surface_session_id(&host_surface));
        let surface_ids_by_session = window
            .iter()
            .filter_map(|host_surface| {
                host_surface_session_id(host_surface).map(|session_id| {
                    (session_id, public_surface_id_for_host_surface(host_surface))
                })
            })
            .collect();

        SessionSurfaceLookup {
            active_session_id,
            last_active_session_id,
            surface_ids_by_session,
        }
    }

    pub fn host_surface_id_for_session(&self, session_id: &str) -> Option<TabId> {
        let mux = Mux::get();
        mux.get_window(self.window_id).and_then(|window| {
            window.iter().find_map(|host_surface| {
                (host_surface_session_id(host_surface).as_deref() == Some(session_id))
                    .then(|| host_surface.tab_id())
            })
        })
    }

    pub fn host_surface_session_id(host_surface: &Arc<Tab>) -> Option<String> {
        host_surface_session_id(host_surface)
    }

    fn host_surface_id_for_public_surface(&self, surface_id: SurfaceId) -> Option<TabId> {
        let mux = Mux::get();
        mux.get_window(self.window_id).and_then(|window| {
            window
                .iter()
                .find(|host_surface| public_surface_id_for_host_surface(host_surface) == surface_id)
                .map(|host_surface| host_surface.tab_id())
        })
    }

    pub fn spawn_surface(&self, request: SpawnSessionSurfaceRequest, window: Option<Window>) {
        let window_id = self.window_id;
        promise::spawn::spawn(async move {
            match Mux::get()
                .spawn_tab_or_window(
                    Some(window_id),
                    request.domain,
                    Some(request.command),
                    None,
                    request.terminal_size,
                    request.current_host_leaf_id,
                    request.workspace,
                    None,
                )
                .await
            {
                Ok(_) => {
                    if let Some(window) = window.as_ref() {
                        window.invalidate();
                    }
                }
                Err(err) => {
                    log::error!(
                        "failed to switch chatminal sidebar session {}: {:#}",
                        request.session_id,
                        err
                    );
                }
            }
        })
        .detach();
    }
}

impl EngineSurfaceAdapter for ChatminalEngineSurfaceAdapter {
    type Error = &'static str;

    fn collect_session_surface_lookup(&self) -> SessionSurfaceLookup {
        self.collect_session_surface_lookup_impl()
    }

    fn attach_surface(&self, session_id: &str) -> Result<EngineSurfaceRef, Self::Error> {
        self.host_surface_id_for_session(session_id)
            .and_then(|host_surface_id| Mux::get().get_tab(host_surface_id))
            .map(|host_surface| EngineSurfaceRef {
                surface_id: public_surface_id_for_host_surface(&host_surface),
                session_id: session_id.to_string(),
            })
            .ok_or("session surface not found")
    }

    fn focus_surface(&self, surface_id: SurfaceId) -> Result<(), Self::Error> {
        let host_surface_id = self
            .host_surface_id_for_public_surface(surface_id)
            .or_else(|| host_surface_id_from_public_surface(surface_id).ok())
            .ok_or("host surface missing")?;
        let mux = Mux::get();
        let pane = mux
            .get_tab(host_surface_id)
            .and_then(|host_surface| host_surface.get_active_pane())
            .ok_or("surface has no active pane")?;
        let host_surface = mux.get_tab(host_surface_id).ok_or("host surface missing")?;
        host_surface.set_active_pane(&pane);
        let Some(mut window) = mux.get_window_mut(self.window_id) else {
            return Err("window missing");
        };
        let Some(idx) = window.idx_by_id(host_surface_id) else {
            return Err("host surface not attached to window");
        };
        window.save_and_then_set_active(idx);
        Ok(())
    }

    fn focus_leaf(&self, surface_id: SurfaceId, leaf_id: LeafId) -> Result<(), Self::Error> {
        let host_surface_id = self
            .host_surface_id_for_public_surface(surface_id)
            .or_else(|| host_surface_id_from_public_surface(surface_id).ok())
            .ok_or("host surface missing")?;
        let mux = Mux::get();
        let host_surface = mux.get_tab(host_surface_id).ok_or("host surface missing")?;
        let pane = host_surface
            .iter_panes()
            .into_iter()
            .find(|pos| {
                pane_leaf_id(&pos.pane).unwrap_or_else(|| LeafId::new(pos.pane.pane_id() as u64))
                    == leaf_id
            })
            .map(|pos| pos.pane)
            .ok_or("leaf not found in surface")?;
        host_surface.set_active_pane(&pane);
        mux.focus_pane_and_containing_tab(pane.pane_id())
            .map_err(|_| "focus leaf failed")
    }

    fn adjacent_active_leaf(
        &self,
        surface_id: SurfaceId,
        direction: PaneDirection,
    ) -> Result<Option<LeafId>, Self::Error> {
        let host_surface_id = self
            .host_surface_id_for_public_surface(surface_id)
            .or_else(|| host_surface_id_from_public_surface(surface_id).ok())
            .ok_or("host surface missing")?;
        let mux = Mux::get();
        let host_surface = mux.get_tab(host_surface_id).ok_or("host surface missing")?;
        let Some(target_index) = host_surface.get_pane_direction(direction, false) else {
            return Ok(None);
        };
        Ok(host_surface
            .iter_panes()
            .into_iter()
            .find(|pos| pos.index == target_index)
            .map(|pos| {
                pane_leaf_id(&pos.pane).unwrap_or_else(|| LeafId::new(pos.pane.pane_id() as u64))
            }))
    }

    fn swap_active_leaf(
        &self,
        surface_id: SurfaceId,
        leaf_id: LeafId,
        keep_focus: bool,
    ) -> Result<(), Self::Error> {
        let host_surface_id = self
            .host_surface_id_for_public_surface(surface_id)
            .or_else(|| host_surface_id_from_public_surface(surface_id).ok())
            .ok_or("host surface missing")?;
        let mux = Mux::get();
        let host_surface = mux.get_tab(host_surface_id).ok_or("host surface missing")?;
        let target_index = host_surface
            .iter_panes()
            .into_iter()
            .find(|pos| {
                pane_leaf_id(&pos.pane).unwrap_or_else(|| LeafId::new(pos.pane.pane_id() as u64))
                    == leaf_id
            })
            .map(|pos| pos.index)
            .ok_or("leaf not found in surface")?;
        host_surface.swap_active_with_index(target_index, keep_focus)
            .ok_or("swap active leaf failed")
    }

    fn move_leaf(
        &self,
        surface_id: SurfaceId,
        leaf_id: LeafId,
        target: MoveLeafTarget,
    ) -> Result<(), Self::Error> {
        let host_surface_id = self
            .host_surface_id_for_public_surface(surface_id)
            .or_else(|| host_surface_id_from_public_surface(surface_id).ok())
            .ok_or("host surface missing")?;
        let mux = Mux::get();
        let host_surface = mux.get_tab(host_surface_id).ok_or("host surface missing")?;
        let host_leaf_id = host_surface
            .iter_panes()
            .into_iter()
            .find(|pos| {
                pane_leaf_id(&pos.pane).unwrap_or_else(|| LeafId::new(pos.pane.pane_id() as u64))
                    == leaf_id
            })
            .map(|pos| pos.pane.pane_id())
            .ok_or("leaf not found in surface")?;
        let window_id = self.window_id;
        promise::spawn::spawn(async move {
            let mux = Mux::get();
            let result = match target {
                MoveLeafTarget::NewWindow => {
                    mux.move_pane_to_new_tab(host_leaf_id, None, None).await
                }
                MoveLeafTarget::NewSurfaceInWindow => {
                    mux.move_pane_to_new_tab(host_leaf_id, Some(window_id), None)
                        .await
                }
            };
            if let Err(err) = result {
                log::error!("failed to move leaf {leaf_id}: {err:#}");
                return;
            }
            if matches!(target, MoveLeafTarget::NewSurfaceInWindow) {
                let _ = mux.focus_pane_and_containing_tab(host_leaf_id);
            }
        })
        .detach();
        Ok(())
    }

    fn close_surface(&self, surface_id: SurfaceId) -> Result<(), Self::Error> {
        let host_surface_id = self
            .host_surface_id_for_public_surface(surface_id)
            .or_else(|| host_surface_id_from_public_surface(surface_id).ok())
            .ok_or("host surface missing")?;
        Mux::get().remove_tab(host_surface_id);
        Ok(())
    }

    fn spawn_surface(
        &self,
        request: SpawnSessionSurfaceRequest,
        window: Option<Window>,
    ) -> Result<(), Self::Error> {
        Self::spawn_surface(self, request, window);
        Ok(())
    }

    fn snapshot_surface(&self, surface_id: SurfaceId) -> Result<SessionSurfaceState, Self::Error> {
        let host_surface_id = self
            .host_surface_id_for_public_surface(surface_id)
            .or_else(|| host_surface_id_from_public_surface(surface_id).ok())
            .ok_or("host surface missing")?;
        let mux = Mux::get();
        let host_surface = mux.get_tab(host_surface_id).ok_or("host surface missing")?;
        let session_id = host_surface_session_id(&host_surface).ok_or("surface session id missing")?;
        let active_host_leaf = host_surface
            .get_active_pane()
            .ok_or("surface has no active pane")?;
        let pane_tree = host_surface.codec_pane_tree();
        let mut state = SessionSurfaceState::detached(session_id, surface_id);
        if let Some(layout) = pane_leaf_id(&active_host_leaf)
            .map(|leaf_id| {
                crate::SessionLayoutSnapshot::single_leaf(
                    metadata_layout_node_id(surface_id, leaf_id),
                    leaf_id,
                    Some(active_host_leaf.get_title()),
                )
            })
            .or_else(|| {
                build_layout_snapshot_from_engine(
                    host_surface_id,
                    active_host_leaf.pane_id() as u64,
                    &pane_tree,
                )
            })
        {
            state.attach_layout(layout);
        }
        Ok(state)
    }
}

fn pane_session_id(pane: &Arc<dyn Pane>) -> Option<String> {
    pane_metadata_string(pane, "chatminal_session_id")
}

fn host_surface_session_id(host_surface: &Arc<Tab>) -> Option<String> {
    for positioned in host_surface.iter_panes() {
        if let Some(session_id) = pane_session_id(&positioned.pane) {
            return Some(session_id);
        }
    }
    None
}

fn public_surface_id_for_host_surface(host_surface: &Arc<Tab>) -> SurfaceId {
    for positioned in host_surface.iter_panes() {
        if let Some(surface_id) = pane_surface_id(&positioned.pane) {
            return surface_id;
        }
    }
    SurfaceId::new(host_surface.tab_id() as u64)
}

fn host_surface_id_from_public_surface(surface_id: SurfaceId) -> Result<TabId, &'static str> {
    usize::try_from(surface_id.as_u64())
        .map(|value| value as TabId)
        .map_err(|_| "surface id out of range")
}

fn pane_surface_id(pane: &Arc<dyn Pane>) -> Option<SurfaceId> {
    pane_metadata_u64(pane, "chatminal_surface_id").map(SurfaceId::new)
}

fn pane_leaf_id(pane: &Arc<dyn Pane>) -> Option<LeafId> {
    pane_metadata_u64(pane, "chatminal_leaf_id").map(LeafId::new)
}

fn pane_metadata_string(pane: &Arc<dyn Pane>, key: &str) -> Option<String> {
    match pane.get_metadata() {
        Value::Object(obj) => match obj.get(&Value::String(key.to_string())) {
            Some(Value::String(value)) if !value.trim().is_empty() => Some(value.to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn pane_metadata_u64(pane: &Arc<dyn Pane>, key: &str) -> Option<u64> {
    match pane.get_metadata() {
        Value::Object(obj) => obj
            .get(&Value::String(key.to_string()))
            .and_then(Value::coerce_unsigned),
        _ => None,
    }
}

fn metadata_layout_node_id(surface_id: SurfaceId, leaf_id: LeafId) -> crate::LayoutNodeId {
    crate::LayoutNodeId::new((surface_id.as_u64() << 32) | (leaf_id.as_u64() & 0xffff_ffff))
}
