use chatminal_protocol::{Event, SessionStatus};

pub const INPUT_WRITE_BATCH_MAX_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEventAction {
    Output(String),
    Error(String),
    ExitRequested,
    Ignore,
}

#[derive(Debug, Clone)]
pub struct ChatminalIpcMuxDomain {
    session_id: String,
    last_seq: u64,
    last_session_update_ts: u64,
    pending_input_bytes: Vec<u8>,
    pending_input_batch: String,
}

impl ChatminalIpcMuxDomain {
    pub fn new(session_id: String, snapshot_seq: u64) -> Self {
        Self {
            session_id,
            last_seq: snapshot_seq,
            last_session_update_ts: 0,
            pending_input_bytes: Vec::new(),
            pending_input_batch: String::new(),
        }
    }

    pub fn queue_input_payload(&mut self, payload: &[u8]) {
        let chunks = decode_input_payload_chunks(&mut self.pending_input_bytes, payload);
        for chunk in chunks {
            self.pending_input_batch.push_str(&chunk);
        }
    }

    pub fn should_flush_input_batch(&self) -> bool {
        self.pending_input_batch.len() >= INPUT_WRITE_BATCH_MAX_BYTES
    }

    pub fn take_input_batch(&mut self) -> Option<String> {
        if self.pending_input_batch.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut self.pending_input_batch))
    }

    pub fn consume_event(&mut self, event: Event) -> DomainEventAction {
        match event {
            Event::PtyOutput(value) => {
                if value.session_id != self.session_id || value.seq <= self.last_seq {
                    return DomainEventAction::Ignore;
                }
                self.last_seq = value.seq;
                DomainEventAction::Output(value.chunk)
            }
            Event::PtyExited(value) => {
                if value.session_id == self.session_id {
                    DomainEventAction::ExitRequested
                } else {
                    DomainEventAction::Ignore
                }
            }
            Event::PtyError(value) => {
                if value.session_id == self.session_id {
                    DomainEventAction::Error(value.message)
                } else {
                    DomainEventAction::Ignore
                }
            }
            Event::SessionUpdated(value) => {
                if value.session_id != self.session_id {
                    return DomainEventAction::Ignore;
                }
                if value.seq < self.last_seq {
                    return DomainEventAction::Ignore;
                }
                if value.ts < self.last_session_update_ts {
                    return DomainEventAction::Ignore;
                }
                self.last_session_update_ts = value.ts;
                self.last_seq = self.last_seq.max(value.seq);
                if value.status == SessionStatus::Disconnected {
                    return DomainEventAction::ExitRequested;
                }
                DomainEventAction::Ignore
            }
            Event::WorkspaceUpdated(_) | Event::DaemonHealth(_) => DomainEventAction::Ignore,
        }
    }
}

pub fn clamp_preview_lines(value: usize) -> usize {
    value.clamp(50, 20_000)
}

pub fn decode_input_payload_chunks(pending: &mut Vec<u8>, payload: &[u8]) -> Vec<String> {
    if !payload.is_empty() {
        pending.extend_from_slice(payload);
    }

    let mut chunks = Vec::<String>::new();
    loop {
        if pending.is_empty() {
            break;
        }
        match std::str::from_utf8(pending) {
            Ok(text) => {
                if !text.is_empty() {
                    chunks.push(text.to_string());
                }
                pending.clear();
                break;
            }
            Err(err) => {
                let valid_up_to = err.valid_up_to();
                if valid_up_to > 0 {
                    let valid = String::from_utf8_lossy(&pending[..valid_up_to]).to_string();
                    if !valid.is_empty() {
                        chunks.push(valid);
                    }
                    pending.drain(..valid_up_to);
                    continue;
                }

                match err.error_len() {
                    None => break,
                    Some(invalid_len) => {
                        let lossy = String::from_utf8_lossy(&pending[..invalid_len]).to_string();
                        if !lossy.is_empty() {
                            chunks.push(lossy);
                        }
                        pending.drain(..invalid_len);
                    }
                }
            }
        }
    }
    chunks
}

#[cfg(test)]
#[path = "chatminal_ipc_mux_domain_tests.rs"]
mod chatminal_ipc_mux_domain_tests;
