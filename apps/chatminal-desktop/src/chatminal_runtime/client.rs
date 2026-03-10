use std::sync::Arc;
use std::time::Duration;

use chatminal_runtime::{
    RuntimeCreatedSession, RuntimeEvent, RuntimeProfile, RuntimeSessionSnapshot,
    RuntimeSubscription, RuntimeWorkspace,
};

use super::EmbeddedRuntime;

pub struct ChatminalRuntimeClient {
    runtime: Arc<EmbeddedRuntime>,
    subscription: RuntimeSubscription,
}

impl ChatminalRuntimeClient {
    pub fn new(runtime: Arc<EmbeddedRuntime>) -> Result<Self, String> {
        let subscription = runtime.state.subscribe()?;
        Ok(Self {
            runtime,
            subscription,
        })
    }

    pub fn workspace_load_passive(&self) -> Result<RuntimeWorkspace, String> {
        self.runtime.state.workspace_load_passive()
    }

    pub fn session_activate(
        &self,
        session_id: &str,
        cols: usize,
        rows: usize,
    ) -> Result<(), String> {
        self.runtime.state.session_activate(session_id, cols, rows)
    }

    pub fn session_create(
        &self,
        name: Option<String>,
        cols: usize,
        rows: usize,
        cwd: Option<String>,
        persist_history: Option<bool>,
    ) -> Result<RuntimeCreatedSession, String> {
        self.runtime
            .state
            .session_create(name, cols, rows, cwd, persist_history)
    }

    pub fn profile_switch(&self, profile_id: &str) -> Result<RuntimeWorkspace, String> {
        self.runtime.state.profile_switch(profile_id)
    }

    pub fn profile_create(&self, name: Option<String>) -> Result<RuntimeProfile, String> {
        self.runtime.state.profile_create(name)
    }

    pub fn session_snapshot_get(
        &self,
        session_id: &str,
        preview_lines: Option<usize>,
    ) -> Result<RuntimeSessionSnapshot, String> {
        self.runtime
            .state
            .session_snapshot_get(session_id, preview_lines)
    }

    pub fn recv_event(&self, timeout: Duration) -> Result<Option<RuntimeEvent>, String> {
        self.subscription.recv_timeout(timeout)
    }
}

pub fn resolve_target_session_id(
    client: &ChatminalRuntimeClient,
    explicit: Option<&str>,
) -> Result<String, String> {
    if let Some(value) = explicit {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let workspace = client.workspace_load_passive()?;
    workspace
        .active_session_id
        .clone()
        .or_else(|| {
            workspace
                .sessions
                .first()
                .map(|value| value.session_id.clone())
        })
        .map(Ok)
        .unwrap_or_else(|| create_default_session(client))
}

fn create_default_session(client: &ChatminalRuntimeClient) -> Result<String, String> {
    client
        .session_create(Some("Shell".to_string()), 120, 32, None, Some(false))
        .map(|value| value.session_id)
}
