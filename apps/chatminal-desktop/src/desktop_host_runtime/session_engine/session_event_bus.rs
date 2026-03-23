use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{RuntimeId, TerminalInstanceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionRuntimeEvent {
    RuntimeAttached {
        session_id: String,
        runtime_id: RuntimeId,
    },
    RuntimeFocused {
        session_id: String,
        runtime_id: RuntimeId,
    },
    TerminalInstanceFocused {
        session_id: String,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
    },
    RuntimeClosed {
        session_id: String,
        runtime_id: RuntimeId,
    },
    TerminalInstanceOutput {
        session_id: String,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        chunk: String,
    },
    TerminalInstanceExited {
        session_id: String,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        exit_code: Option<i32>,
    },
    TerminalInstanceError {
        session_id: String,
        generation: u64,
        runtime_id: RuntimeId,
        terminal_instance_id: TerminalInstanceId,
        message: String,
    },
}

pub trait SessionEventBus: Send + Sync {
    fn publish(&self, event: SessionRuntimeEvent);
}

#[derive(Debug)]
struct SessionEventHubInner {
    subscribers: Mutex<HashMap<u64, std_mpsc::SyncSender<SessionRuntimeEvent>>>,
    next_subscriber_id: AtomicU64,
}

#[derive(Clone, Debug)]
pub struct SessionEventHub {
    inner: Arc<SessionEventHubInner>,
}

impl Default for SessionEventHub {
    fn default() -> Self {
        Self {
            inner: Arc::new(SessionEventHubInner {
                subscribers: Mutex::new(HashMap::new()),
                next_subscriber_id: AtomicU64::new(1),
            }),
        }
    }
}

impl SessionEventHub {
    pub fn subscribe(&self) -> SessionEventSubscription {
        let (tx, rx) = std_mpsc::sync_channel(1024);
        let subscriber_id = self
            .inner
            .next_subscriber_id
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .subscribers
            .lock()
            .unwrap()
            .insert(subscriber_id, tx);
        SessionEventSubscription {
            hub: self.clone(),
            subscriber_id,
            rx,
        }
    }

    fn unsubscribe(&self, subscriber_id: u64) {
        self.inner
            .subscribers
            .lock()
            .unwrap()
            .remove(&subscriber_id);
    }
}

impl SessionEventBus for SessionEventHub {
    fn publish(&self, event: SessionRuntimeEvent) {
        self.inner
            .subscribers
            .lock()
            .unwrap()
            .retain(|_, tx| tx.try_send(event.clone()).is_ok());
    }
}

#[derive(Debug)]
pub struct SessionEventSubscription {
    hub: SessionEventHub,
    subscriber_id: u64,
    rx: std_mpsc::Receiver<SessionRuntimeEvent>,
}

impl SessionEventSubscription {
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<SessionRuntimeEvent>, String> {
        match self.rx.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(std_mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                Err("session event channel disconnected".to_string())
            }
        }
    }
}

impl Drop for SessionEventSubscription {
    fn drop(&mut self) {
        self.hub.unsubscribe(self.subscriber_id);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{RuntimeId, TerminalInstanceId};
    use super::{SessionEventBus, SessionEventHub, SessionRuntimeEvent};

    #[test]
    fn event_hub_broadcasts_runtime_events_to_subscribers() {
        let hub = SessionEventHub::default();
        let sub = hub.subscribe();
        hub.publish(SessionRuntimeEvent::TerminalInstanceOutput {
            session_id: "session-a".into(),
            generation: 3,
            runtime_id: RuntimeId::new(7),
            terminal_instance_id: TerminalInstanceId::new(9),
            chunk: "hello".into(),
        });

        assert_eq!(
            sub.recv_timeout(Duration::from_secs(1)).unwrap(),
            Some(SessionRuntimeEvent::TerminalInstanceOutput {
                session_id: "session-a".into(),
                generation: 3,
                runtime_id: RuntimeId::new(7),
                terminal_instance_id: TerminalInstanceId::new(9),
                chunk: "hello".into(),
            })
        );
    }
}
