// Desktop session host: manages session runtime + host leaf lifecycle for one desktop window.
//
// This is the session-native render path (Phase 03+). The host creates `ChatminalSessionPane`
// objects directly from the session engine's core state and builds `ChatminalRenderState`
// from the session_pane map.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use super::session_engine::{
    RuntimeId, SessionEngineShared, SessionRuntimeState, StatefulSessionEngine, TerminalInstanceId,
};
use chatminal_terminal_core::TerminalSize as CoreTerminalSize;
use engine_term::TerminalSize;
use portable_pty::CommandBuilder;

use super::session_pane::{pane_id_for_terminal_instance, ChatminalSessionPane};
use super::{
    host_window_exists, HostMux, HostRenderScope,
    HostRenderableDimensions as RenderableDimensions, HostTerminal,
};
use crate::chatminal_render::{ChatminalRenderPane, ChatminalRenderState};
use crate::chatminal_runtime::{
    SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle,
};

// ---------------------------------------------------------------------------
// Singleton host registry
// ---------------------------------------------------------------------------

static HOST_REGISTRY: OnceLock<Arc<DesktopSessionHost>> = OnceLock::new();

pub(crate) fn get_or_init_session_host(shared: Arc<SessionEngineShared>) -> Arc<DesktopSessionHost> {
    HOST_REGISTRY
        .get_or_init(|| Arc::new(DesktopSessionHost::new(shared)))
        .clone()
}

// ---------------------------------------------------------------------------
// DesktopSessionHost
// ---------------------------------------------------------------------------

pub(crate) struct DesktopSessionHost {
    shared: Arc<SessionEngineShared>,
    // terminal_instance_id → pane (for output/input routing)
    panes: Mutex<HashMap<TerminalInstanceId, Arc<ChatminalSessionPane>>>,
    // session_id → pane (1 session = 1 pane invariant)
    session_pane: Mutex<HashMap<String, Arc<ChatminalSessionPane>>>,
    // session_id → mux tab_id for the shim tab (desktop-local; replaces Mux global index)
    session_tab_shim: Mutex<HashMap<String, usize>>,
    // runtime_id → first-party render snapshot for termwindow compatibility
    runtime_render_state: Mutex<HashMap<RuntimeId, ChatminalRenderState>>,
    // runtime_id → terminal instances owned by that runtime
    runtime_terminal_instances: Mutex<HashMap<RuntimeId, HashSet<TerminalInstanceId>>>,
    // runtime_id → last terminal size confirmed for the live PTY
    runtime_terminal_size: Mutex<HashMap<RuntimeId, TerminalSize>>,
}

impl DesktopSessionHost {
    fn new(shared: Arc<SessionEngineShared>) -> Self {
        Self {
            shared,
            panes: Mutex::new(HashMap::new()),
            session_pane: Mutex::new(HashMap::new()),
            session_tab_shim: Mutex::new(HashMap::new()),
            runtime_render_state: Mutex::new(HashMap::new()),
            runtime_terminal_instances: Mutex::new(HashMap::new()),
            runtime_terminal_size: Mutex::new(HashMap::new()),
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
        let initial_scrollback = crate::chatminal_runtime::read_session_restore_snapshot(session_id)
        .ok()
        .map(|snapshot| snapshot.content)
        .filter(|content| !content.is_empty());
        let state = self
            .engine()
            .ensure_session_runtime_native(
                session_id,
                generation,
                command,
                core_size,
                initial_scrollback,
            )
            .map_err(|err| {
                log::error!("session host: ensure runtime failed for {session_id}: {err}");
            })
            .ok()?;

        self.runtime_terminal_size
            .lock()
            .unwrap()
            .insert(state.snapshot.runtime_id, size);
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

    /// Hydrate render objects for an existing runtime without changing focus.
    pub(crate) fn hydrate_runtime(&self, runtime_id: RuntimeId) -> Option<SessionRuntimeState> {
        let state = self.engine().snapshot_runtime_from_core(runtime_id)?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    pub(crate) fn remember_runtime_terminal_size(&self, runtime_id: RuntimeId, size: TerminalSize) {
        self.runtime_terminal_size
            .lock()
            .unwrap()
            .insert(runtime_id, size);
    }

    /// Focus a specific leaf. Returns the updated runtime snapshot.
    pub(crate) fn focus_terminal_instance(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<SessionRuntimeState> {
        let state = self.engine().focus_terminal_instance_native(
            session_id,
            runtime_id,
            terminal_instance_id,
        )?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    pub(crate) fn resize_runtime(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        size: TerminalSize,
    ) -> Option<SessionRuntimeState> {
        let pane = self.pane_for_session(session_id)?;
        let known_size = self
            .runtime_terminal_size
            .lock()
            .unwrap()
            .get(&runtime_id)
            .copied();
        let needs_resize = known_size.map_or_else(
            || {
                let dims = pane.get_dimensions();
                dims.cols != size.cols
                    || dims.viewport_rows != size.rows
                    || dims.pixel_width != size.pixel_width
                    || dims.pixel_height != size.pixel_height
                    || dims.dpi != size.dpi
            },
            |known| {
                known.cols != size.cols
                    || known.rows != size.rows
                    || known.pixel_width != size.pixel_width
                    || known.pixel_height != size.pixel_height
                    || known.dpi != size.dpi
            },
        );
        if needs_resize {
            let dims = pane.get_dimensions();
            log::warn!(
                "resize_runtime: session={} runtime={} from cols={} rows={} px={}x{} dpi={} to cols={} rows={} px={}x{} dpi={}",
                session_id,
                runtime_id.as_u64(),
                dims.cols,
                dims.viewport_rows,
                dims.pixel_width,
                dims.pixel_height,
                dims.dpi,
                size.cols,
                size.rows,
                size.pixel_width,
                size.pixel_height,
                size.dpi
            );
            if let Err(err) = pane.resize(size) {
                log::error!("session host: resize runtime failed for {session_id}: {err}");
                return None;
            }
        }
        self.runtime_terminal_size
            .lock()
            .unwrap()
            .insert(runtime_id, size);
        let state = self.engine().snapshot_runtime_from_core(runtime_id)?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    /// Close a session runtime attachment and unregister all associated panes/tabs.
    pub(crate) fn close_runtime(&self, session_id: &str, runtime_id: RuntimeId) {
        let _ = session_id;
        self.engine().close_runtime_native(session_id, runtime_id);
        self.remove_runtime_resources(runtime_id);
    }

    pub(crate) fn render_state_for_runtime(
        &self,
        runtime_id: RuntimeId,
    ) -> Option<ChatminalRenderState> {
        self.runtime_render_state
            .lock()
            .unwrap()
            .get(&runtime_id)
            .cloned()
    }

    pub(crate) fn reconcile_visible_sessions(&self, visible_session_ids: &HashSet<String>) {
        let stale_session_ids: Vec<String> = self
            .session_pane
            .lock()
            .unwrap()
            .keys()
            .filter(|session_id| !visible_session_ids.contains(*session_id))
            .cloned()
            .collect();

        if stale_session_ids.is_empty() {
            return;
        }

        let mux = HostMux::get();
        let mut panes = self.panes.lock().unwrap();
        let mut session_pane = self.session_pane.lock().unwrap();
        let mut session_tab_shim = self.session_tab_shim.lock().unwrap();
        let mut runtime_render_state = self.runtime_render_state.lock().unwrap();

        for session_id in stale_session_ids {
            let stale_terminal_instance_id = session_pane
                .get(&session_id)
                .map(|pane| pane.terminal_instance_id_value());
            if let Some(tab_id) = session_tab_shim.remove(&session_id) {
                let _ = mux.remove_tab(tab_id);
            }
            if let Some(pane) = session_pane.remove(&session_id) {
                runtime_render_state.remove(&pane.runtime_id_value());
            }
            if let Some(terminal_instance_id) = stale_terminal_instance_id {
                panes.remove(&terminal_instance_id);
            }
        }
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
        let pane_size = self
            .runtime_terminal_size
            .lock()
            .unwrap()
            .get(&runtime_id)
            .copied()
            .unwrap_or_else(engine_terminal_size_default);

        let mut panes_guard = self.panes.lock().unwrap();

        // Create panes for any leaf that doesn't have one yet
        for leaf_snapshot in &layout.leaves {
            let terminal_instance_id = leaf_snapshot.terminal_instance_id;
            if !panes_guard.contains_key(&terminal_instance_id) {
                if let Some(pane) = self.adopt_existing_pane(
                    session_id,
                    runtime_id,
                    terminal_instance_id,
                ) {
                    panes_guard.insert(terminal_instance_id, pane);
                    continue;
                }

                match ChatminalSessionPane::new(
                    Arc::clone(&self.shared),
                    session_id.clone(),
                    runtime_id,
                    terminal_instance_id,
                    pane_size,
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
                        log::error!(
                            "session host: create pane for leaf {terminal_instance_id}: {err}"
                        );
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
        let live_terminal_instance_ids: HashSet<TerminalInstanceId> = layout
            .leaves
            .iter()
            .map(|l| l.terminal_instance_id)
            .collect();
        let stale: Vec<TerminalInstanceId> = self
            .runtime_terminal_instances
            .lock()
            .unwrap()
            .insert(runtime_id, live_terminal_instance_ids.clone())
            .unwrap_or_default()
            .into_iter()
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
                log::debug!("session host: replacing stale pane mapping for session {session_id}");
            }
            session_pane_guard.insert(session_id.to_string(), active_pane.clone());
            self.ensure_mux_tab_shim(session_id, &active_pane);

            // Build ChatminalRenderState directly from session_pane — no HostRenderScope needed.
            // 1 session = 1 pane invariant: panes has exactly one element; splits = [] (splits are
            // at workspace layout level, not session level).
            let pane_id = active_pane.pane_id_value();
            let terminal_size = self
                .runtime_terminal_size
                .lock()
                .unwrap()
                .get(&runtime_id)
                .copied()
                .unwrap_or_else(|| terminal_size_from_dims(active_pane.get_dimensions()));
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

    fn adopt_existing_pane(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    ) -> Option<Arc<ChatminalSessionPane>> {
        let pane_id = pane_id_for_terminal_instance(terminal_instance_id);
        let pane = HostMux::get().get_pane(pane_id)?;
        let pane = pane.downcast_arc::<ChatminalSessionPane>().ok()?;
        if pane.runtime_id_value() != runtime_id
            || pane.terminal_instance_id_value() != terminal_instance_id
            || pane.session_id_value() != session_id
        {
            return None;
        }
        log::debug!(
            "session host: adopted existing pane {} for session {} terminal_instance {}",
            pane_id as u64,
            session_id,
            terminal_instance_id.as_u64()
        );
        Some(pane)
    }

    fn ensure_mux_tab_shim(&self, session_id: &str, active_pane: &Arc<ChatminalSessionPane>) {
        if !host_window_exists() {
            log::debug!(
                "session host: deferring tab shim attach for session {session_id}; root window not ready yet",
            );
            return;
        }

        let mux = HostMux::get();
        let active_pane_id = active_pane.pane_id_value();
        let title = active_pane.get_title();

        // Desktop-local lookup: use session_tab_shim instead of Mux global index.
        let existing_tab_id = self
            .session_tab_shim
            .lock()
            .unwrap()
            .get(session_id)
            .copied();
        let existing = existing_tab_id.and_then(|tab_id| mux.get_tab(tab_id));
        let existing = existing.or_else(|| {
            let tab = mux.get_tab_by_chatminal_session_id(session_id)?;
            self.session_tab_shim
                .lock()
                .unwrap()
                .insert(session_id.to_string(), tab.tab_id());
            Some(tab)
        });

        let tab = match existing {
            Some(tab) => {
                let active_matches = tab
                    .get_active_pane()
                    .is_some_and(|pane| pane.pane_id() == active_pane_id);
                if active_matches {
                    tab
                } else {
                    let replacement =
                        Arc::new(HostRenderScope::new(&engine_terminal_size_default()));
                    replacement.assign_pane(&(active_pane.clone() as Arc<dyn HostTerminal>));
                    replacement.set_title(&title);
                    if let Err(err) = mux.add_tab_and_active_pane(&replacement) {
                        log::warn!(
                            "session host: failed to register replacement tab shim for session {session_id}: {err}"
                        );
                        return;
                    }
                    if let Err(err) = mux.attach_tab(&replacement) {
                        log::warn!(
                            "session host: failed to attach replacement tab shim for session {session_id} to root window: {err}"
                        );
                        let _ = mux.remove_tab(replacement.tab_id());
                        return;
                    }
                    let replacement_tab_id = replacement.tab_id();
                    let _ = mux.remove_tab(tab.tab_id());
                    self.session_tab_shim
                        .lock()
                        .unwrap()
                        .insert(session_id.to_string(), replacement_tab_id);
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
                if let Err(err) = mux.attach_tab(&tab) {
                    log::warn!(
                        "session host: failed to attach tab shim for session {session_id} to root window: {err}"
                    );
                    let _ = mux.remove_tab(tab.tab_id());
                    return;
                }
                let new_tab_id = tab.tab_id();
                self.session_tab_shim
                    .lock()
                    .unwrap()
                    .insert(session_id.to_string(), new_tab_id);
                tab
            }
        };

        tab.set_title(&title);
        {
            let mut window = mux.root_window_mut();
            if let Some(idx) = window.idx_by_id(tab.tab_id()) {
                window.set_active_without_saving(idx);
            }
        }
    }

    /// Remove panes and render snapshot for a runtime from all registries.
    fn remove_runtime_resources(&self, runtime_id: RuntimeId) {
        self.runtime_render_state
            .lock()
            .unwrap()
            .remove(&runtime_id);
        self.runtime_terminal_size
            .lock()
            .unwrap()
            .remove(&runtime_id);
        let stale_terminal_instance_ids: Vec<TerminalInstanceId> = self
            .runtime_terminal_instances
            .lock()
            .unwrap()
            .remove(&runtime_id)
            .unwrap_or_default()
            .into_iter()
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
