use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

use chatminal_protocol::{ClientFrame, Event, Request, Response, ServerBody, ServerFrame};

use super::client_runtime::{Incoming, MAX_FRAME_QUEUE, read_frames_loop, response_from_frame};
use super::transport::{ReadWriteStream, connect_local_stream};

#[cfg(test)]
pub(super) use super::client_runtime::MAX_BACKLOG_FRAMES;
pub(super) use super::client_runtime::push_backlog_limited;

pub struct ChatminalClient {
    writer: Mutex<Box<dyn ReadWriteStream>>,
    frames_rx: Mutex<Receiver<Incoming>>,
    backlog: Mutex<VecDeque<ServerFrame>>,
    next_id: AtomicU64,
    broken: AtomicBool,
}

const READ_NEXT_INCOMING_LOCK_SLICE_MS: u64 = 20;

impl ChatminalClient {
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        ipc_debug_log(&format!("connect endpoint={endpoint}"));
        let stream = connect_local_stream(endpoint)?;
        Self::from_stream(stream)
    }

    fn from_stream(stream: Box<dyn ReadWriteStream>) -> Result<Self, String> {
        let reader = stream
            .try_clone_boxed()
            .map_err(|err| format!("clone stream failed: {err}"))?;
        let (frames_tx, frames_rx) = mpsc::sync_channel::<Incoming>(MAX_FRAME_QUEUE);
        std::thread::spawn(move || {
            read_frames_loop(reader, frames_tx);
        });

        Ok(Self {
            writer: Mutex::new(stream),
            frames_rx: Mutex::new(frames_rx),
            backlog: Mutex::new(VecDeque::new()),
            next_id: AtomicU64::new(1),
            broken: AtomicBool::new(false),
        })
    }

    pub fn request(&self, request: Request, timeout: Duration) -> Result<Response, String> {
        let deadline = Instant::now() + timeout;
        let id = format!("req-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let frame = ClientFrame {
            id: id.clone(),
            request,
        };
        let mut encoded =
            serde_json::to_string(&frame).map_err(|err| format!("encode request failed: {err}"))?;
        encoded.push('\n');
        self.write_request_direct(encoded.as_bytes(), &id, deadline)?;

        if let Some(frame) = self.take_matching_response_from_backlog(&id)? {
            return response_from_frame(frame, &id);
        }

        loop {
            if Instant::now() >= deadline {
                return Err(format!("request timeout for id '{id}'"));
            }

            if let Some(frame) = self.take_matching_response_from_backlog(&id)? {
                return response_from_frame(frame, &id);
            }

            let wait = deadline.saturating_duration_since(Instant::now());
            let chunk = wait.min(Duration::from_millis(250));
            let incoming = match self.read_next_incoming(chunk) {
                Ok(value) => value,
                Err(err) => {
                    if let Some(frame) = self.take_matching_response_from_backlog(&id)? {
                        return response_from_frame(frame, &id);
                    }
                    if err == "daemon stream disconnected"
                        && let Some(frame) =
                            self.wait_for_matching_response_until_deadline(&id, deadline)?
                    {
                        return response_from_frame(frame, &id);
                    }
                    return Err(err);
                }
            };
            let Some(incoming) = incoming else {
                continue;
            };
            let frame = match incoming {
                Incoming::Frame(value) => value,
                Incoming::ProtocolError(message) => return Err(message),
            };
            if frame.id.as_deref() == Some(id.as_str()) {
                return response_from_frame(frame, &id);
            }

            let mut backlog = self
                .backlog
                .lock()
                .map_err(|_| "backlog lock poisoned".to_string())?;
            push_backlog_limited(&mut backlog, frame);
        }
    }

    pub fn recv_event(&self, timeout: Duration) -> Result<Option<Event>, String> {
        if let Some(event) = self.take_event_from_backlog()? {
            return Ok(Some(event));
        }

        let incoming = match self.read_next_incoming(timeout) {
            Ok(value) => value,
            Err(err) => {
                if let Some(event) = self.take_event_from_backlog()? {
                    return Ok(Some(event));
                }
                return Err(err);
            }
        };
        let Some(incoming) = incoming else {
            return Ok(None);
        };
        let frame = match incoming {
            Incoming::Frame(frame) => frame,
            Incoming::ProtocolError(message) => return Err(message),
        };

        match frame.body {
            ServerBody::Event { event } => Ok(Some(event)),
            ServerBody::Response { .. } => {
                let mut backlog = self
                    .backlog
                    .lock()
                    .map_err(|_| "backlog lock poisoned".to_string())?;
                push_backlog_limited(&mut backlog, frame);
                Ok(None)
            }
        }
    }

    fn read_next_incoming(&self, timeout: Duration) -> Result<Option<Incoming>, String> {
        if timeout.is_zero() {
            let result = {
                let rx = self
                    .frames_rx
                    .lock()
                    .map_err(|_| "frames receiver lock poisoned".to_string())?;
                rx.try_recv()
            };
            return match result {
                Ok(incoming) => Ok(Some(incoming)),
                Err(TryRecvError::Empty) => Ok(None),
                Err(TryRecvError::Disconnected) => {
                    ipc_debug_log("read_next_incoming zero-timeout: daemon stream disconnected");
                    self.broken.store(true, Ordering::Release);
                    Err("daemon stream disconnected".to_string())
                }
            };
        }

        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            let chunk = remaining.min(Duration::from_millis(READ_NEXT_INCOMING_LOCK_SLICE_MS));
            let result = {
                let rx = self
                    .frames_rx
                    .lock()
                    .map_err(|_| "frames receiver lock poisoned".to_string())?;
                rx.recv_timeout(chunk)
            };
            match result {
                Ok(incoming) => return Ok(Some(incoming)),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    ipc_debug_log("read_next_incoming timed wait: daemon stream disconnected");
                    self.broken.store(true, Ordering::Release);
                    return Err("daemon stream disconnected".to_string());
                }
            }
        }
    }

    fn write_request_direct(
        &self,
        payload: &[u8],
        request_id: &str,
        deadline: Instant,
    ) -> Result<(), String> {
        if self.broken.load(Ordering::Acquire) {
            ipc_debug_log(&format!(
                "reject request write because client is already broken request_id={request_id}"
            ));
            return Err("daemon stream is in failed state; reconnect is required".to_string());
        }
        let mut writer = self.lock_writer_with_deadline(request_id, deadline)?;
        match write_payload_with_deadline(&mut writer, payload, request_id, deadline) {
            Ok(()) => Ok(()),
            Err(err) => {
                ipc_debug_log(&format!(
                    "mark client broken after write error request_id={request_id}: {err}"
                ));
                self.broken.store(true, Ordering::Release);
                Err(err)
            }
        }
    }

    fn lock_writer_with_deadline(
        &self,
        request_id: &str,
        deadline: Instant,
    ) -> Result<MutexGuard<'_, Box<dyn ReadWriteStream>>, String> {
        loop {
            if Instant::now() >= deadline {
                return Err(format!(
                    "request timeout while waiting writer lock for id '{request_id}'"
                ));
            }
            match self.writer.try_lock() {
                Ok(value) => return Ok(value),
                Err(TryLockError::WouldBlock) => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(TryLockError::Poisoned(_)) => {
                    return Err("writer lock poisoned".to_string());
                }
            }
        }
    }

    fn take_matching_response_from_backlog(&self, id: &str) -> Result<Option<ServerFrame>, String> {
        let mut backlog = self
            .backlog
            .lock()
            .map_err(|_| "backlog lock poisoned".to_string())?;
        let index = backlog
            .iter()
            .position(|frame| frame.id.as_deref() == Some(id));
        Ok(index.and_then(|value| backlog.remove(value)))
    }

    fn take_event_from_backlog(&self) -> Result<Option<Event>, String> {
        let mut backlog = self
            .backlog
            .lock()
            .map_err(|_| "backlog lock poisoned".to_string())?;
        let index = backlog
            .iter()
            .position(|frame| matches!(frame.body, ServerBody::Event { .. }));
        let frame = index.and_then(|value| backlog.remove(value));
        Ok(frame.and_then(|value| match value.body {
            ServerBody::Event { event } => Some(event),
            ServerBody::Response { .. } => None,
        }))
    }

    fn wait_for_matching_response_until_deadline(
        &self,
        id: &str,
        deadline: Instant,
    ) -> Result<Option<ServerFrame>, String> {
        loop {
            if let Some(frame) = self.take_matching_response_from_backlog(id)? {
                return Ok(Some(frame));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

fn write_payload_with_deadline(
    writer: &mut Box<dyn ReadWriteStream>,
    payload: &[u8],
    request_id: &str,
    deadline: Instant,
) -> Result<(), String> {
    let mut offset = 0usize;
    while offset < payload.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("request timeout while writing id '{request_id}'"));
        }
        writer
            .set_write_timeout(Some(remaining))
            .map_err(|err| format!("set write timeout failed: {err}"))?;

        match writer.write(&payload[offset..]) {
            Ok(0) => return Err("write request failed: wrote zero bytes".to_string()),
            Ok(written) => {
                offset = offset.saturating_add(written);
            }
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut
                    || err.kind() == std::io::ErrorKind::Interrupted =>
            {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(err) => return Err(format!("write request failed: {err}")),
        }
    }

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("request timeout while flushing id '{request_id}'"));
        }
        writer
            .set_write_timeout(Some(remaining))
            .map_err(|err| format!("set write timeout failed: {err}"))?;
        match writer.flush() {
            Ok(()) => return Ok(()),
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut
                    || err.kind() == std::io::ErrorKind::Interrupted =>
            {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(err) => return Err(format!("flush failed: {err}")),
        }
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

fn ipc_debug_enabled() -> bool {
    std::env::var("CHATMINAL_DEBUG_IPC")
        .ok()
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn ipc_debug_log(message: &str) {
    if ipc_debug_enabled() {
        eprintln!("chatminal-app ipc-debug: {message}");
    }
}
