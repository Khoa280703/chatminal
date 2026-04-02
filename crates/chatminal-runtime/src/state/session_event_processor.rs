use std::sync::Arc;

use chatminal_store::StoredSessionStatus;

use super::{
    RuntimeState, canonical_scrollback::materialize_output_chunk,
    logicalize_prepended_run_boundary, persist_worker::PersistJob, prepend_run_boundary,
    startup_recipe::append_recent_output_tail, strip_duplicate_restored_prompt_prefix,
    strip_volatile_terminal_control_sequences, strip_zsh_prompt_spacer_artifact, trim_live_output,
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
                let mut was_already_running = false;
                let mut output_chunk = chunk;
                let mut raw_replay_chunk = String::new();
                let mut synthetic_run_boundary_prepended = false;
                let mut logicalized: Option<String> = None;
                if let Some(entry) = inner.sessions.get_mut(session_id.as_ref()) {
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
                                return;
                            }
                            entry.prepend_run_boundary_on_next_output = false;
                            entry.restored_trailing_fragment = None;
                            output_chunk = prepend_run_boundary(&stripped_chunk);
                            synthetic_run_boundary_prepended = true;
                        } else {
                            entry.prepend_run_boundary_on_next_output = false;
                            entry.restored_trailing_fragment = None;
                            output_chunk = prepend_run_boundary(&output_chunk);
                            synthetic_run_boundary_prepended = true;
                        }
                    } else if !output_chunk.is_empty() {
                        entry.restored_trailing_fragment = None;
                    }
                    raw_replay_chunk = output_chunk.clone();
                    entry.session.seq += 1;
                    was_already_running = entry.session.status == StoredSessionStatus::Running;
                    entry.session.status = StoredSessionStatus::Running;
                    seq_after = Some(entry.session.seq);
                    persist_history = entry.session.persist_history;
                    logicalized = if synthetic_run_boundary_prepended {
                        Some(logicalize_prepended_run_boundary(&output_chunk))
                    } else {
                        None
                    };
                    let tail_chunk = strip_volatile_terminal_control_sequences(
                        &strip_zsh_prompt_spacer_artifact(
                            logicalized.as_deref().unwrap_or(&output_chunk),
                        ),
                    );
                    append_recent_output_tail(&mut entry.recent_output_tail, &tail_chunk);
                    if !persist_history {
                        let buffered = logicalized.as_deref().unwrap_or(&output_chunk);
                        entry.live_output.push_str(buffered);
                        trim_live_output(&mut entry.live_output, 256 * 1024);
                    }

                    event = Some(RuntimeEvent::PtyOutput(RuntimePtyOutputEvent {
                        session_id: session_id.to_string(),
                        chunk: output_chunk,
                        seq: entry.session.seq,
                        ts,
                    }));
                }

                // Offload persist work to background thread (no SQLite under lock).
                if let Some(seq) = seq_after {
                    let _ = self.persist_tx.try_send(PersistJob::UpdateSeq {
                        session_id: Arc::clone(&session_id),
                        seq,
                    });
                    if !was_already_running {
                        let _ = self.persist_tx.try_send(PersistJob::MarkRunning {
                            session_id: Arc::clone(&session_id),
                        });
                    }
                    if persist_history {
                        let persisted_chunk =
                            logicalized.unwrap_or_else(|| raw_replay_chunk.clone());
                        let persisted_chunk = strip_volatile_terminal_control_sequences(
                            &strip_zsh_prompt_spacer_artifact(&persisted_chunk),
                        );
                        let current_fragment = inner
                            .sessions
                            .get(session_id.as_ref())
                            .map(|entry| {
                                (
                                    entry.canonical_open_fragment.clone(),
                                    entry.canonical_cursor_col,
                                    entry.canonical_pending_carriage_return,
                                )
                            })
                            .unwrap_or_default();
                        let materialized = materialize_output_chunk(
                            &current_fragment.0,
                            current_fragment.1,
                            current_fragment.2,
                            &persisted_chunk,
                        );

                        // Update in-memory canonical state.
                        if let Some(entry) = inner.sessions.get_mut(session_id.as_ref()) {
                            entry.canonical_open_fragment = materialized.open_fragment;
                            entry.canonical_cursor_col = materialized.cursor_col;
                            entry.canonical_pending_carriage_return =
                                materialized.pending_carriage_return;
                        }

                        let _ = self.persist_tx.try_send(PersistJob::OutputChunk {
                            session_id: Arc::clone(&session_id),
                            seq,
                            records: materialized.records,
                            replay_chunk: raw_replay_chunk,
                            ts,
                        });

                        if seq % 50 == 0 {
                            let max_lines = inner.config.max_scrollback_lines_per_session;
                            let _ = self.persist_tx.try_send(PersistJob::EnforceLimit {
                                session_id: Arc::clone(&session_id),
                                max_lines,
                            });
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
                    session_id: session_id.to_string(),
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
                if let Some(entry) = inner.sessions.get(session_id.as_ref())
                    && entry.generation != generation
                {
                    return;
                }
                inner.broadcast_event(RuntimeEvent::PtyError(RuntimePtyErrorEvent {
                    session_id: session_id.to_string(),
                    message,
                }));
            }
        }
    }
}
