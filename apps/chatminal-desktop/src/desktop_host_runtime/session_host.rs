// Desktop session host: manages session runtime + host leaf lifecycle for one desktop window.
//
// This is the session-native render path (Phase 03+). The host creates `ChatminalSessionPane`
// objects directly from the session engine's core state and builds `ChatminalRenderState`
// from the session_pane map.

use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex, OnceLock};

use super::session_engine::{
    TerminalInstanceId, SessionEngineShared, SessionRuntimeState, StatefulSessionEngine, RuntimeId,
};
use chatminal_terminal_core::TerminalSize as CoreTerminalSize;
use config::keyassignment::SessionDirection;
use config::keyassignment::SpawnSessionDomain;
use config::TermConfig;
use engine_term::TerminalSize;
use portable_pty::CommandBuilder;

use super::session_pane::ChatminalSessionPane;
use super::{
    DesktopEngineRuntimeAdapter, HostDomainId as DomainId, HostMux,
    HostTerminal, HostTerminalHandle, HostRenderableDimensions as RenderableDimensions,
    HostSplitSource as SplitSource,
    RuntimeSplitRequest as SplitRequest, RuntimeWindowId as EngineWindowId,
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

pub(crate) fn session_host(window_id: EngineWindowId) -> Option<Arc<DesktopSessionHost>> {
    host_registry().lock().unwrap().get(&window_id).cloned()
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
            runtime_render_state: Mutex::new(HashMap::new()),
        }
    }

    fn engine(&self) -> StatefulSessionEngine<()> {
        StatefulSessionEngine::with_shared((), Arc::clone(&self.shared))
    }

    fn mux_engine(&self) -> StatefulSessionEngine<DesktopEngineRuntimeAdapter> {
        StatefulSessionEngine::with_shared(
            DesktopEngineRuntimeAdapter::new(self.window_id),
            Arc::clone(&self.shared),
        )
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
        let command = crate::chatminal_runtime::runtime_proxy_command(Some(session_id));
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

    pub(crate) fn close_leaf(&self, session_id: &str, runtime_id: RuntimeId, terminal_instance_id: TerminalInstanceId) -> bool {
        let pane_id = self
            .panes
            .lock()
            .unwrap()
            .get(&terminal_instance_id)
            .map(|pane| pane.pane_id_value());
        if !self
            .engine()
            .close_leaf_native(session_id, runtime_id, terminal_instance_id)
        {
            return false;
        }

        if let Some(pane_id) = pane_id {
            HostMux::get().remove_pane(pane_id);
        }

        if let Some(state) = self.snapshot_runtime_from_host(session_id, runtime_id) {
            self.sync_render_state_for_runtime(&state);
        } else if let Some(state) = self.engine().snapshot_runtime_from_core(runtime_id) {
            self.sync_render_state_for_runtime(&state);
        } else {
            self.remove_runtime_resources(runtime_id);
        }

        true
    }

    /// Close a session runtime attachment and unregister all associated panes/tabs.
    pub(crate) fn close_runtime(&self, session_id: &str, runtime_id: RuntimeId) {
        let _ = session_id;
        self.engine().close_runtime_native(session_id, runtime_id);
        self.remove_runtime_resources(runtime_id);
    }

    pub(crate) fn sync_runtime_from_host(&self, session_id: &str) -> Option<SessionRuntimeState> {
        let runtime_id = self.runtime_id_for_session(session_id)?;
        let state = self.snapshot_runtime_from_host(session_id, runtime_id)?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    pub(crate) fn focus_direction(
        &self,
        session_id: &str,
        direction: SessionDirection,
    ) -> Option<SessionRuntimeState> {
        let state = self
            .mux_engine()
            .activate_session_direction(session_id, direction)
            .map_err(|err| {
                log::error!(
                    "session host: activate_session_direction failed for {session_id}: {err}"
                );
            })
            .ok()
            .flatten()?;
        self.sync_render_state_for_runtime(&state);
        Some(state)
    }

    pub(crate) fn swap_active_with_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
        keep_focus: bool,
    ) -> bool {
        let Some(state) = self
            .mux_engine()
            .swap_active_with_session_terminal_instance(session_id, terminal_instance_id, keep_focus)
            .map_err(|err| {
                log::error!(
                    "session host: swap_active_with_session_terminal_instance failed for {session_id}: {err}"
                );
            })
            .ok()
        else {
            return false;
        };
        self.sync_render_state_for_runtime(&state);
        true
    }

    pub(crate) fn move_terminal_instance_to_new_window(&self, session_id: &str, terminal_instance_id: TerminalInstanceId) -> bool {
        self.move_terminal_instance_to_target_window(session_id, terminal_instance_id, None)
    }

    pub(crate) fn move_terminal_instance_to_new_runtime(&self, session_id: &str, terminal_instance_id: TerminalInstanceId) -> bool {
        self.move_terminal_instance_to_target_window(session_id, terminal_instance_id, Some(self.window_id))
    }

    pub async fn split_terminal_instance(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
        request: SplitRequest,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        domain: SpawnSessionDomain,
        term_config: Arc<TermConfig>,
    ) -> anyhow::Result<()> {
        let Some(terminal_handle) = self.terminal_handle_for_leaf(terminal_instance_id) else {
            anyhow::bail!("leaf {terminal_instance_id} missing from desktop session host");
        };
        let (pane, _size) = HostMux::get()
            .split_pane(
                terminal_handle,
                request,
                SplitSource::Spawn {
                    command,
                    command_dir,
                },
                domain,
            )
            .await?;
        pane.set_config(term_config);
        self.sync_runtime_from_host(session_id)
            .ok_or_else(|| anyhow::anyhow!("failed to sync session runtime after split"))?;
        Ok(())
    }

    // -----------------------------------------------------------------
    // Active session tracking (Phase 06)
    // -----------------------------------------------------------------

    fn leaf_count_for_runtime(&self, runtime_id: RuntimeId) -> Option<usize> {
        self.shared
            .core_state()
            .lock()
            .unwrap()
            .runtime(runtime_id)
            .and_then(|runtime| runtime.layout.as_ref().map(|layout| layout.leaves.len()))
            .or_else(|| {
                Some(
                    self.panes
                        .lock()
                        .unwrap()
                        .values()
                        .filter(|pane| pane.runtime_id_value() == runtime_id)
                        .count(),
                )
            })
    }

    // -----------------------------------------------------------------
    // Lookup helpers
    // -----------------------------------------------------------------

    fn runtime_id_for_session(&self, session_id: &str) -> Option<RuntimeId> {
        self.shared
            .core_state()
            .lock()
            .unwrap()
            .runtime_id_for_session(session_id)
    }

    pub(crate) fn render_state_for_runtime(&self, runtime_id: RuntimeId) -> Option<ChatminalRenderState> {
        self.runtime_render_state
            .lock()
            .unwrap()
            .get(&runtime_id)
            .cloned()
    }

    pub async fn split_terminal_handle(
        &self,
        terminal_handle: u64,
        request: SplitRequest,
        command: Option<CommandBuilder>,
        command_dir: Option<String>,
        domain: SpawnSessionDomain,
        term_config: Arc<TermConfig>,
    ) -> anyhow::Result<()> {
        let terminal_handle = HostTerminalHandle::try_from(terminal_handle)
            .map_err(|_| anyhow::anyhow!("invalid terminal handle {terminal_handle}"))?;
        let Some((session_id, terminal_instance_id)) = self.terminal_handle_context(terminal_handle) else {
            anyhow::bail!("terminal handle {terminal_handle} is not a chatminal session pane");
        };
        self.split_terminal_instance(
            &session_id,
            terminal_instance_id,
            request,
            command,
            command_dir,
            domain,
            term_config,
        )
        .await
    }

    pub(crate) fn move_terminal_handle_to_new_window(&self, terminal_handle: u64) -> bool {
        let Ok(terminal_handle) = HostTerminalHandle::try_from(terminal_handle) else {
            return false;
        };
        let Some((session_id, terminal_instance_id)) = self.terminal_handle_context(terminal_handle) else {
            return false;
        };
        self.move_terminal_instance_to_new_window(&session_id, terminal_instance_id)
    }

    pub(crate) fn move_terminal_handle_to_new_runtime(&self, terminal_handle: u64) -> bool {
        let Ok(terminal_handle) = HostTerminalHandle::try_from(terminal_handle) else {
            return false;
        };
        let Some((session_id, terminal_instance_id)) = self.terminal_handle_context(terminal_handle) else {
            return false;
        };
        self.move_terminal_instance_to_new_runtime(&session_id, terminal_instance_id)
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

        // Remove panes for leaves no longer in the layout
        let live_terminal_instance_ids: std::collections::HashSet<TerminalInstanceId> =
            layout.leaves.iter().map(|l| l.terminal_instance_id).collect();
        let stale: Vec<TerminalInstanceId> = panes_guard
            .keys()
            .copied()
            .filter(|id| !live_terminal_instance_ids.contains(id))
            .collect();
        for stale_terminal_instance_id in stale {
            if let Some(stale_pane) = panes_guard.remove(&stale_terminal_instance_id) {
                HostMux::get().remove_pane(stale_pane.pane_id_value());
            }
        }

        // Capture active pane before dropping panes_guard (avoids double-lock)
        let active_pane_for_session = panes_guard
            .get(&layout.active_terminal_instance_id)
            .cloned();

        drop(panes_guard);

        // Sync session_pane index: 1 session = 1 pane invariant
        if let Some(active_pane) = active_pane_for_session {
            let mut session_pane_guard = self.session_pane.lock().unwrap();
            if let Some(existing) = session_pane_guard.get(session_id.as_str()) {
                if existing.pane_id_value() != active_pane.pane_id_value() {
                    debug_assert!(false, "invariant violated: 1 session = 1 pane for session {:?}", session_id);
                    log::error!("session host: invariant violated — multiple panes for session {session_id}; skipping overwrite");
                }
            } else {
                session_pane_guard.insert(session_id.to_string(), active_pane.clone());
            }

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
                // Also remove from session_pane index
                let session_id = pane.session_id_value().to_string();
                self.session_pane.lock().unwrap().remove(&session_id);
                HostMux::get().remove_pane(pane.pane_id_value());
            }
        }
    }

    /// Look up the pane for a session (1 session = 1 pane invariant).
    pub(crate) fn pane_for_session(&self, session_id: &str) -> Option<Arc<ChatminalSessionPane>> {
        self.session_pane.lock().unwrap().get(session_id).cloned()
    }

    fn move_terminal_instance_to_target_window(
        &self,
        session_id: &str,
        terminal_instance_id: TerminalInstanceId,
        target_window_id: Option<EngineWindowId>,
    ) -> bool {
        let Some(runtime_id) = self.runtime_id_for_session(session_id) else {
            return false;
        };
        let Some(leaf_count) = self.leaf_count_for_runtime(runtime_id) else {
            return false;
        };
        if leaf_count != 1 {
            log::warn!(
                "session host: refusing to move leaf {terminal_instance_id} for session {session_id}; multi-leaf move is not session-native yet"
            );
            return false;
        }

        let Some(terminal_handle) = self
            .panes
            .lock()
            .unwrap()
            .get(&terminal_instance_id)
            .map(|pane| pane.pane_id_value())
        else {
            return false;
        };

        let window_id = self.window_id;
        let session_id = session_id.to_string();
        promise::spawn::spawn(async move {
            let mux = HostMux::get();
            if let Err(err) = mux
                .move_pane_to_new_tab(terminal_handle, target_window_id, None)
                .await
            {
                log::error!("failed to move session leaf {terminal_instance_id}: {err:#}");
                return;
            }

            if let Some(host) = session_host(window_id) {
                let _ = host.sync_runtime_from_host(&session_id);
            }
        })
        .detach();

        true
    }

    fn snapshot_runtime_from_host(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Option<SessionRuntimeState> {
        self.mux_engine()
            .refresh_runtime_state_from_adapter(runtime_id)
            .ok()
            .filter(|state| state.snapshot.session_id == session_id)
    }

    fn terminal_handle_for_leaf(&self, terminal_instance_id: TerminalInstanceId) -> Option<HostTerminalHandle> {
        self.panes
            .lock()
            .unwrap()
            .get(&terminal_instance_id)
            .map(|pane| pane.pane_id_value())
    }

    fn terminal_handle_context(&self, terminal_handle: HostTerminalHandle) -> Option<(String, TerminalInstanceId)> {
        self.panes
            .lock()
            .unwrap()
            .values()
            .find(|pane| pane.pane_id_value() == terminal_handle)
            .map(|pane| (pane.session_id_value().to_string(), pane.terminal_instance_id_value()))
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
