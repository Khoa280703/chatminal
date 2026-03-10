use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct RuntimeMetrics {
    requests_total: Arc<AtomicU64>,
    request_errors_total: Arc<AtomicU64>,
    session_events_total: Arc<AtomicU64>,
    broadcast_frames_total: Arc<AtomicU64>,
    dropped_clients_full_total: Arc<AtomicU64>,
    dropped_clients_disconnected_total: Arc<AtomicU64>,
    input_queue_full_total: Arc<AtomicU64>,
    input_retry_total: Arc<AtomicU64>,
    input_drop_total: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeMetricsSnapshot {
    pub requests_total: u64,
    pub request_errors_total: u64,
    pub session_events_total: u64,
    pub broadcast_frames_total: u64,
    pub dropped_clients_full_total: u64,
    pub dropped_clients_disconnected_total: u64,
    pub input_queue_full_total: u64,
    pub input_retry_total: u64,
    pub input_drop_total: u64,
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self {
            requests_total: Arc::new(AtomicU64::new(0)),
            request_errors_total: Arc::new(AtomicU64::new(0)),
            session_events_total: Arc::new(AtomicU64::new(0)),
            broadcast_frames_total: Arc::new(AtomicU64::new(0)),
            dropped_clients_full_total: Arc::new(AtomicU64::new(0)),
            dropped_clients_disconnected_total: Arc::new(AtomicU64::new(0)),
            input_queue_full_total: Arc::new(AtomicU64::new(0)),
            input_retry_total: Arc::new(AtomicU64::new(0)),
            input_drop_total: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn inc_requests_total(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_request_errors_total(&self) {
        self.request_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_session_events_total(&self) {
        self.session_events_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_broadcast_frames_total(&self) {
        self.broadcast_frames_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_dropped_clients_full_total(&self) {
        self.dropped_clients_full_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_dropped_clients_disconnected_total(&self) {
        self.dropped_clients_disconnected_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_input_queue_full_total(&self, value: u64) {
        if value > 0 {
            self.input_queue_full_total
                .fetch_add(value, Ordering::Relaxed);
        }
    }

    pub fn add_input_retry_total(&self, value: u64) {
        if value > 0 {
            self.input_retry_total.fetch_add(value, Ordering::Relaxed);
        }
    }

    pub fn add_input_drop_total(&self, value: u64) {
        if value > 0 {
            self.input_drop_total.fetch_add(value, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> RuntimeMetricsSnapshot {
        RuntimeMetricsSnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            request_errors_total: self.request_errors_total.load(Ordering::Relaxed),
            session_events_total: self.session_events_total.load(Ordering::Relaxed),
            broadcast_frames_total: self.broadcast_frames_total.load(Ordering::Relaxed),
            dropped_clients_full_total: self.dropped_clients_full_total.load(Ordering::Relaxed),
            dropped_clients_disconnected_total: self
                .dropped_clients_disconnected_total
                .load(Ordering::Relaxed),
            input_queue_full_total: self.input_queue_full_total.load(Ordering::Relaxed),
            input_retry_total: self.input_retry_total.load(Ordering::Relaxed),
            input_drop_total: self.input_drop_total.load(Ordering::Relaxed),
        }
    }
}
