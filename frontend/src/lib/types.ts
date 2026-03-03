export type SessionStatus = "running" | "disconnected";
export type RuntimeBackendMode = "in_process" | "daemon";
export type RuntimeOwner = "in_process" | "daemon";

export interface SessionInfo {
  session_id: string;
  name: string;
  cwd: string;
  status: SessionStatus;
  persist_history: boolean;
  seq: number;
}

export interface ProfileInfo {
  profile_id: string;
  name: string;
}

export interface LifecyclePreferences {
  keep_alive_on_close: boolean;
  start_in_tray: boolean;
}

export interface RuntimeUiSettings {
  sync_clear_command_to_history: boolean;
}

export interface RuntimeBackendInfo {
  requested_mode: RuntimeBackendMode;
  runtime_owner: RuntimeOwner;
  daemon_endpoint: string | null;
  note: string;
}

export interface RuntimeBackendPing {
  requested_mode: RuntimeBackendMode;
  runtime_owner: RuntimeOwner;
  daemon_endpoint: string | null;
  reachable: boolean;
  latency_ms: number | null;
  message: string;
}

export interface WorkspaceState {
  profiles: ProfileInfo[];
  active_profile_id: string | null;
  sessions: SessionInfo[];
  active_session_id: string | null;
}

export interface CreateSessionResponse {
  session_id: string;
  name: string;
}

export interface ActivateSessionPayload {
  session_id: string;
  cols: number;
  rows: number;
}

export interface SessionActionPayload {
  session_id: string;
  preview_lines?: number;
}

export interface SetSessionPersistPayload {
  session_id: string;
  persist_history: boolean;
}

export interface SessionSnapshot {
  content: string;
  seq: number;
}

export interface PtyOutputEvent {
  session_id: string;
  chunk: string;
  seq: number;
  ts: number;
}

export interface PtyExitedEvent {
  session_id: string;
  exit_code: number | null;
  reason: "eof" | "error" | "killed";
}

export interface PtyErrorEvent {
  session_id: string;
  message: string;
}
