use std::sync::mpsc as std_mpsc;

use chatminal_store::StoredSessionStatus;

use super::{
    DaemonState, RuntimeHandle, SessionSpawnPlan, StateInner, kill_runtime_handle, now_millis,
    spawn_runtime_handle,
};
use crate::api::{
    RuntimeDaemonHealthEvent, RuntimeEvent, RuntimeSessionStatus, RuntimeSessionUpdatedEvent,
    RuntimeWorkspaceUpdatedEvent,
};

enum SpawnCommitOutcome {
    Committed,
    AlreadyRunning,
    Missing,
    Stale,
}

impl DaemonState {
    pub(super) fn ensure_active_session_runtime(&self) -> Result<(), String> {
        let (store, cols, rows) = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            (
                inner.store.clone(),
                inner.config.default_cols,
                inner.config.default_rows,
            )
        };

        let workspace = store.load_workspace()?;
        let Some(session_id) = workspace.active_session_id else {
            return Ok(());
        };

        let maybe_plan = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let Some(entry) = inner.sessions.get(&session_id) else {
                return Ok(());
            };
            if entry.session.profile_id != workspace.active_profile_id || entry.runtime.is_some() {
                return Ok(());
            }

            Some(SessionSpawnPlan {
                session_id: entry.session.session_id.clone(),
                profile_id: entry.session.profile_id.clone(),
                expected_active_session_id: Some(session_id.clone()),
                expected_generation: entry.generation,
                next_generation: entry.generation.saturating_add(1),
                shell: entry.session.shell.clone(),
                cwd: entry.session.cwd.clone(),
                cols,
                rows,
            })
        };

        let Some(plan) = maybe_plan else {
            return Ok(());
        };

        match self.try_commit_spawned_session(plan)? {
            SpawnCommitOutcome::Committed | SpawnCommitOutcome::AlreadyRunning => Ok(()),
            SpawnCommitOutcome::Missing => Ok(()),
            SpawnCommitOutcome::Stale => {
                Err("session state changed while starting runtime".to_string())
            }
        }
    }

    pub(super) fn commit_spawned_session(&self, plan: SessionSpawnPlan) -> Result<(), String> {
        match self.try_commit_spawned_session(plan)? {
            SpawnCommitOutcome::Committed | SpawnCommitOutcome::AlreadyRunning => Ok(()),
            SpawnCommitOutcome::Missing => Err("session not found".to_string()),
            SpawnCommitOutcome::Stale => {
                Err("session state changed while starting runtime".to_string())
            }
        }
    }

    fn try_commit_spawned_session(
        &self,
        plan: SessionSpawnPlan,
    ) -> Result<SpawnCommitOutcome, String> {
        let runtime = spawn_runtime_handle(
            &plan.session_id,
            plan.next_generation,
            &plan.shell,
            &plan.cwd,
            plan.cols,
            plan.rows,
            self.events.clone(),
        )?;

        let outcome = self.finish_spawn_commit(plan, runtime)?;
        Ok(outcome)
    }

    fn finish_spawn_commit(
        &self,
        plan: SessionSpawnPlan,
        runtime: RuntimeHandle,
    ) -> Result<SpawnCommitOutcome, String> {
        let outcome = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let active_guard_passed =
                if let Some(expected_session_id) = plan.expected_active_session_id.as_deref() {
                    let workspace = inner.store.load_workspace()?;
                    workspace.active_profile_id == plan.profile_id
                        && workspace.active_session_id.as_deref() == Some(expected_session_id)
                } else {
                    true
                };

            if !active_guard_passed {
                SpawnCommitOutcome::Stale
            } else {
                match inner.sessions.get(&plan.session_id) {
                    None => SpawnCommitOutcome::Missing,
                    Some(entry) if entry.session.profile_id != plan.profile_id => {
                        SpawnCommitOutcome::Stale
                    }
                    Some(entry) if entry.runtime.is_some() => {
                        inner
                            .store
                            .set_active_session(&plan.profile_id, Some(&plan.session_id))?;
                        inner.publish_session_updated_for(&plan.session_id);
                        inner.publish_workspace_updated();
                        SpawnCommitOutcome::AlreadyRunning
                    }
                    Some(entry) if entry.generation != plan.expected_generation => {
                        SpawnCommitOutcome::Stale
                    }
                    Some(_) => {
                        if let Some(entry) = inner.sessions.get_mut(&plan.session_id) {
                            entry.generation = plan.next_generation;
                            entry.runtime = Some(runtime.clone());
                            entry.session.status = StoredSessionStatus::Running;
                        }
                        inner
                            .store
                            .set_session_status(&plan.session_id, StoredSessionStatus::Running)?;
                        inner
                            .store
                            .set_active_session(&plan.profile_id, Some(&plan.session_id))?;
                        inner.publish_session_updated_for(&plan.session_id);
                        inner.publish_workspace_updated();
                        SpawnCommitOutcome::Committed
                    }
                }
            }
        };

        if !matches!(
            outcome,
            SpawnCommitOutcome::Committed | SpawnCommitOutcome::AlreadyRunning
        ) {
            kill_runtime_handle(Some(runtime));
        }

        Ok(outcome)
    }
}

impl StateInner {
    pub(super) fn publish_session_updated_for(&mut self, session_id: &str) {
        if let Some(entry) = self.sessions.get(session_id) {
            self.broadcast_event(RuntimeEvent::SessionUpdated(RuntimeSessionUpdatedEvent {
                session_id: session_id.to_string(),
                status: RuntimeSessionStatus::from(entry.session.status.clone()),
                seq: entry.session.seq,
                persist_history: entry.session.persist_history,
                ts: now_millis(),
            }));
        }
    }

    pub(super) fn publish_workspace_updated(&mut self) {
        if let Ok(workspace) = self.store.load_workspace() {
            self.broadcast_event(RuntimeEvent::WorkspaceUpdated(
                RuntimeWorkspaceUpdatedEvent {
                    active_profile_id: Some(workspace.active_profile_id),
                    active_session_id: workspace.active_session_id,
                    profile_count: workspace.profiles.len() as u64,
                    session_count: workspace.sessions.len() as u64,
                    ts: now_millis(),
                },
            ));
        }
    }

    pub(super) fn broadcast_daemon_health(&mut self) {
        let running_sessions = self
            .sessions
            .values()
            .filter(|entry| entry.runtime.is_some())
            .count() as u64;
        self.broadcast_event(RuntimeEvent::DaemonHealth(RuntimeDaemonHealthEvent {
            connected_clients: (self.protocol_clients.len() + self.subscribers.len()) as u64,
            session_count: self.sessions.len() as u64,
            running_sessions,
            ts: now_millis(),
        }));
    }

    pub(super) fn close_profile_runtimes(&mut self, profile_id: &str) {
        let target_ids: Vec<String> = self
            .sessions
            .iter()
            .filter_map(|(session_id, entry)| {
                if entry.session.profile_id == profile_id {
                    Some(session_id.clone())
                } else {
                    None
                }
            })
            .collect();

        for session_id in target_ids {
            if let Some(mut entry) = self.sessions.remove(&session_id)
                && let Some(runtime) = entry.runtime.take()
            {
                if let Ok(mut runtime) = runtime.lock() {
                    runtime.kill();
                }
            }
        }
    }

    pub(super) fn broadcast_event(&mut self, event: RuntimeEvent) {
        self.subscribers
            .retain(|_, tx| match tx.try_send(event.clone()) {
                Ok(_) => true,
                Err(std_mpsc::TrySendError::Full(_)) => false,
                Err(std_mpsc::TrySendError::Disconnected(_)) => false,
            });
        self.protocol_clients.broadcast_event(&event, &self.metrics);
    }
}
