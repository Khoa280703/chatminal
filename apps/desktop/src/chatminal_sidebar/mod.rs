use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use runtime::{RuntimeCreatedSession, RuntimeProfile, RuntimeWorkspace};

use crate::runtime_module::DESKTOP_LAYOUT_WORKSPACE_ID;
use crate::runtime_module::{
    build_desktop_sidebar_sessions, close_runtime_session, create_runtime_profile,
    create_runtime_session, delete_runtime_profile, desktop_workspace_subscribe,
    move_runtime_session_to_profile, move_runtime_sessions_to_profile, rename_runtime_session,
    set_runtime_session_startup_command, switch_runtime_profile,
};
pub use crate::runtime_module::{
    DesktopSidebarProfile as SidebarProfile, DesktopSidebarSession as SidebarSession,
    DesktopSidebarSnapshot as SidebarSnapshot,
};

const SIDEBAR_DEFAULT_WIDTH_PX: f32 = 304.0;
const SIDEBAR_MIN_WIDTH_PX: f32 = 56.0;
const SIDEBAR_MAX_WINDOW_RATIO: f32 = 0.58;
const TERMINAL_MIN_CONTENT_WIDTH_PX: f32 = 640.0;
const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(400);

#[derive(Debug)]
pub struct SidebarSessionContextMenu {
    pub session_id: String,
    pub anchor_x_px: f32,
    pub anchor_y_px: f32,
}

#[derive(Debug)]
pub struct SidebarProfileContextMenu {
    pub profile_id: String,
    pub anchor_x_px: f32,
    pub anchor_y_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarInlineSessionEditKind {
    Rename,
}

#[derive(Debug, Clone)]
pub struct SidebarInlineSessionEditState {
    pub session_id: String,
    pub kind: SidebarInlineSessionEditKind,
    pub input: String,
    pub select_all: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarSessionDropTarget {
    ProfileAppend {
        profile_id: String,
    },
    SessionInsertBefore {
        profile_id: String,
        session_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarSessionDragState {
    pub anchor_session_id: String,
    pub session_ids: Vec<String>,
    pub drop_target: Option<SidebarSessionDropTarget>,
}

#[derive(Debug)]
struct SharedState {
    snapshot: SidebarSnapshot,
    expanded_profile_ids: BTreeSet<String>,
    selected_session_ids: BTreeSet<String>,
    seed_active_profile_on_first_snapshot: bool,
    scroll_offset_px: f32,
    max_scroll_offset_px: f32,
    width_override_px: Option<f32>,
    session_context_menu: Option<SidebarSessionContextMenu>,
    profile_context_menu: Option<SidebarProfileContextMenu>,
    inline_session_edit: Option<SidebarInlineSessionEditState>,
    session_drag: Option<SidebarSessionDragState>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            snapshot: SidebarSnapshot::default(),
            expanded_profile_ids: BTreeSet::new(),
            selected_session_ids: BTreeSet::new(),
            seed_active_profile_on_first_snapshot: true,
            scroll_offset_px: 0.0,
            max_scroll_offset_px: 0.0,
            width_override_px: None,
            session_context_menu: None,
            profile_context_menu: None,
            inline_session_edit: None,
            session_drag: None,
        }
    }
}

#[derive(Debug)]
pub struct ChatminalSidebar {
    shared: Arc<Mutex<SharedState>>,
    sync_started: AtomicBool,
}

impl ChatminalSidebar {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(SharedState::default())),
            sync_started: AtomicBool::new(false),
        }
    }

    pub fn is_enabled(&self) -> bool {
        true
    }

    pub fn snapshot(&self) -> SidebarSnapshot {
        self.shared
            .lock()
            .map(|state| state.snapshot.clone())
            .unwrap_or_default()
    }

    pub fn version(&self) -> u64 {
        self.shared
            .lock()
            .map(|state| state.snapshot.version)
            .unwrap_or(0)
    }

    pub fn start_background_sync(&self) {
        if self
            .sync_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let shared = Arc::clone(&self.shared);
        thread::spawn(move || run_sync_loop(shared));
    }

    pub fn create_session(
        &self,
        cols: usize,
        rows: usize,
    ) -> Result<RuntimeCreatedSession, String> {
        create_runtime_session(None, cols, rows, None, Some(true))
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        close_runtime_session(session_id)
    }

    pub fn rename_session(&self, session_id: &str, name: &str) -> Result<RuntimeWorkspace, String> {
        rename_runtime_session(session_id, name)
    }

    pub fn set_session_startup_command(
        &self,
        session_id: &str,
        startup_command: Option<&str>,
    ) -> Result<RuntimeWorkspace, String> {
        set_runtime_session_startup_command(session_id, startup_command)
    }

    pub fn open_session_context_menu(
        &self,
        session_id: &str,
        anchor_x_px: f32,
        anchor_y_px: f32,
    ) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let Some(session) = state
            .snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == session_id)
        else {
            return false;
        };
        let next = SidebarSessionContextMenu {
            session_id: session.session_id.clone(),
            anchor_x_px,
            anchor_y_px,
        };
        let changed = state
            .session_context_menu
            .as_ref()
            .map(|menu| {
                menu.session_id != next.session_id
                    || (menu.anchor_x_px - next.anchor_x_px).abs() >= f32::EPSILON
                    || (menu.anchor_y_px - next.anchor_y_px).abs() >= f32::EPSILON
            })
            .unwrap_or(true);
        if !changed {
            return false;
        }
        state.session_context_menu = Some(next);
        state.profile_context_menu = None;
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn open_profile_context_menu(
        &self,
        profile_id: &str,
        anchor_x_px: f32,
        anchor_y_px: f32,
    ) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let Some(profile) = state
            .snapshot
            .profiles
            .iter()
            .find(|profile| profile.profile_id == profile_id)
        else {
            return false;
        };
        let next = SidebarProfileContextMenu {
            profile_id: profile.profile_id.clone(),
            anchor_x_px,
            anchor_y_px,
        };
        let changed = state
            .profile_context_menu
            .as_ref()
            .map(|menu| {
                menu.profile_id != next.profile_id
                    || (menu.anchor_x_px - next.anchor_x_px).abs() >= f32::EPSILON
                    || (menu.anchor_y_px - next.anchor_y_px).abs() >= f32::EPSILON
            })
            .unwrap_or(true);
        if !changed {
            return false;
        }
        state.profile_context_menu = Some(next);
        state.session_context_menu = None;
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn selected_session_ids(&self) -> Vec<String> {
        self.shared
            .lock()
            .map(|state| state.selected_session_ids.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn is_session_selected(&self, session_id: &str) -> bool {
        self.shared
            .lock()
            .map(|state| state.selected_session_ids.contains(session_id))
            .unwrap_or(false)
    }

    pub fn select_single_session(&self, session_id: &str) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        if !state
            .snapshot
            .sessions
            .iter()
            .any(|session| session.session_id == session_id)
        {
            return false;
        }

        let changed = state.selected_session_ids.len() != 1
            || !state.selected_session_ids.contains(session_id);
        if !changed {
            return false;
        }

        state.selected_session_ids.clear();
        state.selected_session_ids.insert(session_id.to_string());
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn toggle_session_selected(&self, session_id: &str) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        if !state
            .snapshot
            .sessions
            .iter()
            .any(|session| session.session_id == session_id)
        {
            return false;
        }

        let changed = if state.selected_session_ids.contains(session_id) {
            state.selected_session_ids.remove(session_id)
        } else {
            state.selected_session_ids.insert(session_id.to_string())
        };
        if !changed {
            return false;
        }
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn ensure_context_menu_session_selected(&self, session_id: &str) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        if !state
            .snapshot
            .sessions
            .iter()
            .any(|session| session.session_id == session_id)
        {
            return false;
        }

        if state.selected_session_ids.len() > 1 && state.selected_session_ids.contains(session_id) {
            return false;
        }

        state.selected_session_ids.clear();
        state.selected_session_ids.insert(session_id.to_string());
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn close_session_context_menu(&self) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let closed_session = state.session_context_menu.take().is_some();
        let closed_profile = state.profile_context_menu.take().is_some();
        if !closed_session && !closed_profile {
            return false;
        }
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn session_context_menu(&self) -> Option<SidebarSessionContextMenu> {
        self.shared
            .lock()
            .ok()
            .and_then(|state| state.session_context_menu.as_ref().map(clone_context_menu))
    }

    pub fn profile_context_menu(&self) -> Option<SidebarProfileContextMenu> {
        self.shared
            .lock()
            .ok()
            .and_then(|state| {
                state
                    .profile_context_menu
                    .as_ref()
                    .map(clone_profile_context_menu)
            })
    }

    fn start_inline_session_edit(
        &self,
        session_id: &str,
        kind: SidebarInlineSessionEditKind,
        input: String,
    ) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let Some(session) = state
            .snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == session_id)
        else {
            return false;
        };
        let next = SidebarInlineSessionEditState {
            session_id: session.session_id.clone(),
            kind,
            input,
            select_all: true,
        };
        let changed = state
            .inline_session_edit
            .as_ref()
            .map(|edit| {
                edit.session_id != next.session_id
                    || edit.kind != next.kind
                    || edit.input != next.input
            })
            .unwrap_or(true);
        if !changed {
            return false;
        }
        state.session_context_menu = None;
        state.profile_context_menu = None;
        state.inline_session_edit = Some(next);
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn start_inline_rename(&self, session_id: &str) -> bool {
        let input = self
            .snapshot()
            .sessions
            .into_iter()
            .find(|session| session.session_id == session_id)
            .map(|session| session.name)
            .unwrap_or_default();
        self.start_inline_session_edit(session_id, SidebarInlineSessionEditKind::Rename, input)
    }

    pub fn inline_session_edit_state(&self) -> Option<SidebarInlineSessionEditState> {
        self.shared.lock().ok().and_then(|state| {
            state
                .inline_session_edit
                .as_ref()
                .map(clone_inline_session_edit)
        })
    }

    pub fn inline_session_edit_cancel(&self) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        if state.inline_session_edit.take().is_none() {
            return false;
        }
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn inline_session_edit_backspace(&self) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let Some(edit) = state.inline_session_edit.as_mut() else {
            return false;
        };
        if edit.select_all {
            if edit.input.is_empty() {
                return false;
            }
            edit.input.clear();
            edit.select_all = false;
            state.snapshot.version = state.snapshot.version.saturating_add(1);
            return true;
        }
        if edit.input.is_empty() {
            return false;
        }
        edit.input.pop();
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn inline_session_edit_clear(&self) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let Some(edit) = state.inline_session_edit.as_mut() else {
            return false;
        };
        if edit.input.is_empty() {
            return false;
        }
        edit.input.clear();
        edit.select_all = false;
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn inline_session_edit_push(&self, c: char) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let Some(edit) = state.inline_session_edit.as_mut() else {
            return false;
        };
        if edit.select_all {
            edit.input.clear();
            edit.select_all = false;
        }
        edit.input.push(c);
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn inline_session_edit_commit(
        &self,
    ) -> Option<(SidebarInlineSessionEditKind, String, String)> {
        let Ok(mut state) = self.shared.lock() else {
            return None;
        };
        let edit = state.inline_session_edit.take()?;
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        Some((edit.kind, edit.session_id, edit.input))
    }

    pub fn session_drag_state(&self) -> Option<SidebarSessionDragState> {
        self.shared
            .lock()
            .ok()
            .and_then(|state| state.session_drag.clone())
    }

    pub fn start_session_drag(&self, anchor_session_id: &str, session_ids: Vec<String>) -> bool {
        if session_ids.is_empty() {
            return false;
        }
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        if !session_ids.iter().all(|session_id| {
            state
                .snapshot
                .sessions
                .iter()
                .any(|session| session.session_id == *session_id)
        }) {
            return false;
        }
        let next = SidebarSessionDragState {
            anchor_session_id: anchor_session_id.to_string(),
            session_ids,
            drop_target: None,
        };
        if state.session_drag.as_ref() == Some(&next) {
            return false;
        }
        state.session_context_menu = None;
        state.profile_context_menu = None;
        state.inline_session_edit = None;
        state.session_drag = Some(next);
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn set_session_drag_target(&self, target: Option<SidebarSessionDropTarget>) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let Some(drag) = state.session_drag.as_mut() else {
            return false;
        };
        if drag.drop_target == target {
            return false;
        }
        drag.drop_target = target;
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn clear_session_drag(&self) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        if state.session_drag.take().is_none() {
            return false;
        }
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn toggle_profile_expanded(&self, profile_id: &str) {
        let Ok(mut state) = self.shared.lock() else {
            return;
        };
        if !state
            .snapshot
            .profiles
            .iter()
            .any(|profile| profile.profile_id == profile_id)
        {
            return;
        }

        if !state.expanded_profile_ids.remove(profile_id) {
            state.expanded_profile_ids.insert(profile_id.to_string());
        }
        state.seed_active_profile_on_first_snapshot = false;
        state.snapshot.version = state.snapshot.version.saturating_add(1);
    }

    pub fn is_profile_expanded(&self, profile_id: &str) -> bool {
        self.shared
            .lock()
            .map(|state| state.expanded_profile_ids.contains(profile_id))
            .unwrap_or(false)
    }

    pub fn ensure_profile_expanded(&self, profile_id: &str) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        if !state
            .snapshot
            .profiles
            .iter()
            .any(|profile| profile.profile_id == profile_id)
        {
            return false;
        }
        if !state.expanded_profile_ids.insert(profile_id.to_string()) {
            return false;
        }
        state.seed_active_profile_on_first_snapshot = false;
        state.snapshot.version = state.snapshot.version.saturating_add(1);
        true
    }

    pub fn scroll_offset_px(&self) -> f32 {
        self.shared
            .lock()
            .map(|state| state.scroll_offset_px)
            .unwrap_or(0.0)
    }

    pub fn scroll_pixels(&self, delta: f32) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let next = (state.scroll_offset_px + delta).clamp(0.0, state.max_scroll_offset_px);
        if (next - state.scroll_offset_px).abs() < f32::EPSILON {
            return false;
        }
        state.scroll_offset_px = next;
        true
    }

    pub fn set_scroll_bounds(&self, max_scroll_offset_px: f32) -> bool {
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        let mut changed = false;
        let clamped_max = max_scroll_offset_px.max(0.0);
        if (state.max_scroll_offset_px - clamped_max).abs() >= f32::EPSILON {
            state.max_scroll_offset_px = clamped_max;
            changed = true;
        }
        if state.scroll_offset_px > state.max_scroll_offset_px {
            state.scroll_offset_px = state.max_scroll_offset_px;
            changed = true;
        }
        changed
    }

    pub fn width_pixels_for_window(&self, window_width: usize, dpi: usize) -> usize {
        let override_px = self
            .shared
            .lock()
            .ok()
            .and_then(|state| state.width_override_px);
        clamp_sidebar_width_px(
            override_px.unwrap_or_else(|| default_sidebar_width_px(dpi)),
            window_width,
            dpi,
        )
    }

    pub fn set_width_pixels(
        &self,
        requested_width_px: f32,
        window_width: usize,
        dpi: usize,
    ) -> bool {
        let clamped = clamp_sidebar_width_px(requested_width_px, window_width, dpi) as f32;
        let Ok(mut state) = self.shared.lock() else {
            return false;
        };
        if state
            .width_override_px
            .map(|width| (width - clamped).abs() < f32::EPSILON)
            .unwrap_or(false)
        {
            return false;
        }
        state.width_override_px = Some(clamped);
        true
    }

    #[allow(dead_code)]
    pub fn move_session_to_profile(
        &self,
        session_id: &str,
        profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<RuntimeWorkspace, String> {
        move_runtime_session_to_profile(session_id, profile_id, target_index)
    }

    pub fn move_sessions_to_profile(
        &self,
        session_ids: &[String],
        profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<RuntimeWorkspace, String> {
        move_runtime_sessions_to_profile(session_ids, profile_id, target_index)
    }

    pub fn switch_profile(&self, profile_id: &str) -> Result<RuntimeWorkspace, String> {
        switch_runtime_profile(profile_id)
    }

    pub fn create_profile(&self) -> Result<RuntimeProfile, String> {
        create_runtime_profile(None)
    }

    pub fn delete_profile(&self, profile_id: &str) -> Result<RuntimeWorkspace, String> {
        delete_runtime_profile(profile_id)
    }

    pub fn apply_workspace(&self, workspace: RuntimeWorkspace) {
        replace_workspace(&self.shared, workspace);
    }

    pub fn set_active_session_local(&self, session_id: &str) {
        self.mark_active_session(session_id);
    }

    pub fn set_session_status_local(&self, session_id: &str, status: &str) {
        let Ok(mut state) = self.shared.lock() else {
            return;
        };
        let mut next = state.snapshot.clone();
        let Some(session) = next
            .sessions
            .iter_mut()
            .find(|session| session.session_id == session_id)
        else {
            return;
        };
        if session.status == status {
            return;
        }
        session.status = status.to_string();
        next.version = next.version.saturating_add(1);
        state.snapshot = next;
    }

    fn mark_active_session(&self, session_id: &str) {
        let Ok(mut state) = self.shared.lock() else {
            return;
        };
        let mut next = state.snapshot.clone();
        let next_active_profile_id = next
            .sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .map(|session| session.profile_id.clone());
        let mut changed = next.active_session_id.as_deref() != Some(session_id)
            || next.active_profile_id != next_active_profile_id;
        next.active_session_id = Some(session_id.to_string());
        next.active_profile_id = next_active_profile_id.clone();
        if let Some(profile_id) = next_active_profile_id.as_deref() {
            if state.expanded_profile_ids.insert(profile_id.to_string()) {
                state.seed_active_profile_on_first_snapshot = false;
                changed = true;
            }
        }
        for profile in &mut next.profiles {
            let is_active = next.active_profile_id.as_deref() == Some(profile.profile_id.as_str());
            if profile.is_active != is_active {
                profile.is_active = is_active;
                changed = true;
            }
        }
        for session in &mut next.sessions {
            let is_active = session.session_id == session_id;
            if session.is_active != is_active {
                session.is_active = is_active;
                changed = true;
            }
        }
        if !changed {
            return;
        }
        next.version = next.version.saturating_add(1);
        state.snapshot = next;
    }

    pub fn width_pixels(window_width: usize, dpi: usize) -> usize {
        clamp_sidebar_width_px(default_sidebar_width_px(dpi), window_width, dpi)
    }
}

fn default_sidebar_width_px(dpi: usize) -> f32 {
    let scale = (dpi as f32 / 96.0).max(1.0);
    SIDEBAR_DEFAULT_WIDTH_PX * scale
}

fn clamp_sidebar_width_px(requested_width_px: f32, window_width: usize, dpi: usize) -> usize {
    let scale = (dpi as f32 / 96.0).max(1.0);
    let min_width = SIDEBAR_MIN_WIDTH_PX * scale;
    let min_terminal_width = TERMINAL_MIN_CONTENT_WIDTH_PX * scale;
    let max_width_by_ratio = window_width as f32 * SIDEBAR_MAX_WINDOW_RATIO;
    let max_width_by_terminal = (window_width as f32 - min_terminal_width).max(min_width);
    let max_width = max_width_by_ratio.min(max_width_by_terminal).max(min_width);
    requested_width_px.min(max_width).max(min_width).round() as usize
}

pub fn sidebar_enabled() -> bool {
    true
}

fn run_sync_loop(shared: Arc<Mutex<SharedState>>) {
    loop {
        let subscription = match desktop_workspace_subscribe(DESKTOP_LAYOUT_WORKSPACE_ID) {
            Ok(subscription) => subscription,
            Err(err) => {
                replace_error(&shared, format!("sidebar runtime init failed: {err}"));
                return;
            }
        };

        if let Err(err) = refresh_snapshot(&subscription, &shared) {
            replace_error(&shared, format!("sidebar load failed: {err}"));
            return;
        }

        loop {
            match subscription.recv_sidebar_snapshot(EVENT_POLL_TIMEOUT) {
                Ok(Some(snapshot)) => replace_snapshot(&shared, snapshot),
                Ok(None) => {}
                Err(err) => {
                    replace_error(&shared, format!("sidebar stream failed: {err}"));
                    return;
                }
            }
        }
    }
}

fn refresh_snapshot(
    subscription: &crate::runtime_module::DesktopWorkspaceSubscription,
    shared: &Arc<Mutex<SharedState>>,
) -> Result<(), String> {
    replace_snapshot(shared, subscription.load_sidebar_snapshot()?);
    Ok(())
}

fn replace_workspace(shared: &Arc<Mutex<SharedState>>, workspace: RuntimeWorkspace) {
    let active_profile_id = workspace.active_profile_id.clone();
    let active_session_id = workspace.active_session_id.clone();
    let sessions = build_desktop_sidebar_sessions(
        &workspace.profiles,
        &workspace.sessions,
        active_session_id.as_deref(),
    );
    let next = SidebarSnapshot {
        active_profile_id: active_profile_id.clone(),
        active_session_id: active_session_id.clone(),
        profiles: workspace
            .profiles
            .into_iter()
            .map(|profile| SidebarProfile {
                is_active: active_profile_id.as_deref() == Some(profile.profile_id.as_str()),
                profile_id: profile.profile_id,
                name: profile.name,
            })
            .collect(),
        sessions,
        error: None,
        version: 0,
    };
    replace_snapshot(shared, next);
}

fn replace_error(shared: &Arc<Mutex<SharedState>>, message: String) {
    let next = SidebarSnapshot {
        error: Some(message),
        ..shared
            .lock()
            .map(|state| state.snapshot.clone())
            .unwrap_or_default()
    };
    replace_snapshot(shared, next);
}

fn replace_snapshot(shared: &Arc<Mutex<SharedState>>, mut next: SidebarSnapshot) {
    let Ok(mut state) = shared.lock() else {
        return;
    };
    let valid_profile_ids: BTreeSet<&str> = next
        .profiles
        .iter()
        .map(|profile| profile.profile_id.as_str())
        .collect();
    state
        .expanded_profile_ids
        .retain(|profile_id| valid_profile_ids.contains(profile_id.as_str()));
    if state.seed_active_profile_on_first_snapshot {
        if let Some(profile_id) = next.active_profile_id.as_deref() {
            state.expanded_profile_ids.insert(profile_id.to_string());
        }
        state.seed_active_profile_on_first_snapshot = false;
    }
    let valid_session_ids: BTreeSet<&str> = next
        .sessions
        .iter()
        .map(|session| session.session_id.as_str())
        .collect();
    state
        .selected_session_ids
        .retain(|session_id| valid_session_ids.contains(session_id.as_str()));
    if state
        .session_context_menu
        .as_ref()
        .is_some_and(|menu| !valid_session_ids.contains(menu.session_id.as_str()))
    {
        state.session_context_menu = None;
    }
    if state
        .profile_context_menu
        .as_ref()
        .is_some_and(|menu| !valid_profile_ids.contains(menu.profile_id.as_str()))
    {
        state.profile_context_menu = None;
    }
    if state
        .inline_session_edit
        .as_ref()
        .is_some_and(|edit| !valid_session_ids.contains(edit.session_id.as_str()))
    {
        state.inline_session_edit = None;
    }
    if state.session_drag.as_ref().is_some_and(|drag| {
        drag.session_ids
            .iter()
            .any(|session_id| !valid_session_ids.contains(session_id.as_str()))
    }) {
        state.session_drag = None;
    }
    let changed = state.snapshot.active_profile_id != next.active_profile_id
        || state.snapshot.active_session_id != next.active_session_id
        || state.snapshot.profiles != next.profiles
        || state.snapshot.sessions != next.sessions
        || state.snapshot.error != next.error;
    if !changed {
        return;
    }
    next.version = state.snapshot.version.saturating_add(1);
    state.snapshot = next;
}

fn clone_context_menu(menu: &SidebarSessionContextMenu) -> SidebarSessionContextMenu {
    SidebarSessionContextMenu {
        session_id: menu.session_id.clone(),
        anchor_x_px: menu.anchor_x_px,
        anchor_y_px: menu.anchor_y_px,
    }
}

fn clone_profile_context_menu(menu: &SidebarProfileContextMenu) -> SidebarProfileContextMenu {
    SidebarProfileContextMenu {
        profile_id: menu.profile_id.clone(),
        anchor_x_px: menu.anchor_x_px,
        anchor_y_px: menu.anchor_y_px,
    }
}

fn clone_inline_session_edit(
    edit: &SidebarInlineSessionEditState,
) -> SidebarInlineSessionEditState {
    SidebarInlineSessionEditState {
        session_id: edit.session_id.clone(),
        kind: edit.kind,
        input: edit.input.clone(),
        select_all: edit.select_all,
    }
}
