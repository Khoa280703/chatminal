use chatminal_protocol::{
    CreateSessionResponse, DaemonHealthEvent, Event, LifecyclePreferences, ProfileInfo,
    PtyErrorEvent, PtyExitedEvent, PtyOutputEvent, SessionExplorerEntry,
    SessionExplorerFileContent, SessionExplorerState, SessionInfo, SessionSnapshot, SessionStatus,
    SessionUpdatedEvent, WorkspaceState, WorkspaceUpdatedEvent,
};
use chatminal_store::{
    StoredProfile, StoredSessionSnapshot, StoredSessionStatus, StoredSessionSummary,
};

use super::{
    RuntimeCreatedSession, RuntimeDaemonHealthEvent, RuntimeEvent, RuntimeLifecyclePreferences,
    RuntimeProfile, RuntimePtyErrorEvent, RuntimePtyExitedEvent, RuntimePtyOutputEvent,
    RuntimeSession, RuntimeSessionExplorerEntry, RuntimeSessionExplorerFileContent,
    RuntimeSessionExplorerState, RuntimeSessionSnapshot, RuntimeSessionStatus,
    RuntimeSessionUpdatedEvent, RuntimeWorkspace, RuntimeWorkspaceUpdatedEvent,
};

impl From<SessionStatus> for RuntimeSessionStatus {
    fn from(value: SessionStatus) -> Self {
        match value {
            SessionStatus::Running => Self::Running,
            SessionStatus::Disconnected => Self::Disconnected,
        }
    }
}

impl From<RuntimeSessionStatus> for SessionStatus {
    fn from(value: RuntimeSessionStatus) -> Self {
        match value {
            RuntimeSessionStatus::Running => Self::Running,
            RuntimeSessionStatus::Disconnected => Self::Disconnected,
        }
    }
}

impl From<StoredSessionStatus> for RuntimeSessionStatus {
    fn from(value: StoredSessionStatus) -> Self {
        match value {
            StoredSessionStatus::Running => Self::Running,
            StoredSessionStatus::Disconnected => Self::Disconnected,
        }
    }
}

impl From<RuntimeSessionStatus> for StoredSessionStatus {
    fn from(value: RuntimeSessionStatus) -> Self {
        match value {
            RuntimeSessionStatus::Running => Self::Running,
            RuntimeSessionStatus::Disconnected => Self::Disconnected,
        }
    }
}

impl From<ProfileInfo> for RuntimeProfile {
    fn from(value: ProfileInfo) -> Self {
        Self {
            profile_id: value.profile_id,
            name: value.name,
        }
    }
}

impl From<RuntimeProfile> for ProfileInfo {
    fn from(value: RuntimeProfile) -> Self {
        Self {
            profile_id: value.profile_id,
            name: value.name,
        }
    }
}

impl From<StoredProfile> for RuntimeProfile {
    fn from(value: StoredProfile) -> Self {
        Self {
            profile_id: value.profile_id,
            name: value.name,
        }
    }
}

impl From<SessionInfo> for RuntimeSession {
    fn from(value: SessionInfo) -> Self {
        Self {
            session_id: value.session_id,
            profile_id: value.profile_id,
            name: value.name,
            cwd: value.cwd,
            status: value.status.into(),
            persist_history: value.persist_history,
            seq: value.seq,
        }
    }
}

impl From<RuntimeSession> for SessionInfo {
    fn from(value: RuntimeSession) -> Self {
        Self {
            session_id: value.session_id,
            profile_id: value.profile_id,
            name: value.name,
            cwd: value.cwd,
            status: value.status.into(),
            persist_history: value.persist_history,
            seq: value.seq,
        }
    }
}

impl From<StoredSessionSummary> for RuntimeSession {
    fn from(value: StoredSessionSummary) -> Self {
        Self {
            session_id: value.session_id,
            profile_id: value.profile_id,
            name: value.name,
            cwd: value.cwd,
            status: value.status.into(),
            persist_history: value.persist_history,
            seq: value.seq,
        }
    }
}

impl From<WorkspaceState> for RuntimeWorkspace {
    fn from(value: WorkspaceState) -> Self {
        Self {
            profiles: value.profiles.into_iter().map(Into::into).collect(),
            active_profile_id: value.active_profile_id,
            sessions: value.sessions.into_iter().map(Into::into).collect(),
            active_session_id: value.active_session_id,
        }
    }
}

impl From<RuntimeWorkspace> for WorkspaceState {
    fn from(value: RuntimeWorkspace) -> Self {
        Self {
            profiles: value.profiles.into_iter().map(Into::into).collect(),
            active_profile_id: value.active_profile_id,
            sessions: value.sessions.into_iter().map(Into::into).collect(),
            active_session_id: value.active_session_id,
        }
    }
}

impl From<CreateSessionResponse> for RuntimeCreatedSession {
    fn from(value: CreateSessionResponse) -> Self {
        Self {
            session_id: value.session_id,
            name: value.name,
        }
    }
}

impl From<RuntimeCreatedSession> for CreateSessionResponse {
    fn from(value: RuntimeCreatedSession) -> Self {
        Self {
            session_id: value.session_id,
            name: value.name,
        }
    }
}

impl From<LifecyclePreferences> for RuntimeLifecyclePreferences {
    fn from(value: LifecyclePreferences) -> Self {
        Self {
            keep_alive_on_close: value.keep_alive_on_close,
            start_in_tray: value.start_in_tray,
        }
    }
}

impl From<RuntimeLifecyclePreferences> for LifecyclePreferences {
    fn from(value: RuntimeLifecyclePreferences) -> Self {
        Self {
            keep_alive_on_close: value.keep_alive_on_close,
            start_in_tray: value.start_in_tray,
        }
    }
}

impl From<SessionSnapshot> for RuntimeSessionSnapshot {
    fn from(value: SessionSnapshot) -> Self {
        Self {
            content: value.content,
            seq: value.seq,
        }
    }
}

impl From<RuntimeSessionSnapshot> for SessionSnapshot {
    fn from(value: RuntimeSessionSnapshot) -> Self {
        Self {
            content: value.content,
            seq: value.seq,
        }
    }
}

impl From<StoredSessionSnapshot> for RuntimeSessionSnapshot {
    fn from(value: StoredSessionSnapshot) -> Self {
        Self {
            content: value.content,
            seq: value.seq,
        }
    }
}

impl From<SessionExplorerState> for RuntimeSessionExplorerState {
    fn from(value: SessionExplorerState) -> Self {
        Self {
            session_id: value.session_id,
            root_path: value.root_path,
            current_dir: value.current_dir,
            selected_path: value.selected_path,
            open_file_path: value.open_file_path,
        }
    }
}

impl From<RuntimeSessionExplorerState> for SessionExplorerState {
    fn from(value: RuntimeSessionExplorerState) -> Self {
        Self {
            session_id: value.session_id,
            root_path: value.root_path,
            current_dir: value.current_dir,
            selected_path: value.selected_path,
            open_file_path: value.open_file_path,
        }
    }
}

impl From<SessionExplorerEntry> for RuntimeSessionExplorerEntry {
    fn from(value: SessionExplorerEntry) -> Self {
        Self {
            name: value.name,
            relative_path: value.relative_path,
            is_dir: value.is_dir,
            size: value.size,
        }
    }
}

impl From<RuntimeSessionExplorerEntry> for SessionExplorerEntry {
    fn from(value: RuntimeSessionExplorerEntry) -> Self {
        Self {
            name: value.name,
            relative_path: value.relative_path,
            is_dir: value.is_dir,
            size: value.size,
        }
    }
}

impl From<SessionExplorerFileContent> for RuntimeSessionExplorerFileContent {
    fn from(value: SessionExplorerFileContent) -> Self {
        Self {
            relative_path: value.relative_path,
            content: value.content,
            truncated: value.truncated,
            byte_len: value.byte_len,
        }
    }
}

impl From<RuntimeSessionExplorerFileContent> for SessionExplorerFileContent {
    fn from(value: RuntimeSessionExplorerFileContent) -> Self {
        Self {
            relative_path: value.relative_path,
            content: value.content,
            truncated: value.truncated,
            byte_len: value.byte_len,
        }
    }
}

impl From<PtyOutputEvent> for RuntimePtyOutputEvent {
    fn from(value: PtyOutputEvent) -> Self {
        Self {
            session_id: value.session_id,
            chunk: value.chunk,
            seq: value.seq,
            ts: value.ts,
        }
    }
}

impl From<RuntimePtyOutputEvent> for PtyOutputEvent {
    fn from(value: RuntimePtyOutputEvent) -> Self {
        Self {
            session_id: value.session_id,
            chunk: value.chunk,
            seq: value.seq,
            ts: value.ts,
        }
    }
}

impl From<PtyExitedEvent> for RuntimePtyExitedEvent {
    fn from(value: PtyExitedEvent) -> Self {
        Self {
            session_id: value.session_id,
            exit_code: value.exit_code,
            reason: value.reason,
        }
    }
}

impl From<RuntimePtyExitedEvent> for PtyExitedEvent {
    fn from(value: RuntimePtyExitedEvent) -> Self {
        Self {
            session_id: value.session_id,
            exit_code: value.exit_code,
            reason: value.reason,
        }
    }
}

impl From<PtyErrorEvent> for RuntimePtyErrorEvent {
    fn from(value: PtyErrorEvent) -> Self {
        Self {
            session_id: value.session_id,
            message: value.message,
        }
    }
}

impl From<RuntimePtyErrorEvent> for PtyErrorEvent {
    fn from(value: RuntimePtyErrorEvent) -> Self {
        Self {
            session_id: value.session_id,
            message: value.message,
        }
    }
}

impl From<SessionUpdatedEvent> for RuntimeSessionUpdatedEvent {
    fn from(value: SessionUpdatedEvent) -> Self {
        Self {
            session_id: value.session_id,
            status: value.status.into(),
            seq: value.seq,
            persist_history: value.persist_history,
            ts: value.ts,
        }
    }
}

impl From<RuntimeSessionUpdatedEvent> for SessionUpdatedEvent {
    fn from(value: RuntimeSessionUpdatedEvent) -> Self {
        Self {
            session_id: value.session_id,
            status: value.status.into(),
            seq: value.seq,
            persist_history: value.persist_history,
            ts: value.ts,
        }
    }
}

impl From<WorkspaceUpdatedEvent> for RuntimeWorkspaceUpdatedEvent {
    fn from(value: WorkspaceUpdatedEvent) -> Self {
        Self {
            active_profile_id: value.active_profile_id,
            active_session_id: value.active_session_id,
            profile_count: value.profile_count,
            session_count: value.session_count,
            ts: value.ts,
        }
    }
}

impl From<RuntimeWorkspaceUpdatedEvent> for WorkspaceUpdatedEvent {
    fn from(value: RuntimeWorkspaceUpdatedEvent) -> Self {
        Self {
            active_profile_id: value.active_profile_id,
            active_session_id: value.active_session_id,
            profile_count: value.profile_count,
            session_count: value.session_count,
            ts: value.ts,
        }
    }
}

impl From<DaemonHealthEvent> for RuntimeDaemonHealthEvent {
    fn from(value: DaemonHealthEvent) -> Self {
        Self {
            connected_clients: value.connected_clients,
            session_count: value.session_count,
            running_sessions: value.running_sessions,
            ts: value.ts,
        }
    }
}

impl From<RuntimeDaemonHealthEvent> for DaemonHealthEvent {
    fn from(value: RuntimeDaemonHealthEvent) -> Self {
        Self {
            connected_clients: value.connected_clients,
            session_count: value.session_count,
            running_sessions: value.running_sessions,
            ts: value.ts,
        }
    }
}

impl From<Event> for RuntimeEvent {
    fn from(value: Event) -> Self {
        match value {
            Event::PtyOutput(value) => Self::PtyOutput(value.into()),
            Event::PtyExited(value) => Self::PtyExited(value.into()),
            Event::PtyError(value) => Self::PtyError(value.into()),
            Event::SessionUpdated(value) => Self::SessionUpdated(value.into()),
            Event::WorkspaceUpdated(value) => Self::WorkspaceUpdated(value.into()),
            Event::DaemonHealth(value) => Self::DaemonHealth(value.into()),
        }
    }
}

impl From<RuntimeEvent> for Event {
    fn from(value: RuntimeEvent) -> Self {
        match value {
            RuntimeEvent::PtyOutput(value) => Self::PtyOutput(value.into()),
            RuntimeEvent::PtyExited(value) => Self::PtyExited(value.into()),
            RuntimeEvent::PtyError(value) => Self::PtyError(value.into()),
            RuntimeEvent::SessionUpdated(value) => Self::SessionUpdated(value.into()),
            RuntimeEvent::WorkspaceUpdated(value) => Self::WorkspaceUpdated(value.into()),
            RuntimeEvent::DaemonHealth(value) => Self::DaemonHealth(value.into()),
        }
    }
}
