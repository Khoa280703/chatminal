use chatminal_protocol::{Event, ServerFrame, SessionStatus};

use super::{DaemonState, trim_live_output};
use crate::session::SessionEvent;

impl DaemonState {
    pub(super) fn apply_session_event(&self, event: SessionEvent) {
        self.metrics.inc_session_events_total();
        let mut inner = match self.inner.lock() {
            Ok(value) => value,
            Err(_) => return,
        };

        match event {
            SessionEvent::Output {
                session_id,
                generation,
                chunk,
                ts,
            } => {
                let mut frame = None;
                let mut seq_after = None;
                let mut persist_history = false;
                if let Some(entry) = inner.sessions.get_mut(&session_id) {
                    if entry.generation != generation {
                        return;
                    }
                    entry.session.seq += 1;
                    entry.session.status = SessionStatus::Running;
                    seq_after = Some(entry.session.seq);
                    persist_history = entry.session.persist_history;
                    if !persist_history {
                        entry.live_output.push_str(&chunk);
                        trim_live_output(&mut entry.live_output, 1024 * 1024);
                    }

                    frame = Some(ServerFrame::event(Event::PtyOutput(
                        chatminal_protocol::PtyOutputEvent {
                            session_id: session_id.clone(),
                            chunk: chunk.clone(),
                            seq: entry.session.seq,
                            ts,
                        },
                    )));
                }

                if let Some(seq) = seq_after {
                    if let Err(err) = inner.store.update_session_seq(&session_id, seq) {
                        inner.broadcast(ServerFrame::event(Event::PtyError(
                            chatminal_protocol::PtyErrorEvent {
                                session_id: session_id.clone(),
                                message: format!("persist seq failed: {err}"),
                            },
                        )));
                    }
                    if let Err(err) = inner
                        .store
                        .set_session_status(&session_id, SessionStatus::Running)
                    {
                        inner.broadcast(ServerFrame::event(Event::PtyError(
                            chatminal_protocol::PtyErrorEvent {
                                session_id: session_id.clone(),
                                message: format!("persist status failed: {err}"),
                            },
                        )));
                    }
                    if persist_history {
                        if let Err(err) =
                            inner
                                .store
                                .append_scrollback_chunk(&session_id, seq, &chunk, ts)
                        {
                            inner.broadcast(ServerFrame::event(Event::PtyError(
                                chatminal_protocol::PtyErrorEvent {
                                    session_id: session_id.clone(),
                                    message: format!("persist chunk failed: {err}"),
                                },
                            )));
                        } else if let Err(err) = inner.store.enforce_session_scrollback_line_limit(
                            &session_id,
                            inner.config.max_scrollback_lines_per_session,
                        ) {
                            inner.broadcast(ServerFrame::event(Event::PtyError(
                                chatminal_protocol::PtyErrorEvent {
                                    session_id: session_id.clone(),
                                    message: format!("apply retention failed: {err}"),
                                },
                            )));
                        }
                    }
                }

                if let Some(frame) = frame {
                    inner.broadcast(frame);
                }
            }
            SessionEvent::Exited {
                session_id,
                generation,
                exit_code,
                reason,
            } => {
                let mut updated = false;
                if let Some(entry) = inner.sessions.get_mut(&session_id) {
                    if entry.generation != generation {
                        return;
                    }
                    entry.runtime = None;
                    entry.session.status = SessionStatus::Disconnected;
                    let _ = inner
                        .store
                        .set_session_status(&session_id, SessionStatus::Disconnected);
                    updated = true;
                }

                inner.broadcast(ServerFrame::event(Event::PtyExited(
                    chatminal_protocol::PtyExitedEvent {
                        session_id: session_id.clone(),
                        exit_code,
                        reason,
                    },
                )));
                if updated {
                    inner.publish_session_updated_for(&session_id);
                    inner.publish_workspace_updated();
                }
            }
            SessionEvent::Error {
                session_id,
                generation,
                message,
            } => {
                if let Some(entry) = inner.sessions.get(&session_id)
                    && entry.generation != generation
                {
                    return;
                }
                inner.broadcast(ServerFrame::event(Event::PtyError(
                    chatminal_protocol::PtyErrorEvent {
                        session_id,
                        message,
                    },
                )));
            }
        }
    }
}
