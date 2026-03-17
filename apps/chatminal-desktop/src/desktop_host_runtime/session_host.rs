// Desktop session host: manages session runtime + host leaf lifecycle for one desktop window.
//
// This is the session-native render path (Phase 03+). The host creates `ChatminalSessionPane`
// objects directly from the session engine's core state and builds `ChatminalRenderState`
// from the session_pane map.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::session_engine::{
    TerminalInstanceId, SessionEngineShared, SessionRuntimeState, StatefulSessionEngine, RuntimeId,
};
use chatminal_terminal_core::TerminalSize as CoreTerminalSize;
use engine_term::TerminalSize;
use portable_pty::CommandBuilder;

use super::session_pane::ChatminalSessionPane;
use super::{
    HostDomainId as DomainId, HostMux, HostRenderScope, HostTerminal,
    HostRenderableDimensions as RenderableDimensions, RuntimeWindowId as EngineWindowId,
};
use crate::chatminal_render::{ChatminalRenderPane, ChatminalRenderState};
use crate::chatminal_runtime::{SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle};

// ---------------------------------------------------------------------------
// Per-window host registry
// ---------------------------------------------------------------------------

static HOST_REGISTRY: OnceLock<Mutex<HashMap<EngineWindowId, Arc<DesktopSessionHost>>>> =
    OnceLock::new();

fn host_registry() -> &'static Mutex<HashMap<EngineWindowId, Arc<DesktopSessionHost>>> {
    HOST_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn get_or_init_session_host(
    window_id: EngineWindowId,
    domain_id: DomainId,
    shared: Arc<SessionEngineShared>,
) -> Arc<DesktopSessionHost> {
    let mut registry = host_registry().lock().unwrap();
    registry
        .entry(window_id)
        .or_insert_with(|| Arc::new(DesktopSessionHost::new(window_id, domain_id, shared)))
        .clone()
}

// ---------------------------------------------------------------------------
// DesktopSessionHost
// ---------------------------------------------------------------------------

pub(crate) struct DesktopSessionHost {
    window_id: EngineWindowId,
    domain_id: DomainId,
    shared: Arc<SessionEngineShared>,
    // terminal_instance_id → pane (for output/input routing)
    panes: Mutex<HashMap<TerminalInstanceId, Arc<ChatminalSessionPane>>>,
    // session_id → pane (1 session = 1 pane invariant)
    session_pane: Mutex<HashMap<String, Arc<ChatminalSessionPane>>>,
    // session_id → mux tab_id for the shim tab (desktop-local; replaces Mux global index)
    session_tab_shim: Mutex<HashMap<String, usize>>,
    // runtime_id → first-party render snapshot for termwindow compatibility
    runtime_render_state: Mutex<HashMap<RuntimeId, ChatminalRenderState>>,
}

impl DesktopSessionHost {
    fn new(
        window_id: EngineWindowId,
        domain_id: DomainId,
        shared: Arc<SessionEngineShared>,
    ) -> Self {
        Self {
            window_id,
            domain_id,
            shared,
            panes: Mutex::new(HashMap::new()),
            session_pane: Mutex::new(HashMap::new()),
            session_tab_shim: Mutex::new(HashMap::new()),
            runtime_render_state: Mutex::new(HashMap::new()),
        }
    }

    fn engine(&self) -> StatefulSessionEngine {
        StatefulSessionEngine::with_shared(Arc::clone(&self.shared))
    }

    // -----------------------------------------------------------------
    // Runtime lifecycle
    // -----------------------------------------------------------------

    /// Ensure a session runtime attachment exists: focus it if it already exists, or
    /// spawn a new one. Creates/updates the host render-scope wrapper for termwindow.
    ///
    /// Returns the session runtime state (contains layout snapshot), or `None`
    /// on failure.
    pub(crate) fn ensure_runtime(
        &self,
        session_id: &str,
        generation: u64,
        command: CommandBuilder,
        size: TerminalSize,
    ) -> Option<SessionRuntimeState> {
        self.ensure_runtime_inner(session_id, generation, command, size, true)
    }

    pub(crate) fn attach_layout_session(
        &self,
        session_id: &str,
        size: TerminalSize,
        activate: bool,
    ) -> Option<SessionRuntimeState> {
        let command = super::native_session_command(session_id).ok()?;
        self.ensure_runtime_inner(session_id, 0, command, size, activate)
    }

    fn ensure_runtime_inner(
        &self,
        session_id: &str,
        generation: u64,
        command: CommandBuilder,
        size: TerminalSize,
        activate: bool,
    ) -> Option<SessionRuntimeState> {
        let core_size = core_terminal_size(size);
        let state = self
            .engine()
            .ensure_session_runtime_native(session_id, generation, command, core_size)
            .map_err(|err| {
                log::error!("session host: ensure runtime failed for {session_id}: {err}");
            })
            .ok()?;

        self.sync_render_state_for_runtime(&state);
        let _ = activate;
        Some(state)
    }

    /// Focus an already-existing session runtime attachment. Returns the runtime state or
    /// `None` if the runtime is not found in core state.
    pub(crate) fn focus_runtime(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Option<SessionRuntimeState> {
        let state = self.engine().focus_runtime_native(session_id, runtime_id)?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    /// Focus a specific leaf. Returns the updated runtime snapshot.
    pub(crate) fn focus_terminal_instance(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<SessionRuntimeState> {
        let state = self
            .engine()
            .focus_terminal_instance_native(session_id, runtime_id, terminal_instance_id)?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    /// Close a session runtime attachment and unregister all associated panes/tabs.
    pub(crate) fn close_runtime(&self, session_id: &str, runtime_id: RuntimeId) {
        let _ = session_id;
        self.engine().close_runtime_native(session_id, runtime_id);
        self.remove_runtime_resources(runtime_id);
    }

    pub(crate) fn render_state_for_runtime(&self, runtime_id: RuntimeId) -> Option<ChatminalRenderState> {
        self.runtime_render_state
            .lock()
            .unwrap()
            .get(&runtime_id)
            .cloned()
    }

    // -----------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------

    /// Create or update panes for each leaf in the state, then ensure a render tab
    /// shim exists with the current active pane.
    fn sync_render_state_for_runtime(&self, state: &SessionRuntimeState) {
        let Some(layout) = &state.layout else {
            return;
        };
        let runtime_id = state.snapshot.runtime_id;
        let session_id = &state.snapshot.session_id;

        let mut panes_guard = self.panes.lock().unwrap();

        // Create panes for any leaf that doesn't have one yet
        let mux_size = engine_terminal_size_default();
        for leaf_snapshot in &layout.leaves {
            let terminal_instance_id = leaf_snapshot.terminal_instance_id;
            if !panes_guard.contains_key(&terminal_instance_id) {
                match ChatminalSessionPane::new(
                    Arc::clone(&self.shared),
                    self.domain_id,
                    session_id.clone(),
                    runtime_id,
                    terminal_instance_id,
                    mux_size,
                ) {
                    Ok(pane) => {
                        // Register with Mux for render compat
                        if let Err(err) =
                            HostMux::get().add_pane(&(pane.clone() as Arc<dyn HostTerminal>))
                        {
                            log::warn!("session host: could not register pane {terminal_instance_id}: {err}");
                        }
                        panes_guard.insert(terminal_instance_id, pane);
                    }
                    Err(err) => {
                        log::error!("session host: create pane for leaf {terminal_instance_id}: {err}");
                    }
                }
            }
        }

        // Capture active pane before dropping panes_guard (avoids double-lock)
        let active_pane_for_session = panes_guard
            .get(&layout.active_terminal_instance_id)
            .cloned();

        // Defer stale-pane cleanup until after we have attached a live tab shim
        // to the host window. Otherwise the bootstrap pane can be removed first,
        // making the mux window empty and causing it to be pruned before the
        // session-native tab is attached.
        let live_terminal_instance_ids: std::collections::HashSet<TerminalInstanceId> =
            layout.leaves.iter().map(|l| l.terminal_instance_id).collect();
        let stale: Vec<TerminalInstanceId> = panes_guard
            .keys()
            .copied()
            .filter(|id| !live_terminal_instance_ids.contains(id))
            .collect();

        drop(panes_guard);

        // Sync session_pane index: 1 session = 1 pane invariant
        if let Some(active_pane) = active_pane_for_session {
            let mut session_pane_guard = self.session_pane.lock().unwrap();
            if session_pane_guard
                .get(session_id.as_str())
                .is_some_and(|existing| existing.pane_id_value() != active_pane.pane_id_value())
            {
                log::warn!(
                    "session host: replacing stale pane mapping for session {session_id}"
                );
            }
            session_pane_guard.insert(session_id.to_string(), active_pane.clone());
            self.ensure_mux_tab_shim(session_id, &active_pane);

            // Build ChatminalRenderState directly from session_pane — no HostRenderScope needed.
            // 1 session = 1 pane invariant: panes has exactly one element; splits = [] (splits are
            // at workspace layout level, not session level).
            let pane_id = active_pane.pane_id_value();
            let dims = active_pane.get_dimensions();
            let terminal_size = terminal_size_from_dims(dims);
            let render_state = ChatminalRenderState {
                render_target: SessionRenderTargetSnapshot {
                    render_target_id: SessionRenderTargetId::new(runtime_id.as_u64()),
                    runtime_id,
                    active_terminal_instance_id: Some(layout.active_terminal_instance_id),
                },
                terminal_size,
                active_terminal_instance_id: Some(layout.active_terminal_instance_id),
                panes: vec![ChatminalRenderPane {
                    terminal_handle: SessionTerminalHandle::new(pane_id as u64),
                    terminal_instance_id: layout.active_terminal_instance_id,
                    index: 0,
                    is_active: true,
                    is_zoomed: false,
                    left: 0,
                    top: 0,
                    width: terminal_size.cols as usize,
                    pixel_width: 0,
                    height: terminal_size.rows as usize,
                    pixel_height: 0,
                }],
                splits: vec![],
            };
            self.runtime_render_state
                .lock()
                .unwrap()
                .insert(runtime_id, render_state);
        }

        if !stale.is_empty() {
            let mut panes_guard = self.panes.lock().unwrap();
            for stale_terminal_instance_id in stale {
                if let Some(stale_pane) = panes_guard.remove(&stale_terminal_instance_id) {
                    HostMux::get().remove_pane(stale_pane.pane_id_value());
                }
            }
        }
    }

    fn ensure_mux_tab_shim(&self, session_id: &str, active_pane: &Arc<ChatminalSessionPane>) {
        let mux = HostMux::get();
        let active_pane_id = active_pane.pane_id_value();
        let title = active_pane.get_title();

        // Desktop-local lookup: use session_tab_shim instead of Mux global index.
        let existing_tab_id = self.session_tab_shim.lock().unwrap().get(session_id).copied();
        let existing = existing_tab_id
            .and_then(|tab_id| mux.get_tab(tab_id))
            .filter(|tab| mux.window_containing_tab(tab.tab_id()) == Some(self.window_id));

        let tab = match existing {
            Some(tab) => {
                let active_matches = tab
                    .get_active_pane()
                    .is_some_and(|pane| pane.pane_id() == active_pane_id);
                if active_matches {
                    tab
                } else {
                    let replacement = Arc::new(HostRenderScope::new(&engine_terminal_size_default()));
                    replacement.assign_pane(&(active_pane.clone() as Arc<dyn HostTerminal>));
                    replacement.set_title(&title);
                    if let Err(err) = mux.add_tab_and_active_pane(&replacement) {
                        log::warn!(
                            "session host: failed to register replacement tab shim for session {session_id}: {err}"
                        );
                        return;
                    }
                    if let Err(err) = mux.add_tab_to_window(&replacement, self.window_id) {
                        log::warn!(
                            "session host: failed to attach replacement tab shim for session {session_id} to window {}: {err}",
                            self.window_id
                        );
                        let _ = mux.remove_tab(replacement.tab_id());
                        return;
                    }
                    let replacement_tab_id = replacement.tab_id();
                    let _ = mux.remove_tab(tab.tab_id());
                    self.session_tab_shim.lock().unwrap().insert(session_id.to_string(), replacement_tab_id);
                    replacement
                }
            }
            None => {
                let tab = Arc::new(HostRenderScope::new(&engine_terminal_size_default()));
                tab.assign_pane(&(active_pane.clone() as Arc<dyn HostTerminal>));
                tab.set_title(&title);
                if let Err(err) = mux.add_tab_and_active_pane(&tab) {
                    log::warn!(
                        "session host: failed to register tab shim for session {session_id}: {err}"
                    );
                    return;
                }
                if let Err(err) = mux.add_tab_to_window(&tab, self.window_id) {
                    log::warn!(
                        "session host: failed to attach tab shim for session {session_id} to window {}: {err}",
                        self.window_id
                    );
                    let _ = mux.remove_tab(tab.tab_id());
                    return;
                }
                let new_tab_id = tab.tab_id();
                self.session_tab_shim.lock().unwrap().insert(session_id.to_string(), new_tab_id);
                tab
            }
        };

        tab.set_title(&title);
        {
            let maybe_window = mux.get_window_mut(self.window_id);
            if let Some(mut window) = maybe_window {
                if let Some(idx) = window.idx_by_id(tab.tab_id()) {
                    window.set_active_without_saving(idx);
                }
            }
        }
    }

    /// Remove panes and render snapshot for a runtime from all registries.
    fn remove_runtime_resources(&self, runtime_id: RuntimeId) {
        self.runtime_render_state.lock().unwrap().remove(&runtime_id);
        let stale_terminal_instance_ids: Vec<TerminalInstanceId> = self
            .panes
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(terminal_instance_id, pane)| {
                (pane.runtime_id_value() == runtime_id).then_some(*terminal_instance_id)
            })
            .collect();
        let mut panes = self.panes.lock().unwrap();
        for terminal_instance_id in stale_terminal_instance_ids {
            if let Some(pane) = panes.remove(&terminal_instance_id) {
                // Also remove from session_pane and session_tab_shim indexes
                let session_id = pane.session_id_value().to_string();
                self.session_pane.lock().unwrap().remove(&session_id);
                self.session_tab_shim.lock().unwrap().remove(&session_id);
                HostMux::get().remove_pane(pane.pane_id_value());
            }
        }
    }

    /// Look up the pane for a session (1 session = 1 pane invariant).
    pub(crate) fn pane_for_session(&self, session_id: &str) -> Option<Arc<ChatminalSessionPane>> {
        self.session_pane.lock().unwrap().get(session_id).cloned()
    }

}

// ---------------------------------------------------------------------------
// Size conversion helpers
// ---------------------------------------------------------------------------

fn core_terminal_size(size: TerminalSize) -> CoreTerminalSize {
    CoreTerminalSize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: size.pixel_width,
        pixel_height: size.pixel_height,
        dpi: size.dpi,
    }
}

fn engine_terminal_size_default() -> TerminalSize {
    TerminalSize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
        dpi: 96,
    }
}

fn terminal_size_from_dims(dims: RenderableDimensions) -> TerminalSize {
    TerminalSize {
        rows: dims.viewport_rows.max(1),
        cols: dims.cols.max(1),
        pixel_width: dims.pixel_width,
        pixel_height: dims.pixel_height,
        dpi: dims.dpi.max(1),
    }
}
