use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chatminal_runtime::{RuntimeCreatedSession, RuntimeProfile, RuntimeWorkspace};

use crate::chatminal_runtime::DESKTOP_LAYOUT_WORKSPACE_ID;
use crate::chatminal_runtime::{
    activate_runtime_session, close_runtime_session, create_runtime_profile,
    create_runtime_session, desktop_workspace_subscribe, switch_runtime_profile,
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

#[derive(Debug, Default)]
struct SharedState {
    snapshot: SidebarSnapshot,
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
        activate_runtime_session(session_id, cols, rows)
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
        sessions: workspace
            .sessions
            .into_iter()
            .map(|session| SidebarSession {
                is_active: active_session_id.as_deref() == Some(session.session_id.as_str()),
                session_id: session.session_id,
                name: session.name,
                status: format!("{:?}", session.status).to_lowercase(),
            })
            .collect(),
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
