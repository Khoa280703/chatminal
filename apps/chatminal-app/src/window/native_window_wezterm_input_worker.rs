use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::time::Duration;

use chatminal_protocol::{Request, Response};

use crate::ipc::ChatminalClient;

const INPUT_WORKER_QUEUE_CAPACITY: usize = 2048;
const INPUT_WORKER_RESULT_CAPACITY: usize = 4096;
const INPUT_WORKER_TIMEOUT_MS: u64 = 250;
const INPUT_WORKER_RETRY_LIMIT: usize = 1;
const INPUT_WORKER_CONNECT_RETRY_MS: u64 = 50;

struct QueuedInput {
    session_id: String,
    data: String,
}

pub(in crate::window) struct InputWorkerResult {
    pub session_id: String,
    pub bytes: usize,
    pub error: Option<String>,
}

pub(in crate::window) struct TerminalInputWorker {
    tx: SyncSender<QueuedInput>,
    rx: Receiver<InputWorkerResult>,
}

impl TerminalInputWorker {
    pub(in crate::window) fn spawn(endpoint: &str) -> Self {
        let (tx, work_rx) = mpsc::sync_channel::<QueuedInput>(INPUT_WORKER_QUEUE_CAPACITY);
        let (result_tx, rx) = mpsc::sync_channel::<InputWorkerResult>(INPUT_WORKER_RESULT_CAPACITY);
        let endpoint = endpoint.to_string();

        std::thread::spawn(move || {
            worker_loop(endpoint, work_rx, result_tx);
        });

        Self { tx, rx }
    }

    pub(in crate::window) fn enqueue(
        &self,
        session_id: String,
        data: String,
    ) -> Result<(), String> {
        let payload = QueuedInput { session_id, data };
        match self.tx.try_send(payload) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err("input queue is full".to_string()),
            Err(TrySendError::Disconnected(_)) => Err("input worker disconnected".to_string()),
        }
    }

    pub(in crate::window) fn try_recv(&self) -> Option<InputWorkerResult> {
        match self.rx.try_recv() {
            Ok(value) => Some(value),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => None,
        }
    }
}

fn worker_loop(
    endpoint: String,
    work_rx: Receiver<QueuedInput>,
    result_tx: SyncSender<InputWorkerResult>,
) {
    let mut client = ChatminalClient::connect(&endpoint).ok();
    while let Ok(job) = work_rx.recv() {
        debug_input_worker_log(&format!(
            "worker recv session_id={} len={} text={:?}",
            job.session_id,
            job.data.len(),
            job.data
        ));
        let mut attempts = 0usize;
        let result = loop {
            if client.is_none() {
                client = ChatminalClient::connect(&endpoint).ok();
                if client.is_none() {
                    attempts = attempts.saturating_add(1);
                    if attempts > INPUT_WORKER_RETRY_LIMIT {
                        break InputWorkerResult {
                            session_id: job.session_id.clone(),
                            bytes: job.data.len(),
                            error: Some(format!("connect daemon failed: '{endpoint}'")),
                        };
                    }
                    std::thread::sleep(Duration::from_millis(INPUT_WORKER_CONNECT_RETRY_MS));
                    continue;
                }
            }

            let Some(active_client) = client.as_ref() else {
                continue;
            };
            debug_input_worker_log(&format!(
                "worker request session_input_write session_id={} attempt={} len={}",
                job.session_id,
                attempts + 1,
                job.data.len()
            ));
            match active_client.request(
                Request::SessionInputWrite {
                    session_id: job.session_id.clone(),
                    data: job.data.clone(),
                },
                Duration::from_millis(INPUT_WORKER_TIMEOUT_MS),
            ) {
                Ok(Response::Empty) => {
                    debug_input_worker_log(&format!(
                        "worker request ok session_id={} len={}",
                        job.session_id,
                        job.data.len()
                    ));
                    break InputWorkerResult {
                        session_id: job.session_id.clone(),
                        bytes: job.data.len(),
                        error: None,
                    };
                }
                Ok(other) => {
                    debug_input_worker_log(&format!(
                        "worker request unexpected response session_id={}: {:?}",
                        job.session_id, other
                    ));
                    client = None;
                    attempts = attempts.saturating_add(1);
                    if attempts > INPUT_WORKER_RETRY_LIMIT {
                        break InputWorkerResult {
                            session_id: job.session_id.clone(),
                            bytes: job.data.len(),
                            error: Some(format!(
                                "unexpected response for session_input_write: {:?}",
                                other
                            )),
                        };
                    }
                }
                Err(err) => {
                    debug_input_worker_log(&format!(
                        "worker request error session_id={}: {err}",
                        job.session_id
                    ));
                    client = None;
                    attempts = attempts.saturating_add(1);
                    if attempts > INPUT_WORKER_RETRY_LIMIT {
                        break InputWorkerResult {
                            session_id: job.session_id.clone(),
                            bytes: job.data.len(),
                            error: Some(err),
                        };
                    }
                }
            }
        };
        if result_tx.send(result).is_err() {
            return;
        }
    }
}

fn debug_input_worker_enabled() -> bool {
    std::env::var("CHATMINAL_DEBUG_NATIVE_WINDOW")
        .ok()
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn debug_input_worker_log(message: &str) {
    if debug_input_worker_enabled() {
        eprintln!("chatminal-app input-worker-debug: {message}");
    }
}
