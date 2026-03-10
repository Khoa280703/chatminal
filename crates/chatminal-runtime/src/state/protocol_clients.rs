use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;

use chatminal_protocol::ServerFrame;

use crate::api::RuntimeEvent;
use crate::metrics::RuntimeMetrics;

pub(super) type ProtocolClientSender = std_mpsc::SyncSender<ServerFrame>;

pub(super) struct ProtocolClients {
    clients: HashMap<u64, ProtocolClientSender>,
}

impl ProtocolClients {
    pub(super) fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.clients.len()
    }

    pub(super) fn register(&mut self, client_id: u64, tx: ProtocolClientSender) {
        self.clients.insert(client_id, tx);
    }

    pub(super) fn unregister(&mut self, client_id: u64) {
        self.clients.remove(&client_id);
    }

    pub(super) fn broadcast_event(&mut self, event: &RuntimeEvent, metrics: &RuntimeMetrics) {
        self.broadcast_frame(ServerFrame::event(event.clone().into()), metrics);
    }

    fn broadcast_frame(&mut self, frame: ServerFrame, metrics: &RuntimeMetrics) {
        metrics.inc_broadcast_frames_total();
        self.clients
            .retain(|_, tx| match tx.try_send(frame.clone()) {
                Ok(_) => true,
                Err(std_mpsc::TrySendError::Full(_)) => {
                    log::warn!("dropping daemon broadcast client because outbound queue is full");
                    metrics.inc_dropped_clients_full_total();
                    false
                }
                Err(std_mpsc::TrySendError::Disconnected(_)) => {
                    log::warn!(
                        "dropping daemon broadcast client because outbound queue is disconnected"
                    );
                    metrics.inc_dropped_clients_disconnected_total();
                    false
                }
            });
    }
}
