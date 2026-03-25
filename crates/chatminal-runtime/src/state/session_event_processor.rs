use chatminal_store::StoredSessionStatus;

use super::{
    RuntimeState, prepend_run_boundary, strip_duplicate_restored_prompt_prefix,
    strip_zsh_prompt_spacer_artifact, trim_live_output,
};
use crate::api::{
    RuntimeEvent, RuntimePtyErrorEvent, RuntimePtyExitedEvent, RuntimePtyOutputEvent,
};
use crate::session::SessionEvent;

impl RuntimeState {
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
                let mut event = None;
                let mut seq_after = None;
                let mut persist_history = false;
                let mut output_chunk = chunk;
                if let Some(entry) = inner.sessions.get_mut(&session_id) {
                    if entry.generation != generation {
                        return;
                    }
                    if entry.prepend_run_boundary_on_next_output && !output_chunk.is_empty() {
                        let stripped_duplicate_prompt = entry
                            .restored_trailing_fragment
                            .as_deref()
                            .and_then(|fragment| {
                                strip_duplicate_restored_prompt_prefix(fragment, &output_chunk)
                            });
                        if let Some(stripped_chunk) = stripped_duplicate_prompt {
                            if stripped_chunk.is_empty() {
                                // Keep the restore-dedupe state armed until we see either
                                // real output or a prompt-prefixed chunk that carries new
                                // content. Some shells redraw the prompt multiple times
                                // immediately after startup/restore.
                                return;
                            }
                            entry.prepend_run_boundary_on_next_output = false;
                            entry.restored_trailing_fragment = None;
                            output_chunk = prepend_run_boundary(&stripped_chunk);
                        } else {
                            entry.prepend_run_boundary_on_next_output = false;
                            entry.restored_trailing_fragment = None;
                            output_chunk = prepend_run_boundary(&output_chunk);
                        }
                    } else if !output_chunk.is_empty() {
                        entry.restored_trailing_fragment = None;
                    }
                    entry.session.seq += 1;
                    entry.session.status = StoredSessionStatus::Running;
                    seq_after = Some(entry.session.seq);
                    persist_history = entry.session.persist_history;
                    if !persist_history {
                        entry.live_output.push_str(&output_chunk);
                        trim_live_output(&mut entry.live_output, 1024 * 1024);
                    }

                    event = Some(RuntimeEvent::PtyOutput(RuntimePtyOutputEvent {
                        session_id: session_id.clone(),
                        chunk: output_chunk.clone(),
                        seq: entry.session.seq,
                        ts,
                    }));
                }

                if let Some(seq) = seq_after {
                    if let Err(err) = inner.store.update_session_seq(&session_id, seq) {
                        inner.broadcast_event(RuntimeEvent::PtyError(RuntimePtyErrorEvent {
                            session_id: session_id.clone(),
                            message: format!("persist seq failed: {err}"),
                        }));
                    }
                    if let Err(err) = inner
                        .store
                        .set_session_status(&session_id, StoredSessionStatus::Running)
                    {
                        inner.broadcast_event(RuntimeEvent::PtyError(RuntimePtyErrorEvent {
                            session_id: session_id.clone(),
                            message: format!("persist status failed: {err}"),
                        }));
                    }
                    if persist_history {
                        let persisted_chunk = strip_zsh_prompt_spacer_artifact(&output_chunk);
                        if let Err(err) =
                            inner
                                .store
                                .append_scrollback_chunk(&session_id, seq, &persisted_chunk, ts)
                        {
                            inner.broadcast_event(RuntimeEvent::PtyError(RuntimePtyErrorEvent {
                                session_id: session_id.clone(),
                                message: format!("persist chunk failed: {err}"),
                            }));
                        } else if let Err(err) = inner.store.enforce_session_scrollback_line_limit(
                            &session_id,
                            inner.config.max_scrollback_lines_per_session,
                        ) {
                            inner.broadcast_event(RuntimeEvent::PtyError(RuntimePtyErrorEvent {
                                session_id: session_id.clone(),
                                message: format!("apply retention failed: {err}"),
                            }));
                        }
                    }
                }

                if let Some(event) = event {
                    inner.broadcast_event(event);
                }
            }
            SessionEvent::Exited {
                session_id,
                generation,
                exit_code,
                reason,
            } => {
                let updated = inner.mark_session_exited(&session_id, generation);

                inner.broadcast_event(RuntimeEvent::PtyExited(RuntimePtyExitedEvent {
                    session_id: session_id.clone(),
                    exit_code,
                    reason,
                }));
                if updated {
                    inner.publish_session_and_workspace_updated(&session_id);
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
                inner.broadcast_event(RuntimeEvent::PtyError(RuntimePtyErrorEvent {
                    session_id,
                    message,
                }));
            }
        }
    }
}
