use std::sync::mpsc as std_mpsc;

use chatminal_protocol::{Event, ServerFrame, SessionStatus};

use super::{StateInner, now_millis};
use crate::session::{SessionEvent, SessionRuntime};

impl StateInner {
    pub(super) fn ensure_active_session_runtime(
        &mut self,
        events: std_mpsc::SyncSender<SessionEvent>,
    ) -> Result<(), String> {
        let (_, active_profile_id, _, active_session_id) = self.store.load_workspace()?;
        let Some(session_id) = active_session_id else {
            return Ok(());
        };

        let mut started = false;
        if let Some(entry) = self.sessions.get_mut(&session_id)
            && entry.session.profile_id == active_profile_id
            && entry.runtime.is_none()
        {
            entry.generation = entry.generation.saturating_add(1);
            let runtime = SessionRuntime::spawn(
                entry.session.session_id.clone(),
                entry.generation,
                entry.session.shell.clone(),
                entry.session.cwd.clone(),
                self.config.default_cols,
                self.config.default_rows,
                events,
            )?;
            entry.runtime = Some(runtime);
            entry.session.status = SessionStatus::Running;
            self.store
                .set_session_status(&entry.session.session_id, SessionStatus::Running)?;
            started = true;
        }

        if started {
            self.publish_session_updated_for(&session_id);
            self.publish_workspace_updated();
        }
        Ok(())
    }

    pub(super) fn publish_session_updated_for(&mut self, session_id: &str) {
        if let Some(entry) = self.sessions.get(session_id) {
            self.broadcast(ServerFrame::event(Event::SessionUpdated(
                chatminal_protocol::SessionUpdatedEvent {
                    session_id: session_id.to_string(),
                    status: entry.session.status.clone(),
                    seq: entry.session.seq,
                    persist_history: entry.session.persist_history,
                    ts: now_millis(),
                },
            )));
        }
    }

    pub(super) fn publish_workspace_updated(&mut self) {
        if let Ok((profiles, active_profile_id, sessions, active_session_id)) =
            self.store.load_workspace()
        {
            self.broadcast(ServerFrame::event(Event::WorkspaceUpdated(
                chatminal_protocol::WorkspaceUpdatedEvent {
                    active_profile_id: Some(active_profile_id),
                    active_session_id,
                    profile_count: profiles.len() as u64,
                    session_count: sessions.len() as u64,
                    ts: now_millis(),
                },
            )));
        }
    }

    pub(super) fn broadcast_daemon_health(&mut self) {
        let running_sessions = self
            .sessions
            .values()
            .filter(|entry| entry.runtime.is_some())
            .count() as u64;
        self.broadcast(ServerFrame::event(Event::DaemonHealth(
            chatminal_protocol::DaemonHealthEvent {
                connected_clients: self.clients.len() as u64,
                session_count: self.sessions.len() as u64,
                running_sessions,
                ts: now_millis(),
            },
        )));
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
                && let Some(mut runtime) = entry.runtime.take()
            {
                runtime.kill();
            }
        }
    }

    pub(super) fn broadcast(&mut self, frame: ServerFrame) {
        let metrics = self.metrics.clone();
        metrics.inc_broadcast_frames_total();
        self.clients
            .retain(|_, tx| match tx.try_send(frame.clone()) {
                Ok(_) => true,
                Err(std_mpsc::TrySendError::Full(_)) => {
                    metrics.inc_dropped_clients_full_total();
                    false
                }
                Err(std_mpsc::TrySendError::Disconnected(_)) => {
                    metrics.inc_dropped_clients_disconnected_total();
                    false
                }
            });
    }
}
