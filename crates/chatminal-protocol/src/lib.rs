use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub profile_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub status: SessionStatus,
    pub persist_history: bool,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub profiles: Vec<ProfileInfo>,
    pub active_profile_id: Option<String>,
    pub sessions: Vec<SessionInfo>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub content: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExplorerState {
    pub session_id: String,
    pub root_path: Option<String>,
    pub current_dir: String,
    pub selected_path: Option<String>,
    pub open_file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExplorerEntry {
    pub name: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExplorerFileContent {
    pub relative_path: String,
    pub content: String,
    pub truncated: bool,
    pub byte_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyOutputEvent {
    pub session_id: String,
    pub chunk: String,
    pub seq: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyExitedEvent {
    pub session_id: String,
    pub exit_code: Option<i32>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyErrorEvent {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdatedEvent {
    pub session_id: String,
    pub status: SessionStatus,
    pub seq: u64,
    pub persist_history: bool,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceUpdatedEvent {
    pub active_profile_id: Option<String>,
    pub active_session_id: Option<String>,
    pub profile_count: u64,
    pub session_count: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonHealthEvent {
    pub connected_clients: u64,
    pub session_count: u64,
    pub running_sessions: u64,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecyclePreferences {
    pub keep_alive_on_close: bool,
    pub start_in_tray: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientFrame {
    pub id: String,
    pub request: Request,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Request {
    Ping,
    LifecyclePreferencesGet,
    LifecyclePreferencesSet {
        keep_alive_on_close: Option<bool>,
        start_in_tray: Option<bool>,
    },
    WorkspaceLoad,
    ProfileList,
    ProfileCreate {
        name: Option<String>,
    },
    ProfileRename {
        profile_id: String,
        name: String,
    },
    ProfileDelete {
        profile_id: String,
    },
    ProfileSwitch {
        profile_id: String,
    },
    SessionList,
    SessionCreate {
        name: Option<String>,
        cols: usize,
        rows: usize,
        cwd: Option<String>,
        persist_history: Option<bool>,
    },
    SessionActivate {
        session_id: String,
        cols: usize,
        rows: usize,
    },
    SessionRename {
        session_id: String,
        name: String,
    },
    SessionClose {
        session_id: String,
    },
    SessionSetPersist {
        session_id: String,
        persist_history: bool,
    },
    SessionInputWrite {
        session_id: String,
        data: String,
    },
    SessionResize {
        session_id: String,
        cols: usize,
        rows: usize,
    },
    SessionSnapshotGet {
        session_id: String,
        preview_lines: Option<usize>,
    },
    SessionExplorerStateGet {
        session_id: String,
    },
    SessionExplorerRootSet {
        session_id: String,
        root_path: String,
    },
    SessionExplorerStateUpdate {
        session_id: String,
        current_dir: String,
        selected_path: Option<String>,
        open_file_path: Option<String>,
    },
    SessionExplorerList {
        session_id: String,
        relative_path: Option<String>,
    },
    SessionExplorerReadFile {
        session_id: String,
        relative_path: String,
        max_bytes: Option<usize>,
    },
    SessionHistoryClear {
        session_id: String,
    },
    WorkspaceHistoryClearAll,
    AppShutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFrame {
    pub id: Option<String>,
    #[serde(flatten)]
    pub body: ServerBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerBody {
    Response {
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<Response>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Event {
        event: Event,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Response {
    Ping(PingResponse),
    LifecyclePreferences(LifecyclePreferences),
    Workspace(WorkspaceState),
    Profiles(Vec<ProfileInfo>),
    Profile(ProfileInfo),
    Sessions(Vec<SessionInfo>),
    SessionCreate(CreateSessionResponse),
    SessionSnapshot(SessionSnapshot),
    SessionExplorerState(SessionExplorerState),
    SessionExplorerEntries(Vec<SessionExplorerEntry>),
    SessionExplorerFileContent(SessionExplorerFileContent),
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Event {
    PtyOutput(PtyOutputEvent),
    PtyExited(PtyExitedEvent),
    PtyError(PtyErrorEvent),
    SessionUpdated(SessionUpdatedEvent),
    WorkspaceUpdated(WorkspaceUpdatedEvent),
    DaemonHealth(DaemonHealthEvent),
}

impl ServerFrame {
    pub fn ok(id: String, response: Response) -> Self {
        Self {
            id: Some(id),
            body: ServerBody::Response {
                ok: true,
                response: Some(response),
                error: None,
            },
        }
    }

    pub fn ok_empty(id: String) -> Self {
        Self {
            id: Some(id),
            body: ServerBody::Response {
                ok: true,
                response: Some(Response::Empty),
                error: None,
            },
        }
    }

    pub fn err(id: String, message: String) -> Self {
        Self {
            id: Some(id),
            body: ServerBody::Response {
                ok: false,
                response: None,
                error: Some(message),
            },
        }
    }

    pub fn event(event: Event) -> Self {
        Self {
            id: None,
            body: ServerBody::Event { event },
        }
    }
}
