use chatminal_runtime::SessionTerminalHandle;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

static CLIENT_ID: AtomicUsize = AtomicUsize::new(0);
lazy_static::lazy_static! {
    static ref EPOCH: u64 = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap().as_secs();
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClientId {
    pub hostname: String,
    pub username: String,
    pub pid: u32,
    pub epoch: u64,
    pub id: usize,
    pub ssh_auth_sock: Option<String>,
}

impl ClientId {
    pub fn new() -> Self {
        let id = CLIENT_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            hostname: hostname::get()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|_| "localhost".to_string()),
            username: config::username_from_env().unwrap_or_else(|_| "somebody".to_string()),
            pid: unsafe { libc::getpid() as u32 },
            epoch: *EPOCH,
            id,
            ssh_auth_sock: std::env::var("SSH_AUTH_SOCK").ok(),
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct ClientInfo {
    pub client_id: Arc<ClientId>,
    /// The time this client last connected
    #[serde(with = "ts_seconds")]
    pub connected_at: DateTime<Utc>,
    /// Which workspace is active
    pub active_workspace: Option<String>,
    /// The last time we received input from this client
    #[serde(with = "ts_seconds")]
    pub last_input: DateTime<Utc>,
    /// The currently-focused pane
    #[serde(rename = "focused_pane_id", alias = "focused_terminal_handle")]
    focused_terminal_handle: Option<SessionTerminalHandle>,
}

impl ClientInfo {
    pub(crate) fn new(client_id: Arc<ClientId>) -> Self {
        Self {
            client_id,
            connected_at: Utc::now(),
            active_workspace: None,
            last_input: Utc::now(),
            focused_terminal_handle: None,
        }
    }

    pub(crate) fn update_last_input(&mut self) {
        self.last_input = Utc::now();
    }

    pub(crate) fn focused_terminal_handle(&self) -> Option<SessionTerminalHandle> {
        self.focused_terminal_handle
    }

    pub(crate) fn update_focused_terminal_handle(
        &mut self,
        terminal_handle: SessionTerminalHandle,
    ) {
        self.focused_terminal_handle.replace(terminal_handle);
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientId, ClientInfo};
    use chatminal_runtime::SessionTerminalHandle;
    use chrono::Utc;
    use std::sync::Arc;

    #[test]
    fn client_id_snapshots_ssh_auth_sock_from_environment() {
        let previous = std::env::var_os("SSH_AUTH_SOCK");
        std::env::set_var("SSH_AUTH_SOCK", "/tmp/chatminal-test-agent.sock");

        let client_id = ClientId::new();

        match previous {
            Some(value) => std::env::set_var("SSH_AUTH_SOCK", value),
            None => std::env::remove_var("SSH_AUTH_SOCK"),
        }

        assert_eq!(
            client_id.ssh_auth_sock.as_deref(),
            Some("/tmp/chatminal-test-agent.sock")
        );
    }

    #[test]
    fn client_info_keeps_legacy_focused_pane_id_wire_name() {
        let client_id = Arc::new(ClientId::new());
        let mut info = ClientInfo {
            client_id,
            connected_at: Utc::now(),
            active_workspace: Some("default".to_string()),
            last_input: Utc::now(),
            focused_terminal_handle: None,
        };
        info.update_focused_terminal_handle(SessionTerminalHandle::new(17));

        let json = serde_json::to_value(&info).expect("serialize client info");

        assert_eq!(
            json.get("focused_pane_id").and_then(|value| value.as_u64()),
            Some(17)
        );
        assert!(json.get("focused_terminal_handle").is_none());
    }

    #[test]
    fn client_info_accepts_new_focus_field_alias_when_deserializing() {
        let json = serde_json::json!({
            "client_id": {
                "hostname": "localhost",
                "username": "khoa",
                "pid": 1,
                "epoch": 2,
                "id": 3,
                "ssh_auth_sock": null
            },
            "connected_at": 0,
            "active_workspace": "default",
            "last_input": 0,
            "focused_terminal_handle": 29
        });

        let info: ClientInfo = serde_json::from_value(json).expect("deserialize client info");

        assert_eq!(
            info.focused_terminal_handle(),
            Some(SessionTerminalHandle::new(29))
        );
    }
}
