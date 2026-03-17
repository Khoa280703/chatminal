use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chatminal_runtime::{RuntimeCreatedSession, RuntimeProfile, RuntimeWorkspace};
use serde::{Deserialize, Serialize};

use crate::chatminal_runtime::DESKTOP_LAYOUT_WORKSPACE_ID;
use crate::chatminal_runtime::{
    activate_runtime_session, close_runtime_session, create_runtime_profile,
    create_runtime_session, desktop_workspace_subscribe, load_desktop_sidebar_sessions_from_store,
    switch_runtime_profile,
};
pub use crate::chatminal_runtime::{
    DesktopSidebarProfile as SidebarProfile, DesktopSidebarSession as SidebarSession,
    DesktopSidebarSnapshot as SidebarSnapshot,
};

const SIDEBAR_ENABLE_ENV: &str = "CHATMINAL_DESKTOP_SESSIONS_SIDEBAR";
const SIDEBAR_DEFAULT_WIDTH_PX: f32 = 304.0;
const SIDEBAR_MIN_WIDTH_PX: f32 = 280.0;
const SIDEBAR_MAX_WINDOW_RATIO: f32 = 0.32;
const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(400);
const SIDEBAR_TREE_STATE_FILE: &str = "chatminal-sidebar-tree-state.json";

#[derive(Debug)]
struct SharedState {
    snapshot: SidebarSnapshot,
    expanded_profile_ids: BTreeSet<String>,
    seed_active_profile_on_first_snapshot: bool,
}

impl Default for SharedState {
    fn default() -> Self {
        let (expanded_profile_ids, seed_active_profile_on_first_snapshot) = load_tree_state();
        Self {
            snapshot: SidebarSnapshot::default(),
            expanded_profile_ids,
            seed_active_profile_on_first_snapshot,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedSidebarTreeState {
    expanded_profile_ids: BTreeSet<String>,
}

#[derive(Debug)]
pub struct ChatminalSidebar {
    enabled: bool,
    shared: Arc<Mutex<SharedState>>,
    sync_started: AtomicBool,
}

impl ChatminalSidebar {
    pub fn from_env() -> Self {
        Self {
            enabled: sidebar_enabled_from_env(),
            shared: Arc::new(Mutex::new(SharedState::default())),
            sync_started: AtomicBool::new(false),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
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
        if !self.enabled {
            return;
        }
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

    pub fn activate_session(
        &self,
        session_id: &str,
        cols: usize,
        rows: usize,
    ) -> Result<(), String> {
        activate_runtime_session(session_id, cols, rows)?;
        self.mark_active_session(session_id);
        Ok(())
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

    pub fn switch_profile(&self, profile_id: &str) -> Result<RuntimeWorkspace, String> {
        switch_runtime_profile(profile_id)
    }

    pub fn create_profile(&self) -> Result<RuntimeProfile, String> {
        create_runtime_profile(None)
    }

    pub fn apply_workspace(&self, workspace: RuntimeWorkspace) {
        replace_workspace(&self.shared, workspace);
    }

    pub fn ensure_profile_expanded(&self, profile_id: &str) {
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
        if !state.expanded_profile_ids.insert(profile_id.to_string()) {
            return;
        }
        state.seed_active_profile_on_first_snapshot = false;
        let seed_active_profile_on_first_snapshot = state.seed_active_profile_on_first_snapshot;
        let mut next = state.snapshot.clone();
        apply_expanded_profiles(
            &mut next,
            &mut state.expanded_profile_ids,
            seed_active_profile_on_first_snapshot,
        );
        next.version = next.version.saturating_add(1);
        state.snapshot = next;
        let _ = persist_tree_state(&state.expanded_profile_ids);
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
        if state.expanded_profile_ids.contains(profile_id) {
            state.expanded_profile_ids.remove(profile_id);
        } else {
            state.expanded_profile_ids.insert(profile_id.to_string());
        }
        state.seed_active_profile_on_first_snapshot = false;
        let seed_active_profile_on_first_snapshot = state.seed_active_profile_on_first_snapshot;
        let mut next = state.snapshot.clone();
        apply_expanded_profiles(
            &mut next,
            &mut state.expanded_profile_ids,
            seed_active_profile_on_first_snapshot,
        );
        next.version = next.version.saturating_add(1);
        state.snapshot = next;
        let _ = persist_tree_state(&state.expanded_profile_ids);
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
        let seed_active_profile_on_first_snapshot = state.seed_active_profile_on_first_snapshot;
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
        changed |= apply_expanded_profiles(
            &mut next,
            &mut state.expanded_profile_ids,
            seed_active_profile_on_first_snapshot,
        );
        if !changed {
            return;
        }
        next.version = next.version.saturating_add(1);
        state.snapshot = next;
        let _ = persist_tree_state(&state.expanded_profile_ids);
    }

    pub fn width_pixels(window_width: usize, dpi: usize) -> usize {
        if !sidebar_enabled_from_env() {
            return 0;
        }
        let scale = (dpi as f32 / 96.0).max(1.0);
        let preferred = SIDEBAR_DEFAULT_WIDTH_PX * scale;
        let min_width = SIDEBAR_MIN_WIDTH_PX * scale;
        preferred
            .min(window_width as f32 * SIDEBAR_MAX_WINDOW_RATIO)
            .max(min_width)
            .round() as usize
    }
}

pub fn sidebar_enabled_from_env() -> bool {
    std::env::var(SIDEBAR_ENABLE_ENV)
        .ok()
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
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
    subscription: &crate::chatminal_runtime::DesktopWorkspaceSubscription,
    shared: &Arc<Mutex<SharedState>>,
) -> Result<(), String> {
    replace_snapshot(shared, subscription.load_sidebar_snapshot()?);
    Ok(())
}

fn replace_workspace(shared: &Arc<Mutex<SharedState>>, workspace: RuntimeWorkspace) {
    let active_profile_id = workspace.active_profile_id.clone();
    let active_session_id = workspace.active_session_id.clone();
    let fallback_sessions = workspace.sessions.clone();
    let sessions =
        load_desktop_sidebar_sessions_from_store(&workspace.profiles, active_session_id.as_deref())
            .unwrap_or_else(|err| {
                log::warn!(
                    "sidebar: using workspace session fallback during apply_workspace: {err}"
                );
                fallback_sessions
                    .into_iter()
                    .map(|session| SidebarSession {
                        profile_id: session.profile_id,
                        is_active: active_session_id.as_deref()
                            == Some(session.session_id.as_str()),
                        session_id: session.session_id,
                        name: session.name,
                        status: format!("{:?}", session.status).to_lowercase(),
                    })
                    .collect()
            });
    let next = SidebarSnapshot {
        active_profile_id: active_profile_id.clone(),
        active_session_id: active_session_id.clone(),
        profiles: workspace
            .profiles
            .into_iter()
            .map(|profile| SidebarProfile {
                is_active: active_profile_id.as_deref() == Some(profile.profile_id.as_str()),
                is_expanded: false,
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
    let seed_active_profile_on_first_snapshot = state.seed_active_profile_on_first_snapshot;
    let expanded_changed = apply_expanded_profiles(
        &mut next,
        &mut state.expanded_profile_ids,
        seed_active_profile_on_first_snapshot,
    );
    if state.seed_active_profile_on_first_snapshot && !state.expanded_profile_ids.is_empty() {
        state.seed_active_profile_on_first_snapshot = false;
        let _ = persist_tree_state(&state.expanded_profile_ids);
    }
    let changed = state.snapshot.active_profile_id != next.active_profile_id
        || state.snapshot.active_session_id != next.active_session_id
        || state.snapshot.profiles != next.profiles
        || state.snapshot.sessions != next.sessions
        || state.snapshot.error != next.error
        || expanded_changed;
    if !changed {
        return;
    }
    next.version = state.snapshot.version.saturating_add(1);
    state.snapshot = next;
}

fn apply_expanded_profiles(
    snapshot: &mut SidebarSnapshot,
    expanded_profile_ids: &mut BTreeSet<String>,
    seed_active_profile_on_first_snapshot: bool,
) -> bool {
    let valid_profile_ids: BTreeSet<String> = snapshot
        .profiles
        .iter()
        .map(|profile| profile.profile_id.clone())
        .collect();
    let original_expanded = expanded_profile_ids.clone();
    expanded_profile_ids.retain(|profile_id| valid_profile_ids.contains(profile_id));
    if seed_active_profile_on_first_snapshot && expanded_profile_ids.is_empty() {
        if let Some(profile_id) = snapshot.active_profile_id.clone() {
            if valid_profile_ids.contains(&profile_id) {
                expanded_profile_ids.insert(profile_id);
            }
        }
    }

    let mut changed = original_expanded != *expanded_profile_ids;
    for profile in &mut snapshot.profiles {
        let is_expanded = expanded_profile_ids.contains(profile.profile_id.as_str());
        if profile.is_expanded != is_expanded {
            profile.is_expanded = is_expanded;
            changed = true;
        }
    }
    changed
}

fn load_tree_state() -> (BTreeSet<String>, bool) {
    let path = sidebar_tree_state_file();
    let Ok(file) = std::fs::File::open(&path) else {
        return (BTreeSet::new(), true);
    };
    let Ok(state) = serde_json::from_reader::<_, PersistedSidebarTreeState>(file) else {
        return (BTreeSet::new(), true);
    };
    (state.expanded_profile_ids, false)
}

fn persist_tree_state(expanded_profile_ids: &BTreeSet<String>) -> Result<(), String> {
    std::fs::create_dir_all(config::DATA_DIR.as_path())
        .map_err(|err| format!("create sidebar data dir failed: {err}"))?;
    let payload = PersistedSidebarTreeState {
        expanded_profile_ids: expanded_profile_ids.clone(),
    };
    let json = serde_json::to_vec_pretty(&payload)
        .map_err(|err| format!("serialize sidebar tree state failed: {err}"))?;
    std::fs::write(sidebar_tree_state_file(), json)
        .map_err(|err| format!("write sidebar tree state failed: {err}"))
}

fn sidebar_tree_state_file() -> std::path::PathBuf {
    config::DATA_DIR.join(SIDEBAR_TREE_STATE_FILE)
}

#[cfg(test)]
mod tests {
    use super::{apply_expanded_profiles, SidebarProfile, SidebarSnapshot};
    use std::collections::BTreeSet;

    #[test]
    fn seeds_active_profile_once_when_no_tree_state_exists() {
        let mut snapshot = SidebarSnapshot {
            active_profile_id: Some("profile-a".to_string()),
            profiles: vec![SidebarProfile {
                profile_id: "profile-a".to_string(),
                name: "A".to_string(),
                is_active: true,
                is_expanded: false,
            }],
            ..SidebarSnapshot::default()
        };
        let mut expanded = BTreeSet::new();

        let changed = apply_expanded_profiles(&mut snapshot, &mut expanded, true);

        assert!(changed);
        assert!(expanded.contains("profile-a"));
        assert!(snapshot.profiles[0].is_expanded);
    }

    #[test]
    fn prunes_removed_profiles_from_expanded_state() {
        let mut snapshot = SidebarSnapshot {
            profiles: vec![SidebarProfile {
                profile_id: "profile-a".to_string(),
                name: "A".to_string(),
                is_active: false,
                is_expanded: false,
            }],
            ..SidebarSnapshot::default()
        };
        let mut expanded = BTreeSet::from(["profile-a".to_string(), "profile-b".to_string()]);

        let changed = apply_expanded_profiles(&mut snapshot, &mut expanded, false);

        assert!(changed);
        assert_eq!(expanded, BTreeSet::from(["profile-a".to_string()]));
        assert!(snapshot.profiles[0].is_expanded);
    }
}
