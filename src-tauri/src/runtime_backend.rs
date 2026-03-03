use crate::chatminald_client::{ChatminaldClient, DaemonPingStatus};
use crate::models::{RuntimeBackendInfo, RuntimeBackendMode, RuntimeBackendPing, RuntimeOwner};

#[derive(Debug, Clone)]
pub struct RuntimeBackend {
    requested_mode: RuntimeBackendMode,
    runtime_owner: RuntimeOwner,
    daemon_client: Option<ChatminaldClient>,
    startup_probe: Option<DaemonPingStatus>,
}

impl RuntimeBackend {
    pub fn from_env() -> Self {
        let raw_mode = std::env::var("CHATMINAL_RUNTIME_BACKEND")
            .unwrap_or_else(|_| std::env::var("CHATMIMAL_RUNTIME_BACKEND").unwrap_or_default());
        let daemon_requested = matches!(
            raw_mode.trim().to_ascii_lowercase().as_str(),
            "daemon" | "external" | "chatminald"
        );

        let daemon_client = ChatminaldClient::from_env();
        let requested_mode = if daemon_requested {
            RuntimeBackendMode::Daemon
        } else {
            RuntimeBackendMode::InProcess
        };
        let startup_probe = if daemon_requested {
            daemon_client.as_ref().map(ChatminaldClient::ping)
        } else {
            None
        };

        // Safe-slice rule: until daemon proxy is fully wired, commands always execute in-process.
        let runtime_owner = RuntimeOwner::InProcess;

        Self {
            requested_mode,
            runtime_owner,
            daemon_client,
            startup_probe,
        }
    }

    pub fn info(&self) -> RuntimeBackendInfo {
        let daemon_endpoint = self
            .daemon_client
            .as_ref()
            .map(|client| client.endpoint().to_string());

        let note = match self.requested_mode {
            RuntimeBackendMode::InProcess => {
                "PTY runtime is handled in-process by Tauri backend".to_string()
            }
            RuntimeBackendMode::Daemon => {
                if daemon_endpoint.is_none() {
                    "daemon mode requested but CHATMINAL_DAEMON_ENDPOINT is missing".to_string()
                } else if let Some(probe) = &self.startup_probe {
                    if probe.reachable {
                        "daemon reachable at startup; runtime owner is still in_process until cutover is enabled".to_string()
                    } else {
                        format!(
                            "daemon mode requested but probe failed; fallback owner=in_process ({})",
                            probe.message
                        )
                    }
                } else {
                    "daemon mode requested; startup probe unavailable; fallback owner=in_process"
                        .to_string()
                }
            }
        };

        RuntimeBackendInfo {
            requested_mode: self.requested_mode,
            runtime_owner: self.runtime_owner,
            daemon_endpoint,
            note,
        }
    }

    pub fn ping(&self) -> RuntimeBackendPing {
        let daemon_endpoint = self
            .daemon_client
            .as_ref()
            .map(|client| client.endpoint().to_string());

        if self.requested_mode == RuntimeBackendMode::InProcess {
            return RuntimeBackendPing {
                requested_mode: self.requested_mode,
                runtime_owner: self.runtime_owner,
                daemon_endpoint,
                reachable: false,
                latency_ms: None,
                message: "daemon ping skipped: backend is in_process".to_string(),
            };
        }

        let Some(client) = &self.daemon_client else {
            return RuntimeBackendPing {
                requested_mode: self.requested_mode,
                runtime_owner: self.runtime_owner,
                daemon_endpoint,
                reachable: false,
                latency_ms: None,
                message: "daemon endpoint is not configured".to_string(),
            };
        };

        let ping = client.ping();
        RuntimeBackendPing {
            requested_mode: self.requested_mode,
            runtime_owner: self.runtime_owner,
            daemon_endpoint,
            reachable: ping.reachable,
            latency_ms: ping.latency_ms,
            message: ping.message,
        }
    }
}
