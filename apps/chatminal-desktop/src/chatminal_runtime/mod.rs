mod client;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use crate::chatminal_layout::workspace_store::{
    DesktopWorkspaceLayoutStore, DEFAULT_LAYOUT_WORKSPACE_ID,
};
use crate::chatminal_render::ChatminalRenderState;
use crate::scripting::guiwin::DesktopWindowId;
use config::keyassignment::{SessionDirection, SpawnSessionDomain};
use chatminal_runtime::{
    RuntimeCreatedSession, RuntimeEvent, RuntimeProfile, RuntimeSessionSnapshot, RuntimeWorkspace,
};

pub(crate) use client::ChatminalRuntimeClient;
pub(crate) use client::resolve_target_session_id;
pub(crate) use crate::desktop_host_runtime::*;
pub use chatminal_runtime::{
    RuntimeId, RuntimeSessionBridgeAction as DesktopSessionBridgeAction,
    RuntimeSessionLookup as DesktopSessionLookup, SessionEngineCapability,
    SessionGroupId, SessionGroupSnapshot, SessionLayoutTarget,
    SessionRenderTargetId, SessionRenderTargetSnapshot, SessionTerminalHandle,
    SessionViewBinding, SessionViewId, SessionWindowBinding, TerminalInstanceId,
    WorkspaceLayoutNodeKind, WorkspaceLayoutState, WorkspaceNodeId, WorkspaceSplitAxis,
};
pub use crate::desktop_host_runtime::session_engine::SessionEngineShared;

pub const DESKTOP_LAYOUT_WORKSPACE_ID: &str = DEFAULT_LAYOUT_WORKSPACE_ID;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopSplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopSplitSize {
    Cells(usize),
    Percent(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DesktopSplitRequest {
    pub direction: DesktopSplitDirection,
    pub target_is_second: bool,
    pub top_level: bool,
    pub size: DesktopSplitSize,
}

impl DesktopSplitRequest {
    pub(crate) fn into_host_request(self) -> RuntimeSplitRequest {
        RuntimeSplitRequest {
            direction: match self.direction {
                DesktopSplitDirection::Horizontal => RuntimeSplitDirection::Horizontal,
                DesktopSplitDirection::Vertical => RuntimeSplitDirection::Vertical,
            },
            target_is_second: self.target_is_second,
            top_level: self.top_level,
            size: match self.size {
                DesktopSplitSize::Cells(size) => RuntimeSplitSize::Cells(size),
                DesktopSplitSize::Percent(size) => RuntimeSplitSize::Percent(size),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct DesktopWorkspaceSnapshot {
    pub workspace: RuntimeWorkspace,
    pub layout: Option<WorkspaceLayoutState>,
}

pub struct DesktopWorkspaceSubscription {
    client: ChatminalRuntimeClient,
    workspace_id: String,
}

#[derive(Clone, Debug)]
pub struct DesktopSessionWindowSnapshot {
    pub window_binding: SessionWindowBinding,
    pub workspace_snapshot: DesktopWorkspaceSnapshot,
    pub lookup: DesktopSessionLookup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopSessionEntryBinding {
    pub entry_index: usize,
    pub session_id: String,
    pub title: String,
    pub view_id: Option<SessionViewId>,
    pub render_target_id: Option<SessionRenderTargetId>,
    pub is_active: bool,
    pub is_last_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopSidebarProfile {
    pub profile_id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopSidebarSession {
    pub session_id: String,
    pub name: String,
    pub status: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesktopSidebarSnapshot {
    pub active_profile_id: Option<String>,
    pub active_session_id: Option<String>,
    pub profiles: Vec<DesktopSidebarProfile>,
    pub sessions: Vec<DesktopSidebarSession>,
    pub error: Option<String>,
    pub version: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopSessionRuntimeSummary {
    pub runtime_id: RuntimeId,
    pub render_target: SessionRenderTargetSnapshot,
    pub view_binding: Option<SessionViewBinding>,
    pub group_snapshot: Option<SessionGroupSnapshot>,
    pub window_binding: SessionWindowBinding,
    pub capability: SessionEngineCapability,
}

impl DesktopSessionRuntimeSummary {
    fn from_session(window_id: DesktopWindowId, workspace_id: &str, session_id: &str, runtime_id: RuntimeId) -> Self {
        let render_target = desktop_render_state_for_session(window_id, session_id)
            .map(|state| state.render_target.clone())
            .unwrap_or(SessionRenderTargetSnapshot {
                render_target_id: SessionRenderTargetId::new(runtime_id.as_u64()),
                runtime_id,
                active_terminal_instance_id: None,
            });
        let view_binding = desktop_session_view_binding(workspace_id, session_id).ok().flatten();
        let group_snapshot = view_binding
            .as_ref()
            .map(desktop_group_snapshot_for_view_binding);
        Self {
            runtime_id,
            render_target,
            view_binding,
            group_snapshot,
            window_binding: desktop_window_binding(window_id, workspace_id),
            capability: SessionEngineCapability::desktop_embedded(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopSessionTerminalBinding {
    pub session_id: String,
    pub runtime_id: RuntimeId,
    pub terminal_instance_id: TerminalInstanceId,
    pub terminal_handle: SessionTerminalHandle,
}

static DESKTOP_LAST_ACTIVE_SESSION: OnceLock<Mutex<HashMap<DesktopWindowId, Option<String>>>> =
    OnceLock::new();

fn desktop_last_active_registry() -> &'static Mutex<HashMap<DesktopWindowId, Option<String>>> {
    DESKTOP_LAST_ACTIVE_SESSION.get_or_init(|| Mutex::new(HashMap::new()))
}

fn desktop_last_active_session(window_id: DesktopWindowId) -> Option<String> {
    desktop_last_active_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(&window_id).cloned().flatten())
}

fn remember_desktop_last_active_session(
    window_id: DesktopWindowId,
    previous_session_id: Option<String>,
    next_session_id: Option<&str>,
) {
    let Some(previous_session_id) = previous_session_id else {
        return;
    };
    if next_session_id == Some(previous_session_id.as_str()) {
        return;
    }
    if let Ok(mut registry) = desktop_last_active_registry().lock() {
        registry.insert(window_id, Some(previous_session_id));
    }
}

fn desktop_focus_layout_session(session_id: &str) {
    let store = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID);
    let _ = store.ensure_for_session(session_id);
    if let Some(view_id) = store.view_id_for_session(session_id) {
        let _ = store.focus_view(view_id);
    }
}

fn desktop_close_layout_session(session_id: &str) -> Option<String> {
    let store = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID);
    let view_id = store.view_id_for_session(session_id)?;
    let layout = store.snapshot()?;
    if layout.views.len() <= 1 {
        let _ = workspace_layout_remove(DEFAULT_LAYOUT_WORKSPACE_ID);
        return None;
    }
    let updated = store.close_view(view_id)?;
    updated
        .view(updated.active_view_id)
        .map(|view| view.session_id.clone())
}

fn desktop_group_snapshot_for_view_binding(
    binding: &SessionViewBinding,
) -> SessionGroupSnapshot {
    let group_id = binding
        .group_id
        .unwrap_or_else(|| SessionGroupId::new(binding.view_id.as_u64()));
    SessionGroupSnapshot {
        group_id,
        layout_target: SessionLayoutTarget::Group(group_id),
        view_ids: vec![binding.view_id],
        active_view_id: Some(binding.view_id),
    }
}

pub fn desktop_session_view_binding(
    workspace_id: &str,
    session_id: &str,
) -> Result<Option<SessionViewBinding>, String> {
    let layout = workspace_layout_snapshot(workspace_id)?;
    let Some(layout) = layout else {
        return Ok(None);
    };
    let Some(view_id) = layout.view_for_session(session_id) else {
        return Ok(None);
    };
    let group_id = SessionGroupId::new(view_id.as_u64());
    Ok(Some(SessionViewBinding {
        workspace_id: workspace_id.to_string(),
        session_id: session_id.to_string(),
        view_id,
        group_id: Some(group_id),
        layout_target: SessionLayoutTarget::View(view_id),
    }))
}

pub fn desktop_window_binding(
    window_id: DesktopWindowId,
    workspace_id: &str,
) -> SessionWindowBinding {
    let snapshot = load_desktop_workspace_snapshot(workspace_id).ok();
    let active_session_id = snapshot.as_ref().and_then(DesktopWorkspaceSnapshot::active_session_id);
    let active_view_id = snapshot.as_ref().and_then(DesktopWorkspaceSnapshot::active_view_id);
    let active_render_target_id = active_session_id
        .as_deref()
        .and_then(|session_id| desktop_render_state_for_session(window_id, session_id))
        .map(|state| state.render_target_id());
    SessionWindowBinding {
        window_id: window_id as u64,
        workspace_id: workspace_id.to_string(),
        active_session_id,
        active_view_id,
        active_render_target_id,
    }
}

fn desktop_terminal_binding_for_session(
    window_id: DesktopWindowId,
    session_id: &str,
    terminal_handle: SessionTerminalHandle,
) -> Option<DesktopSessionTerminalBinding> {
    let runtime_id = desktop_runtime_id_for_session(session_id)?;
    let render_state = desktop_render_state_for_session(window_id, session_id)?;
    let pane = render_state
        .panes
        .iter()
        .find(|pane| pane.terminal_handle == terminal_handle)?;
    Some(DesktopSessionTerminalBinding {
        session_id: session_id.to_string(),
        runtime_id,
        terminal_instance_id: pane.terminal_instance_id,
        terminal_handle,
    })
}

pub fn desktop_session_terminal_binding(
    window_id: DesktopWindowId,
    terminal_handle: SessionTerminalHandle,
) -> Option<DesktopSessionTerminalBinding> {
    let snapshot = load_desktop_workspace_snapshot(DEFAULT_LAYOUT_WORKSPACE_ID).ok()?;
    let active_session_id = snapshot.active_session_id();
    if let Some(binding) = active_session_id
        .as_deref()
        .and_then(|session_id| {
            desktop_terminal_binding_for_session(window_id, session_id, terminal_handle)
        })
    {
        return Some(binding);
    }
    snapshot.workspace.sessions.iter().find_map(|session| {
        desktop_terminal_binding_for_session(window_id, &session.session_id, terminal_handle)
    })
}

impl DesktopWorkspaceSnapshot {
    pub fn active_session_id(&self) -> Option<String> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.view(layout.active_view_id))
            .map(|view| view.session_id.clone())
            .or_else(|| self.workspace.active_session_id.clone())
    }

    pub fn active_view_id(&self) -> Option<SessionViewId> {
        self.layout.as_ref().map(|layout| layout.active_view_id)
    }
}

impl DesktopSessionWindowSnapshot {
    pub fn active_session_id(&self) -> Option<String> {
        self.workspace_snapshot.active_session_id()
    }

    pub fn active_view_id(&self) -> Option<SessionViewId> {
        self.workspace_snapshot.active_view_id()
    }
}

pub fn desktop_workspace_snapshot_needs_refresh(event: &RuntimeEvent) -> bool {
    matches!(
        event,
        RuntimeEvent::WorkspaceUpdated(_)
            | RuntimeEvent::SessionUpdated(_)
            | RuntimeEvent::PtyExited(_)
    )
}

pub fn load_desktop_sidebar_snapshot(
    workspace_id: &str,
) -> Result<DesktopSidebarSnapshot, String> {
    let snapshot = load_desktop_workspace_snapshot(workspace_id)?;
    let active_profile_id = snapshot.workspace.active_profile_id.clone();
    let active_session_id = snapshot.active_session_id();
    Ok(DesktopSidebarSnapshot {
        active_profile_id: active_profile_id.clone(),
        active_session_id: active_session_id.clone(),
        profiles: snapshot
            .workspace
            .profiles
            .into_iter()
            .map(|profile| DesktopSidebarProfile {
                is_active: active_profile_id.as_deref() == Some(profile.profile_id.as_str()),
                profile_id: profile.profile_id,
                name: profile.name,
            })
            .collect(),
        sessions: snapshot
            .workspace
            .sessions
            .into_iter()
            .map(|session| DesktopSidebarSession {
                is_active: active_session_id.as_deref() == Some(session.session_id.as_str()),
                session_id: session.session_id,
                name: session.name,
                status: format!("{:?}", session.status).to_lowercase(),
            })
            .collect(),
        error: None,
        version: 0,
    })
}

impl DesktopWorkspaceSubscription {
    pub fn load_sidebar_snapshot(&self) -> Result<DesktopSidebarSnapshot, String> {
        load_desktop_sidebar_snapshot(&self.workspace_id)
    }

    pub fn recv_sidebar_snapshot(
        &self,
        timeout: Duration,
    ) -> Result<Option<DesktopSidebarSnapshot>, String> {
        match self.client.recv_event(timeout)? {
            Some(event) if desktop_workspace_snapshot_needs_refresh(&event) => {
                self.load_sidebar_snapshot().map(Some)
            }
            Some(_) | None => Ok(None),
        }
    }
}

fn desktop_runtime_window_id(window_id: DesktopWindowId) -> Option<RuntimeWindowId> {
    RuntimeWindowId::try_from(window_id).ok()
}

fn embedded_runtime_arc() -> Result<Arc<EmbeddedRuntime>, String> {
    EmbeddedRuntime::global().map(Arc::clone)
}

fn session_engine_shared() -> Option<Arc<SessionEngineShared>> {
    EmbeddedRuntime::global()
        .ok()
        .map(|runtime| runtime.session_engine_shared())
}

fn desktop_runtime_id_for_session(session_id: &str) -> Option<RuntimeId> {
    session_engine_shared()?
        .core_state()
        .lock()
        .ok()?
        .runtime_id_for_session(session_id)
}

pub(crate) fn desktop_session_host(window_id: DesktopWindowId) -> Option<Arc<DesktopSessionHost>> {
    let eng_window_id = desktop_runtime_window_id(window_id)?;
    if chatminal_domain_id().is_none() {
        let command = runtime_proxy_command(None);
        let _ = ensure_chatminal_domain_for_command(&Some(command));
    }
    let shared = session_engine_shared()?;
    let domain_id = chatminal_domain_id()?;
    Some(get_or_init_session_host(eng_window_id, domain_id, shared))
}

fn desktop_prepare_host_layout(
    window_id: DesktopWindowId,
    size: engine_term::TerminalSize,
) -> Option<Arc<DesktopSessionHost>> {
    let host = desktop_session_host(window_id)?;
    let Some(layout) = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID).snapshot_or_restore() else {
        return Some(host);
    };

    let active_session_id = layout
        .view(layout.active_view_id)
        .map(|view| view.session_id.clone());

    for view in &layout.views {
        if desktop_runtime_id_for_session(&view.session_id).is_none() {
            let _ = host.attach_layout_session(&view.session_id, size, false);
        }
    }

    if let Some(active_session_id) = active_session_id {
        if let Some(runtime_id) = desktop_runtime_id_for_session(&active_session_id) {
            let _ = host.focus_runtime(&active_session_id, runtime_id);
        } else {
            let _ = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID)
                .ensure_for_session(&active_session_id);
        }
    }

    Some(host)
}

#[cfg(test)]
pub fn workspace_layout_save(layout: &WorkspaceLayoutState) -> Result<(), String> {
    embedded_runtime_arc()?.state.workspace_layout_save(layout)
}

#[cfg(test)]
pub fn workspace_layout_clear() -> Result<(), String> {
    embedded_runtime_arc()?.state.workspace_layout_clear()
}

#[cfg(test)]
pub fn store_runtime_layout(layout: &WorkspaceLayoutState) -> Result<(), String> {
    workspace_layout_save(layout)
}

#[cfg(test)]
pub fn clear_runtime_layout_state() -> Result<(), String> {
    workspace_layout_clear()
}

pub fn workspace_layout_restore_persisted(
    workspace_id: &str,
) -> Result<Option<WorkspaceLayoutState>, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_restore_persisted(workspace_id)
}

pub fn workspace_layout_snapshot(
    workspace_id: &str,
) -> Result<Option<WorkspaceLayoutState>, String> {
    embedded_runtime_arc()?.state.workspace_layout_snapshot(workspace_id)
}

pub fn workspace_layout_remove(workspace_id: &str) -> Result<(), String> {
    embedded_runtime_arc()?.state.workspace_layout_remove(workspace_id)
}

pub fn workspace_layout_replace(
    workspace_id: &str,
    layout: WorkspaceLayoutState,
) -> Result<WorkspaceLayoutState, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_replace(workspace_id, layout)
}

pub fn workspace_layout_ensure_session(
    workspace_id: &str,
    session_id: &str,
) -> Result<WorkspaceLayoutState, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_ensure_session(workspace_id, session_id)
}

pub fn workspace_layout_split_view(
    workspace_id: &str,
    view_id: SessionViewId,
    axis: WorkspaceSplitAxis,
    session_id: &str,
) -> Result<Option<WorkspaceLayoutState>, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_split_view(workspace_id, view_id, axis, session_id)
}

pub fn workspace_layout_attach_session(
    workspace_id: &str,
    view_id: SessionViewId,
    session_id: &str,
) -> Result<Option<WorkspaceLayoutState>, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_attach_session(workspace_id, view_id, session_id)
}

pub fn workspace_layout_focus_view(
    workspace_id: &str,
    view_id: SessionViewId,
) -> Result<Option<WorkspaceLayoutState>, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_focus_view(workspace_id, view_id)
}

pub fn workspace_layout_close_view(
    workspace_id: &str,
    view_id: SessionViewId,
) -> Result<Option<WorkspaceLayoutState>, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_close_view(workspace_id, view_id)
}

pub fn workspace_layout_resize_split(
    workspace_id: &str,
    node_id: WorkspaceNodeId,
    ratio: u16,
) -> Result<Option<WorkspaceLayoutState>, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_resize_split(workspace_id, node_id, ratio)
}

pub fn workspace_layout_view_id_for_session(
    workspace_id: &str,
    session_id: &str,
) -> Result<Option<SessionViewId>, String> {
    embedded_runtime_arc()?
        .state
        .workspace_layout_view_id_for_session(workspace_id, session_id)
}

#[allow(dead_code)]
pub fn load_workspace_passive() -> Result<RuntimeWorkspace, String> {
    runtime_client()?.workspace_load_passive()
}

pub fn load_desktop_workspace_snapshot(
    workspace_id: &str,
) -> Result<DesktopWorkspaceSnapshot, String> {
    Ok(DesktopWorkspaceSnapshot {
        workspace: load_workspace_passive()?,
        layout: workspace_layout_snapshot(workspace_id)?,
    })
}

pub fn desktop_session_window_snapshot(
    window_id: DesktopWindowId,
) -> Result<DesktopSessionWindowSnapshot, String> {
    let workspace_snapshot = load_desktop_workspace_snapshot(DEFAULT_LAYOUT_WORKSPACE_ID)?;
    let lookup = desktop_collect_session_lookup(window_id);
    let window_binding = desktop_window_binding(window_id, DEFAULT_LAYOUT_WORKSPACE_ID);
    Ok(DesktopSessionWindowSnapshot {
        window_binding,
        workspace_snapshot,
        lookup,
    })
}

pub fn desktop_ordered_session_ids(window_id: DesktopWindowId) -> Vec<String> {
    let Ok(snapshot) = desktop_session_window_snapshot(window_id) else {
        return Vec::new();
    };
    let workspace_ids: Vec<String> = snapshot
        .workspace_snapshot
        .workspace
        .sessions
        .iter()
        .map(|session| session.session_id.clone())
        .collect();
    let Some(layout) = snapshot.workspace_snapshot.layout else {
        return workspace_ids;
    };

    let mut ordered = Vec::new();
    for view in layout.views {
        if workspace_ids
            .iter()
            .any(|session_id| session_id == &view.session_id)
            && !ordered
                .iter()
                .any(|session_id| session_id == &view.session_id)
        {
            ordered.push(view.session_id);
        }
    }
    for session_id in workspace_ids {
        if !ordered.iter().any(|existing| existing == &session_id) {
            ordered.push(session_id);
        }
    }
    ordered
}

pub fn desktop_last_active_session_id(window_id: DesktopWindowId) -> Option<String> {
    desktop_session_window_snapshot(window_id)
        .ok()
        .and_then(|snapshot| snapshot.lookup.last_active_session_id)
}

pub fn desktop_can_close_view_only(window_id: DesktopWindowId) -> bool {
    desktop_session_window_snapshot(window_id)
        .ok()
        .and_then(|snapshot| snapshot.workspace_snapshot.layout)
        .map(|layout| layout.views.len() > 1)
        .unwrap_or(false)
}

pub fn desktop_session_entry_bindings(
    window_id: DesktopWindowId,
) -> Vec<DesktopSessionEntryBinding> {
    let Ok(snapshot) = desktop_session_window_snapshot(window_id) else {
        return Vec::new();
    };

    let layout = snapshot.workspace_snapshot.layout.clone();
    let active_view_id = snapshot.active_view_id();
    let view_order: HashMap<String, (usize, SessionViewId)> = layout
        .as_ref()
        .map(|layout| {
            layout
                .views
                .iter()
                .enumerate()
                .map(|(idx, view)| (view.session_id.clone(), (idx, view.view_id)))
                .collect()
        })
        .unwrap_or_default();

    let mut sessions = snapshot.workspace_snapshot.workspace.sessions.clone();
    sessions.sort_by_key(|session| {
        view_order
            .get(&session.session_id)
            .map(|(idx, _)| *idx)
            .unwrap_or(usize::MAX)
    });

    sessions
        .into_iter()
        .enumerate()
        .map(|(idx, session)| {
            let session_id = session.session_id;
            let title = session.name;
            let view_id = view_order.get(&session_id).map(|(_, view_id)| *view_id);
            let render_target_id = desktop_render_state_for_session(window_id, &session_id)
                .map(|state| state.render_target_id());
            DesktopSessionEntryBinding {
                entry_index: idx,
                session_id: session_id.clone(),
                title,
                view_id,
                render_target_id,
                is_active: view_id
                    .map(|view_id| Some(view_id) == active_view_id)
                    .unwrap_or_else(|| {
                        snapshot.lookup.active_session_id.as_deref()
                            == Some(session_id.as_str())
                    }),
                is_last_active: snapshot.lookup.last_active_session_id.as_deref()
                    == Some(session_id.as_str()),
            }
        })
        .collect()
}

pub fn desktop_active_session_entry_binding(
    window_id: DesktopWindowId,
) -> Option<DesktopSessionEntryBinding> {
    desktop_session_entry_bindings(window_id)
        .into_iter()
        .find(|entry| entry.is_active)
}

pub fn desktop_session_entry_binding_for_render_target(
    window_id: DesktopWindowId,
    render_target_id: SessionRenderTargetId,
) -> Option<DesktopSessionEntryBinding> {
    desktop_session_entry_bindings(window_id)
        .into_iter()
        .find(|entry| entry.render_target_id == Some(render_target_id))
}

pub fn desktop_active_render_target_id(
    window_id: DesktopWindowId,
) -> Option<SessionRenderTargetId> {
    desktop_session_window_snapshot(window_id)
        .ok()
        .and_then(|snapshot| snapshot.window_binding.active_render_target_id)
}

pub(crate) fn desktop_render_scope_capability(
    window_id: DesktopWindowId,
    render_target_id: SessionRenderTargetId,
) -> Option<Arc<overlay_compat::OverlayRenderScope>> {
    let session_id =
        desktop_session_entry_binding_for_render_target(window_id, render_target_id)?.session_id;
    desktop_render_scope_capability_for_session(window_id, &session_id)
}

pub fn desktop_workspace_subscribe(
    workspace_id: &str,
) -> Result<DesktopWorkspaceSubscription, String> {
    Ok(DesktopWorkspaceSubscription {
        client: runtime_client()?,
        workspace_id: workspace_id.to_string(),
    })
}

pub fn activate_runtime_session(session_id: &str, cols: usize, rows: usize) -> Result<(), String> {
    runtime_client()?.session_activate(session_id, cols, rows)
}

pub fn create_runtime_session(
    name: Option<String>,
    cols: usize,
    rows: usize,
    cwd: Option<String>,
    persist_history: Option<bool>,
) -> Result<RuntimeCreatedSession, String> {
    runtime_client()?.session_create(name, cols, rows, cwd, persist_history)
}

pub fn close_runtime_session(session_id: &str) -> Result<(), String> {
    runtime_client()?.session_close(session_id)
}

pub fn switch_runtime_profile(profile_id: &str) -> Result<RuntimeWorkspace, String> {
    runtime_client()?.profile_switch(profile_id)
}

pub fn create_runtime_profile(name: Option<String>) -> Result<RuntimeProfile, String> {
    runtime_client()?.profile_create(name)
}

#[allow(dead_code)]
pub fn read_session_snapshot(
    session_id: &str,
    preview_lines: Option<usize>,
) -> Result<RuntimeSessionSnapshot, String> {
    runtime_client()?.session_snapshot_get(session_id, preview_lines)
}

pub fn runtime_session_attachment(
    session_id: &str,
) -> Result<Option<(RuntimeId, TerminalInstanceId)>, String> {
    Ok(embedded_runtime_arc()?.state.session_runtime_attachment(session_id))
}

pub fn reconcile_runtime_session_lookup(
    lookup: &DesktopSessionLookup,
) -> Result<DesktopSessionBridgeAction, String> {
    runtime_client()?.reconcile_session_lookup(lookup)
}

pub fn notify_runtime_session_activated(
    session_id: &str,
    runtime_id: RuntimeId,
) -> Result<(), String> {
    runtime_client()?.notify_session_activated(session_id, runtime_id)
}

pub fn notify_runtime_session_closed(
    session_id: &str,
    runtime_id: RuntimeId,
    lookup_after_close: &DesktopSessionLookup,
) -> Result<(), String> {
    runtime_client()?.notify_session_closed(session_id, runtime_id, lookup_after_close)
}

pub fn desktop_collect_session_lookup(window_id: DesktopWindowId) -> DesktopSessionLookup {
    let runtime_ids_by_session = session_engine_shared()
        .and_then(|shared| {
            shared
                .core_state()
                .lock()
                .ok()
                .map(|core| {
                    core.session_runtime_map()
                        .map(|(session_id, runtime_id)| (session_id.clone(), *runtime_id))
                        .collect()
                })
        })
        .unwrap_or_default();
    DesktopSessionLookup {
        active_session_id: desktop_current_active_session_id(window_id),
        last_active_session_id: desktop_last_active_session(window_id),
        runtime_ids_by_session,
    }
}


pub fn desktop_render_state_for_session(
    window_id: DesktopWindowId,
    session_id: &str,
) -> Option<ChatminalRenderState> {
    let runtime_id = desktop_runtime_id_for_session(session_id)?;
    desktop_session_host(window_id)?.render_state_for_runtime(runtime_id)
}

// Kept for overlay compatibility: `OverlayRenderScope = Tab` is used by launcher/confirm/prompt
// overlays. This path is NOT used for session render state (splits = [] after Phase 03).
pub(crate) fn desktop_render_scope_capability_for_session(
    window_id: DesktopWindowId,
    session_id: &str,
) -> Option<Arc<overlay_compat::OverlayRenderScope>> {
    let runtime_id = desktop_runtime_id_for_session(session_id)?;
    desktop_session_host(window_id)?.overlay_scope_for_runtime(runtime_id)
}

/// Direct pane lookup for a session (1 session = 1 pane invariant, Phase 02+).
/// Returns None if session has no active pane (not started or already closed).
pub(crate) fn desktop_pane_for_session(
    window_id: DesktopWindowId,
    session_id: &str,
) -> Option<Arc<ChatminalSessionPane>> {
    desktop_session_host(window_id)?.pane_for_session(session_id)
}

fn workspace_split_axis(direction: DesktopSplitDirection) -> WorkspaceSplitAxis {
    match direction {
        DesktopSplitDirection::Horizontal => WorkspaceSplitAxis::Horizontal,
        DesktopSplitDirection::Vertical => WorkspaceSplitAxis::Vertical,
    }
}

pub fn desktop_activate_session(
    window_id: DesktopWindowId,
    session_id: &str,
    preferred_runtime_id: Option<RuntimeId>,
    size: engine_term::TerminalSize,
) -> Option<DesktopSessionRuntimeSummary> {
    let previous_session_id = desktop_current_active_session_id(window_id);
    let host = desktop_prepare_host_layout(window_id, size)?;
    if let Some(runtime_id) = preferred_runtime_id.or_else(|| desktop_runtime_id_for_session(session_id)) {
        let result = host
            .focus_runtime(session_id, runtime_id)
            .map(|state| {
                DesktopSessionRuntimeSummary::from_session(
                    window_id,
                    DEFAULT_LAYOUT_WORKSPACE_ID,
                    session_id,
                    state.snapshot.runtime_id,
                )
            });
        if result.is_some() {
            desktop_focus_layout_session(session_id);
            remember_desktop_last_active_session(window_id, previous_session_id, Some(session_id));
        }
        return result;
    }
    let command = runtime_proxy_command(Some(session_id));
    let result = host
        .ensure_runtime(session_id, 0, command, size)
        .map(|state| {
            DesktopSessionRuntimeSummary::from_session(
                window_id,
                DEFAULT_LAYOUT_WORKSPACE_ID,
                session_id,
                state.snapshot.runtime_id,
            )
        });
    if result.is_some() {
        desktop_focus_layout_session(session_id);
        remember_desktop_last_active_session(window_id, previous_session_id, Some(session_id));
    }
    result
}

pub fn desktop_focus_session_view(
    window_id: DesktopWindowId,
    view_id: SessionViewId,
) -> Option<String> {
    let previous_session_id = desktop_current_active_session_id(window_id);
    let layout = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID).focus_view(view_id)?;
    let session_id = layout.view(view_id)?.session_id.clone();
    let host = desktop_session_host(window_id)?;
    if let Some(runtime_id) = desktop_runtime_id_for_session(&session_id) {
        let _ = host.focus_runtime(&session_id, runtime_id);
    }
    remember_desktop_last_active_session(window_id, previous_session_id, Some(&session_id));
    Some(session_id)
}

pub fn desktop_detach_session_runtime_and_notify(
    window_id: DesktopWindowId,
    session_id: &str,
) -> bool {
    let previous_session_id = desktop_current_active_session_id(window_id);
    let host = match desktop_session_host(window_id) {
        Some(host) => host,
        None => return false,
    };
    let runtime_id = match desktop_runtime_id_for_session(session_id) {
        Some(runtime_id) => runtime_id,
        None => return false,
    };
    host.close_runtime(session_id, runtime_id);
    let promoted_session_id = desktop_close_layout_session(session_id);

    let lookup_after_close = desktop_collect_session_lookup(window_id);
    if let Err(err) = notify_runtime_session_closed(session_id, runtime_id, &lookup_after_close) {
        log::error!("failed to notify runtime bridge about session detach: {err}");
    }
    remember_desktop_last_active_session(
        window_id,
        previous_session_id,
        promoted_session_id
            .as_deref()
            .or(lookup_after_close.active_session_id.as_deref()),
    );
    true
}

#[allow(dead_code)]
pub fn desktop_remove_session_terminal_instance(
    window_id: DesktopWindowId,
    session_id: &str,
    runtime_id: RuntimeId,
    terminal_instance_id: TerminalInstanceId,
) -> bool {
    desktop_session_host(window_id)
        .map(|host| host.close_leaf(session_id, runtime_id, terminal_instance_id))
        .unwrap_or(false)
}

pub fn desktop_focus_session_terminal_instance(
    window_id: DesktopWindowId,
    session_id: &str,
    terminal_instance_id: TerminalInstanceId,
) -> Option<DesktopSessionRuntimeSummary> {
    let previous_session_id = desktop_current_active_session_id(window_id);
    let host = desktop_session_host(window_id)?;
    let runtime_id = desktop_runtime_id_for_session(session_id)?;
    let result = host
        .focus_terminal_instance(session_id, runtime_id, terminal_instance_id)
        .map(|state| {
            DesktopSessionRuntimeSummary::from_session(
                window_id,
                DEFAULT_LAYOUT_WORKSPACE_ID,
                session_id,
                state.snapshot.runtime_id,
            )
        });
    if result.is_some() {
        desktop_focus_layout_session(session_id);
        remember_desktop_last_active_session(window_id, previous_session_id, Some(session_id));
    }
    result
}

pub fn desktop_focus_session_terminal_handle(
    window_id: DesktopWindowId,
    terminal_handle: SessionTerminalHandle,
) -> Option<DesktopSessionRuntimeSummary> {
    let binding = desktop_session_terminal_binding(window_id, terminal_handle)?;
    desktop_focus_session_terminal_instance(
        window_id,
        &binding.session_id,
        binding.terminal_instance_id,
    )
}

pub fn desktop_swap_active_with_terminal_instance(
    window_id: DesktopWindowId,
    session_id: &str,
    terminal_instance_id: TerminalInstanceId,
    keep_focus: bool,
) -> bool {
    desktop_session_host(window_id)
        .map(|host| host.swap_active_with_terminal_instance(session_id, terminal_instance_id, keep_focus))
        .unwrap_or(false)
}

pub fn desktop_swap_active_with_terminal_handle(
    window_id: DesktopWindowId,
    terminal_handle: SessionTerminalHandle,
    keep_focus: bool,
) -> bool {
    let Some(binding) = desktop_session_terminal_binding(window_id, terminal_handle) else {
        return false;
    };
    desktop_swap_active_with_terminal_instance(
        window_id,
        &binding.session_id,
        binding.terminal_instance_id,
        keep_focus,
    )
}

pub fn desktop_focus_session_direction(
    window_id: DesktopWindowId,
    session_id: &str,
    direction: SessionDirection,
) -> Option<DesktopSessionRuntimeSummary> {
    let previous_session_id = desktop_current_active_session_id(window_id);
    let result = desktop_session_host(window_id)?
        .focus_direction(session_id, direction)
        .map(|state| {
            DesktopSessionRuntimeSummary::from_session(
                window_id,
                DEFAULT_LAYOUT_WORKSPACE_ID,
                session_id,
                state.snapshot.runtime_id,
            )
        });
    if result.is_some() {
        desktop_focus_layout_session(session_id);
        remember_desktop_last_active_session(window_id, previous_session_id, Some(session_id));
    }
    result
}

#[allow(dead_code)]
pub fn desktop_close_session_terminal_handle_or_session(
    window_id: DesktopWindowId,
    terminal_handle: SessionTerminalHandle,
) -> bool {
    let Some(binding) = desktop_session_terminal_binding(window_id, terminal_handle) else {
        return false;
    };
    let leaf_count = desktop_render_state_for_session(window_id, &binding.session_id)
        .map(|render_state| render_state.panes.len())
        .unwrap_or(1);
    if leaf_count <= 1 {
        // CloseCurrentSession = hard delete: stop PTY + remove layout slot + delete SessionEntry.
        return desktop_hard_close_session(window_id, &binding.session_id);
    }
    if !desktop_remove_session_terminal_instance(
        window_id,
        &binding.session_id,
        binding.runtime_id,
        binding.terminal_instance_id,
    ) {
        return false;
    }
    if let Err(err) = notify_runtime_session_activated(&binding.session_id, binding.runtime_id) {
        log::error!(
            "failed to notify runtime bridge about session activation after host leaf close: {err}"
        );
    }
    true
}

/// Hard delete a session: stop PTY + remove layout slot + delete SessionEntry from DaemonState.
/// This is the `CloseCurrentSession` path. `DetachSession` (future) uses
/// `desktop_detach_session_runtime_and_notify` instead which keeps the SessionEntry.
#[allow(dead_code)]
pub fn desktop_hard_close_session(window_id: DesktopWindowId, session_id: &str) -> bool {
    let previous_session_id = desktop_current_active_session_id(window_id);
    // Stop host runtime and remove layout slot first so UI collapses immediately.
    if let Some(host) = desktop_session_host(window_id) {
        if let Some(runtime_id) = desktop_runtime_id_for_session(session_id) {
            host.close_runtime(session_id, runtime_id);
        }
    }
    let promoted_session_id = desktop_close_layout_session(session_id);

    // Hard delete: remove SessionEntry from DaemonState (publishes WorkspaceUpdated event).
    if let Err(err) = close_runtime_session(session_id) {
        log::warn!("desktop_hard_close_session: session_close failed for {session_id}: {err}");
    }

    let lookup_after_close = desktop_collect_session_lookup(window_id);
    let runtime_id = desktop_runtime_id_for_session(session_id)
        .unwrap_or_else(|| chatminal_runtime::RuntimeId::new(0));
    if let Err(err) = notify_runtime_session_closed(session_id, runtime_id, &lookup_after_close) {
        log::error!("failed to notify runtime bridge about session close: {err}");
    }
    remember_desktop_last_active_session(
        window_id,
        previous_session_id,
        promoted_session_id
            .as_deref()
            .or(lookup_after_close.active_session_id.as_deref()),
    );
    true
}

pub fn desktop_create_split_session_view(
    window_id: DesktopWindowId,
    request: DesktopSplitRequest,
    size: engine_term::TerminalSize,
    cwd: Option<String>,
) -> anyhow::Result<String> {
    desktop_session_host(window_id)
        .ok_or_else(|| anyhow::anyhow!("desktop session host missing"))?;
    let layout_store = DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID);
    let active_session_id = desktop_current_active_session_id(window_id)
        .ok_or_else(|| anyhow::anyhow!("no active session for split view"))?;
    let _ = layout_store.ensure_for_session(&active_session_id);
    let active_view_id = layout_store
        .snapshot_or_restore()
        .map(|layout| layout.active_view_id)
        .or_else(|| layout_store.view_id_for_session(&active_session_id))
        .ok_or_else(|| anyhow::anyhow!("no active session view for split view"))?;

    let created = create_runtime_session(None, size.cols.max(20), size.rows.max(5), cwd, Some(true))
        .map_err(anyhow::Error::msg)?;

    let split = layout_store.split_view(
        active_view_id,
        workspace_split_axis(request.direction),
        &created.session_id,
    );
    if split.is_none() {
        let _ = close_runtime_session(&created.session_id);
        anyhow::bail!("failed to split active session view");
    }

    let state = desktop_activate_session(window_id, &created.session_id, None, size)
        .ok_or_else(|| anyhow::anyhow!("failed to ensure session runtime for split view"))?;
    if let Err(err) = notify_runtime_session_activated(&created.session_id, state.runtime_id) {
        log::error!("failed to notify runtime bridge about split-view focus: {err}");
    }

    Ok(created.session_id)
}

pub async fn desktop_split_terminal_handle(
    window_id: DesktopWindowId,
    terminal_handle: u64,
    request: DesktopSplitRequest,
    command: Option<portable_pty::CommandBuilder>,
    command_dir: Option<String>,
    domain: SpawnSessionDomain,
    term_config: Arc<config::TermConfig>,
) -> anyhow::Result<()> {
    let host = desktop_session_host(window_id)
        .ok_or_else(|| anyhow::anyhow!("desktop session host missing"))?;
    host.split_terminal_handle(
        terminal_handle,
        request.into_host_request(),
        command,
        command_dir,
        domain,
        term_config,
    )
    .await
}

pub fn desktop_sync_session_from_host(
    window_id: DesktopWindowId,
    session_id: &str,
) -> Option<DesktopSessionRuntimeSummary> {
    let previous_session_id = desktop_current_active_session_id(window_id);
    let result = desktop_session_host(window_id)?
        .sync_runtime_from_host(session_id)
        .map(|state| {
            DesktopSessionRuntimeSummary::from_session(
                window_id,
                DEFAULT_LAYOUT_WORKSPACE_ID,
                session_id,
                state.snapshot.runtime_id,
            )
        });
    if result.is_some() {
        desktop_focus_layout_session(session_id);
        remember_desktop_last_active_session(window_id, previous_session_id, Some(session_id));
    }
    result
}

pub fn desktop_move_terminal_handle_to_new_window(
    window_id: DesktopWindowId,
    terminal_handle: u64,
) -> bool {
    desktop_session_host(window_id)
        .map(|host| host.move_terminal_handle_to_new_window(terminal_handle))
        .unwrap_or(false)
}

pub fn desktop_move_terminal_handle_to_new_runtime(
    window_id: DesktopWindowId,
    terminal_handle: u64,
) -> bool {
    desktop_session_host(window_id)
        .map(|host| host.move_terminal_handle_to_new_runtime(terminal_handle))
        .unwrap_or(false)
}

pub fn desktop_current_active_session_id(window_id: DesktopWindowId) -> Option<String> {
    let _ = window_id;
    load_desktop_workspace_snapshot(DEFAULT_LAYOUT_WORKSPACE_ID)
        .ok()?
        .active_session_id()
}

pub fn desktop_current_active_terminal_handle(
    window_id: DesktopWindowId,
) -> Option<SessionTerminalHandle> {
    let session_id = desktop_current_active_session_id(window_id)?;
    let render_state = desktop_render_state_for_session(window_id, &session_id)?;
    render_state
        .panes
        .iter()
        .find(|pane| pane.is_active)
        .or_else(|| {
            render_state.active_terminal_instance_id.and_then(|active_terminal_instance_id| {
                render_state
                    .panes
                    .iter()
                    .find(|pane| pane.terminal_instance_id == active_terminal_instance_id)
            })
        })
        .map(|pane| pane.terminal_handle)
}

pub fn desktop_resize_layout_split(
    window_id: DesktopWindowId,
    node_id: WorkspaceNodeId,
    ratio: u16,
) -> Option<WorkspaceLayoutState> {
    let _ = window_id;
    DesktopWorkspaceLayoutStore::new(DEFAULT_LAYOUT_WORKSPACE_ID).resize_split(node_id, ratio)
}
