mod protocol;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSessionStatus {
    Running,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProfile {
    pub profile_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSession {
    pub session_id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub status: RuntimeSessionStatus,
    pub persist_history: bool,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeWorkspace {
    pub profiles: Vec<RuntimeProfile>,
    pub active_profile_id: Option<String>,
    pub sessions: Vec<RuntimeSession>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCreatedSession {
    pub session_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLifecyclePreferences {
    pub keep_alive_on_close: bool,
    pub start_in_tray: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionSnapshot {
    pub content: String,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionExplorerState {
    pub session_id: String,
    pub root_path: Option<String>,
    pub current_dir: String,
    pub selected_path: Option<String>,
    pub open_file_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionExplorerEntry {
    pub name: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionExplorerFileContent {
    pub relative_path: String,
    pub content: String,
    pub truncated: bool,
    pub byte_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePtyOutputEvent {
    pub session_id: String,
    pub chunk: String,
    pub seq: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePtyExitedEvent {
    pub session_id: String,
    pub exit_code: Option<i32>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePtyErrorEvent {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionUpdatedEvent {
    pub session_id: String,
    pub status: RuntimeSessionStatus,
    pub seq: u64,
    pub persist_history: bool,
    pub ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeWorkspaceUpdatedEvent {
    pub active_profile_id: Option<String>,
    pub active_session_id: Option<String>,
    pub profile_count: u64,
    pub session_count: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDaemonHealthEvent {
    pub connected_clients: u64,
    pub session_count: u64,
    pub running_sessions: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEvent {
    PtyOutput(RuntimePtyOutputEvent),
    PtyExited(RuntimePtyExitedEvent),
    PtyError(RuntimePtyErrorEvent),
    SessionUpdated(RuntimeSessionUpdatedEvent),
    WorkspaceUpdated(RuntimeWorkspaceUpdatedEvent),
    DaemonHealth(RuntimeDaemonHealthEvent),
}
