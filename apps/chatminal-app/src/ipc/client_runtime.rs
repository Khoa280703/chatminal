use std::collections::VecDeque;
use std::io::Read;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use chatminal_protocol::{Response, ServerBody, ServerFrame};

use super::frame_decoder::decode_pending_frames;
use super::transport::ReadWriteStream;

pub(super) const MAX_FRAME_QUEUE: usize = 2048;
pub(super) const MAX_BACKLOG_FRAMES: usize = 2048;

pub(super) enum Incoming {
    Frame(ServerFrame),
    ProtocolError(String),
}

pub(super) fn read_frames_loop(mut reader: Box<dyn ReadWriteStream>, tx: SyncSender<Incoming>) {
    let mut buf = [0u8; 4096];
    let mut pending = Vec::<u8>::new();
    loop {
        let read = match reader.read(&mut buf) {
            Ok(value) => value,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(_) => break,
        };
        if read == 0 {
            break;
        }

        let (frames, protocol_error) = decode_pending_frames(&mut pending, &buf[..read]);
        for frame in frames {
            if tx.send(Incoming::Frame(frame)).is_err() {
                return;
            }
        }
        if let Some(message) = protocol_error
            && tx.send(Incoming::ProtocolError(message)).is_err()
        {
            return;
        }
    }
}

pub(super) fn push_backlog_limited(backlog: &mut VecDeque<ServerFrame>, frame: ServerFrame) {
    if backlog.len() >= MAX_BACKLOG_FRAMES {
        let incoming_is_response = frame.id.is_some();
        if incoming_is_response {
            if let Some(event_index) = backlog.iter().position(|value| value.id.is_none()) {
                backlog.remove(event_index);
            } else {
                backlog.pop_front();
            }
        } else if let Some(event_index) = backlog.iter().position(|value| value.id.is_none()) {
            backlog.remove(event_index);
        } else {
            return;
        }
    }
    backlog.push_back(frame);
}

pub(super) fn response_from_frame(
    frame: ServerFrame,
    request_id: &str,
) -> Result<Response, String> {
    match frame.body {
        ServerBody::Response {
            ok,
            response,
            error,
        } => {
            if ok {
                response.ok_or_else(|| format!("missing response body for '{request_id}'"))
            } else {
                Err(error.unwrap_or_else(|| format!("request '{request_id}' failed")))
            }
        }
        ServerBody::Event { .. } => Err(format!(
            "unexpected event frame while waiting response for '{request_id}'"
        )),
    }
}
