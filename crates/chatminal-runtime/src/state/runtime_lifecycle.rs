use std::sync::mpsc as std_mpsc;

use chatminal_store::{StoredSession, StoredSessionStatus};

use super::{
    DaemonState, SessionEntry, SessionSpawnPlan, StateInner, kill_runtime_handle, now_millis,
    runtime_bridge::RuntimeHandle, snapshot_requires_run_boundary,
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

#[derive(Clone, Copy)]
pub(super) struct DisconnectOptions {
    pub reset_history: bool,
    pub bump_generation: bool,
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
        let runtime = self.spawn_runtime_handle(
            &plan.session_id,
            plan.next_generation,
            &plan.shell,
            &plan.cwd,
            plan.cols,
            plan.rows,
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
                        inner.set_active_session_and_publish(&plan.profile_id, &plan.session_id)?;
                        SpawnCommitOutcome::AlreadyRunning
                    }
                    Some(entry) if entry.generation != plan.expected_generation => {
                        SpawnCommitOutcome::Stale
                    }
                    Some(_) => {
                        let prepend_run_boundary_on_next_output = inner
                            .store
                            .session_snapshot(&plan.session_id, 1)
                            .map(|snapshot| snapshot_requires_run_boundary(&snapshot))?;
                        if let Some(entry) = inner.sessions.get_mut(&plan.session_id) {
                            entry.generation = plan.next_generation;
                            entry.runtime = Some(runtime.clone());
                            entry.session.status = StoredSessionStatus::Running;
                            entry.prepend_run_boundary_on_next_output =
                                prepend_run_boundary_on_next_output;
                        }
                        inner
                            .store
                            .set_session_status(&plan.session_id, StoredSessionStatus::Running)?;
                        inner.set_active_session_and_publish(&plan.profile_id, &plan.session_id)?;
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
    pub(super) fn insert_running_session_and_publish(
        &mut self,
        mut session: StoredSession,
        runtime: RuntimeHandle,
    ) -> Result<(), String> {
        session.status = StoredSessionStatus::Running;
        self.store
            .set_session_status(&session.session_id, StoredSessionStatus::Running)?;
        self.sessions.insert(
            session.session_id.clone(),
            SessionEntry {
                session: session.clone(),
                runtime: Some(runtime),
                live_output: String::new(),
                generation: 0,
                prepend_run_boundary_on_next_output: false,
            },
        );
        self.set_active_session_and_publish(&session.profile_id, &session.session_id)
    }

    pub(super) fn remove_session_and_publish_workspace(
        &mut self,
        session_id: &str,
    ) -> Result<Option<RuntimeHandle>, String> {
        self.store.delete_session(session_id)?;
        let runtime = self
            .sessions
            .remove(session_id)
            .and_then(|mut entry| entry.runtime.take());
        self.publish_workspace_updated();
        Ok(runtime)
    }

    pub(super) fn clear_session_history_and_publish(
        &mut self,
        session_id: &str,
    ) -> Result<Option<RuntimeHandle>, String> {
        self.store.clear_session_history(session_id)?;
        let runtime = self.sessions.get_mut(session_id).and_then(|entry| {
            disconnect_session_entry(
                entry,
                DisconnectOptions {
                    reset_history: true,
                    bump_generation: true,
                },
            )
        });
        if self.sessions.contains_key(session_id) {
            self.store
                .set_session_status(session_id, StoredSessionStatus::Disconnected)?;
        }
        self.publish_session_and_workspace_updated(session_id);
        Ok(runtime)
    }

    pub(super) fn disconnect_all_sessions_and_publish(
        &mut self,
        options: DisconnectOptions,
    ) -> Vec<RuntimeHandle> {
        let mut updated_ids = Vec::new();
        let mut runtimes = Vec::new();
        for entry in self.sessions.values_mut() {
            if let Some(runtime) = disconnect_session_entry(entry, options) {
                runtimes.push(runtime);
            }
            updated_ids.push(entry.session.session_id.clone());
        }
        for session_id in &updated_ids {
            let _ = self
                .store
                .set_session_status(session_id, StoredSessionStatus::Disconnected);
        }
        for session_id in updated_ids {
            self.publish_session_updated_for(&session_id);
        }
        self.publish_workspace_updated();
        runtimes
    }

    pub(super) fn mark_session_exited(&mut self, session_id: &str, generation: u64) -> bool {
        let Some(entry) = self.sessions.get_mut(session_id) else {
            return false;
        };
        if entry.generation != generation {
            return false;
        }

        let _ = disconnect_session_entry(
            entry,
            DisconnectOptions {
                reset_history: false,
                bump_generation: false,
            },
        );
        let _ = self
            .store
            .set_session_status(session_id, StoredSessionStatus::Disconnected);
        true
    }

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

fn disconnect_session_entry(
    entry: &mut SessionEntry,
    options: DisconnectOptions,
) -> Option<RuntimeHandle> {
    if options.bump_generation {
        entry.generation = entry.generation.saturating_add(1);
    }
    if options.reset_history {
        entry.session.seq = 0;
        entry.live_output.clear();
    }
    entry.prepend_run_boundary_on_next_output = false;
    entry.session.status = StoredSessionStatus::Disconnected;
    entry.runtime.take()
}
