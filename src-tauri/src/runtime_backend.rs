use crate::chatminald_client::ChatminaldClient;
use crate::models::{RuntimeBackendInfo, RuntimeBackendMode, RuntimeBackendPing};

#[derive(Debug, Clone)]
pub struct RuntimeBackend {
    mode: RuntimeBackendMode,
    daemon_client: Option<ChatminaldClient>,
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
        let mode = if daemon_requested {
            RuntimeBackendMode::Daemon
        } else {
            RuntimeBackendMode::InProcess
        };

        Self {
            mode,
            daemon_client,
        }
    }

    pub fn info(&self) -> RuntimeBackendInfo {
        let daemon_endpoint = self
            .daemon_client
            .as_ref()
            .map(|client| client.endpoint().to_string());

        let note = match self.mode {
            RuntimeBackendMode::InProcess => {
                "PTY runtime is handled in-process by Tauri backend".to_string()
            }
            RuntimeBackendMode::Daemon => {
                if daemon_endpoint.is_some() {
                    "daemon mode requested; run ping_runtime_backend to verify reachability"
                        .to_string()
                } else {
                    "daemon mode requested but CHATMINAL_DAEMON_ENDPOINT is missing".to_string()
                }
            }
        };

        RuntimeBackendInfo {
            mode: self.mode,
            daemon_endpoint,
            note,
        }
    }

    pub fn ping(&self) -> RuntimeBackendPing {
        let daemon_endpoint = self
            .daemon_client
            .as_ref()
            .map(|client| client.endpoint().to_string());

        if self.mode == RuntimeBackendMode::InProcess {
            return RuntimeBackendPing {
                mode: self.mode,
                daemon_endpoint,
                reachable: false,
                latency_ms: None,
                message: "daemon ping skipped: backend is in_process".to_string(),
            };
        }

        let Some(client) = &self.daemon_client else {
            return RuntimeBackendPing {
                mode: self.mode,
                daemon_endpoint,
                reachable: false,
                latency_ms: None,
                message: "daemon endpoint is not configured".to_string(),
            };
        };

        let ping = client.ping();
        RuntimeBackendPing {
            mode: self.mode,
            daemon_endpoint,
            reachable: ping.reachable,
            latency_ms: ping.latency_ms,
            message: ping.message,
        }
    }
}
