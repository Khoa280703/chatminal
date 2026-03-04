use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chatminal_protocol::{ClientFrame, Event, Request, Response, ServerBody, ServerFrame};

use super::client_runtime::{Incoming, MAX_FRAME_QUEUE, read_frames_loop, response_from_frame};
use super::transport::{ReadWriteStream, connect_local_stream};

#[cfg(test)]
pub(super) use super::client_runtime::MAX_BACKLOG_FRAMES;
pub(super) use super::client_runtime::push_backlog_limited;
pub struct ChatminalClient {
    writer: Arc<Mutex<Box<dyn ReadWriteStream>>>,
    frames_rx: Mutex<Receiver<Incoming>>,
    backlog: Mutex<VecDeque<ServerFrame>>,
    next_id: AtomicU64,
}

impl ChatminalClient {
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        let stream = connect_local_stream(endpoint)?;
        Self::from_stream(stream)
    }

    fn from_stream(stream: Box<dyn ReadWriteStream>) -> Result<Self, String> {
        let reader = stream
            .try_clone_boxed()
            .map_err(|err| format!("clone stream failed: {err}"))?;
        let (tx, rx) = mpsc::sync_channel::<Incoming>(MAX_FRAME_QUEUE);
        std::thread::spawn(move || {
            read_frames_loop(reader, tx);
        });

        stream
            .set_read_timeout(Some(Duration::from_millis(300)))
            .map_err(|err| format!("set read timeout failed: {err}"))?;
        stream
            .set_write_timeout(Some(Duration::from_millis(700)))
            .map_err(|err| format!("set write timeout failed: {err}"))?;

        Ok(Self {
            writer: Arc::new(Mutex::new(stream)),
            frames_rx: Mutex::new(rx),
            backlog: Mutex::new(VecDeque::new()),
            next_id: AtomicU64::new(1),
        })
    }

    pub fn request(&self, request: Request, timeout: Duration) -> Result<Response, String> {
        let id = format!("req-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let frame = ClientFrame {
            id: id.clone(),
            request,
        };
        let encoded =
            serde_json::to_string(&frame).map_err(|err| format!("encode request failed: {err}"))?;

        {
            let mut writer = self
                .writer
                .lock()
                .map_err(|_| "writer lock poisoned".to_string())?;
            writer
                .write_all(encoded.as_bytes())
                .map_err(|err| format!("write request failed: {err}"))?;
            writer
                .write_all(b"\n")
                .map_err(|err| format!("write newline failed: {err}"))?;
            writer
                .flush()
                .map_err(|err| format!("flush failed: {err}"))?;
        }

        if let Some(frame) = self.take_matching_response_from_backlog(&id)? {
            return response_from_frame(frame, &id);
        }

        let deadline = Instant::now() + timeout;
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
        let rx = self
            .frames_rx
            .lock()
            .map_err(|_| "frames receiver lock poisoned".to_string())?;
        match rx.recv_timeout(timeout) {
            Ok(incoming) => Ok(Some(incoming)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err("daemon stream disconnected".to_string())
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
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
