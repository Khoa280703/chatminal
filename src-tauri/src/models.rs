use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Disconnected,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub name: String,
    pub cwd: String,
    pub status: SessionStatus,
    pub persist_history: bool,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub profile_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProfilePayload {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwitchProfilePayload {
    pub profile_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenameProfilePayload {
    pub profile_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteProfilePayload {
    pub profile_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionPayload {
    pub name: Option<String>,
    pub cols: usize,
    pub rows: usize,
    pub cwd: Option<String>,
    pub persist_history: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActivateSessionPayload {
    pub session_id: String,
    pub cols: usize,
    pub rows: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionActionPayload {
    pub session_id: String,
    pub preview_lines: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WriteInputPayload {
    pub session_id: String,
    pub data: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResizeSessionPayload {
    pub session_id: String,
    pub cols: usize,
    pub rows: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenameSessionPayload {
    pub session_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSessionPersistPayload {
    pub session_id: String,
    pub persist_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecyclePreferences {
    pub keep_alive_on_close: bool,
    pub start_in_tray: bool,
}

impl Default for LifecyclePreferences {
    fn default() -> Self {
        Self {
            keep_alive_on_close: true,
            start_in_tray: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetLifecyclePreferencesPayload {
    pub keep_alive_on_close: Option<bool>,
    pub start_in_tray: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSnapshot {
    pub content: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceState {
    pub profiles: Vec<ProfileInfo>,
    pub active_profile_id: Option<String>,
    pub sessions: Vec<SessionInfo>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtyOutputEvent {
    pub session_id: String,
    pub chunk: String,
    pub seq: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtyExitedEvent {
    pub session_id: String,
    pub exit_code: Option<i32>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtyErrorEvent {
    pub session_id: String,
    pub message: String,
}
