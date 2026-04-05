use std::sync::Arc;
use std::time::Duration;

use chatminal_runtime::{
    RuntimeCreatedSession, RuntimeEvent, RuntimeProfile, RuntimeSubscription, RuntimeWorkspace,
};
use chatminal_runtime::{RuntimeSessionLaunchSpec, RuntimeSessionSnapshot};

use crate::desktop_session_host::EmbeddedRuntime;

use super::{DesktopSessionBridgeAction, DesktopSessionLookup, RuntimeId};

pub(crate) struct ChatminalRuntimeClient {
    runtime: Arc<EmbeddedRuntime>,
    subscription: RuntimeSubscription,
}

impl ChatminalRuntimeClient {
    pub(crate) fn new(runtime: Arc<EmbeddedRuntime>) -> Result<Self, String> {
        let subscription = runtime.state.subscribe()?;
        Ok(Self {
            runtime,
            subscription,
        })
    }

    pub(crate) fn workspace_load_passive(&self) -> Result<RuntimeWorkspace, String> {
        self.runtime.state.workspace_load_passive()
    }

    pub(crate) fn session_activate(
        &self,
        session_id: &str,
        cols: usize,
        rows: usize,
    ) -> Result<(), String> {
        self.runtime.state.session_activate(session_id, cols, rows)
    }

    pub(crate) fn session_close(&self, session_id: &str) -> Result<(), String> {
        self.runtime.state.session_close(session_id)
    }

    pub(crate) fn session_create(
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

    #[allow(dead_code)]
    pub(crate) fn session_move_to_profile(
        &self,
        session_id: &str,
        profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<RuntimeWorkspace, String> {
        self.runtime
            .state
            .session_move_to_profile(session_id, profile_id, target_index)
    }

    pub(crate) fn sessions_move_to_profile(
        &self,
        session_ids: &[String],
        profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<RuntimeWorkspace, String> {
        self.runtime
            .state
            .sessions_move_to_profile(session_ids, profile_id, target_index)
    }

    pub(crate) fn session_rename(
        &self,
        session_id: &str,
        name: &str,
    ) -> Result<RuntimeWorkspace, String> {
        self.runtime.state.session_rename(session_id, name)
    }

    pub(crate) fn session_set_startup_command(
        &self,
        session_id: &str,
        startup_command: Option<&str>,
    ) -> Result<RuntimeWorkspace, String> {
        self.runtime
            .state
            .session_set_startup_command(session_id, startup_command)
    }

    pub(crate) fn session_run_startup_command(&self, session_id: &str) -> Result<(), String> {
        self.runtime.state.session_run_startup_command(session_id)
    }

    pub(crate) fn profile_switch(&self, profile_id: &str) -> Result<RuntimeWorkspace, String> {
        self.runtime.state.profile_switch(profile_id)
    }

    pub(crate) fn profile_create(&self, name: Option<String>) -> Result<RuntimeProfile, String> {
        self.runtime.state.profile_create(name)
    }

    pub(crate) fn session_restore_snapshot_get(
        &self,
        session_id: &str,
    ) -> Result<RuntimeSessionSnapshot, String> {
        self.runtime.state.session_restore_snapshot_get(session_id)
    }

    pub(crate) fn session_launch_spec(
        &self,
        session_id: &str,
    ) -> Result<RuntimeSessionLaunchSpec, String> {
        self.runtime.state.session_launch_spec(session_id)
    }

    pub(crate) fn recv_event(&self, timeout: Duration) -> Result<Option<RuntimeEvent>, String> {
        self.subscription.recv_timeout(timeout)
    }

    pub(crate) fn reconcile_session_lookup(
        &self,
        lookup: &DesktopSessionLookup,
    ) -> Result<DesktopSessionBridgeAction, String> {
        self.runtime.state.reconcile_session_lookup(lookup)
    }

    pub(crate) fn notify_session_activated(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
    ) -> Result<(), String> {
        self.runtime
            .state
            .notify_session_activated(session_id, runtime_id)
    }

    pub(crate) fn notify_session_closed(
        &self,
        session_id: &str,
        runtime_id: RuntimeId,
        lookup_after_close: &DesktopSessionLookup,
    ) -> Result<(), String> {
        self.runtime
            .state
            .notify_session_closed(session_id, runtime_id, lookup_after_close)
    }
}

pub(crate) fn resolve_target_session_id(
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
        .session_create(Some("Shell".to_string()), 120, 32, None, Some(true))
        .map(|value| value.session_id)
}
