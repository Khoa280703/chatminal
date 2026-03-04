use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chatminal_protocol::{
    ClientFrame, Event, PingResponse, PtyErrorEvent, Request, Response, ServerFrame, SessionInfo,
    WorkspaceState,
};

use crate::ipc::frame_decoder::{MAX_FRAME_BUFFER_BYTES, decode_pending_frames};

use super::{ChatminalClient, MAX_BACKLOG_FRAMES, push_backlog_limited};

#[test]
fn decode_pending_frames_clears_when_buffer_exceeds_limit() {
    let mut pending = vec![b'a'; MAX_FRAME_BUFFER_BYTES - 4];
    let (frames, error) = decode_pending_frames(&mut pending, b"bbbbbbbbbb");
    assert!(frames.is_empty());
    assert!(error.is_some());
    assert!(pending.is_empty());
}

#[test]
fn push_backlog_limited_drops_oldest_when_full() {
    let mut backlog = VecDeque::new();
    for index in 0..(MAX_BACKLOG_FRAMES + 10) {
        let frame = ServerFrame::event(Event::PtyError(PtyErrorEvent {
            session_id: format!("s-{index}"),
            message: "x".to_string(),
        }));
        push_backlog_limited(&mut backlog, frame);
    }
    assert_eq!(backlog.len(), MAX_BACKLOG_FRAMES);
}

#[cfg(unix)]
#[test]
fn concurrent_requests_receive_correct_response_variant() {
    use std::os::unix::net::UnixStream;

    let (client_stream, server_stream) = UnixStream::pair().expect("unix pair");
    let client = Arc::new(ChatminalClient::from_stream(Box::new(client_stream)).expect("client"));

    let server_handle = thread::spawn(move || {
        let mut reader = BufReader::new(server_stream.try_clone().expect("clone server stream"));
        let mut writer = server_stream;

        let first = read_client_frame(&mut reader);
        let second = read_client_frame(&mut reader);

        let first_response = response_for_request(&first);
        let second_response = response_for_request(&second);

        write_server_frame(
            &mut writer,
            ServerFrame::ok(second.id.clone(), second_response),
        );
        write_server_frame(
            &mut writer,
            ServerFrame::ok(first.id.clone(), first_response),
        );
    });

    let c1 = Arc::clone(&client);
    let t1 = thread::spawn(move || {
        c1.request(Request::WorkspaceLoad, Duration::from_secs(2))
            .expect("workspace request")
    });
    let c2 = Arc::clone(&client);
    let t2 = thread::spawn(move || {
        c2.request(Request::SessionList, Duration::from_secs(2))
            .expect("session request")
    });

    let r1 = t1.join().expect("thread 1");
    let r2 = t2.join().expect("thread 2");
    server_handle.join().expect("server thread");

    assert!(matches!(r1, Response::Workspace(_)));
    assert!(matches!(r2, Response::Sessions(_)));
}

#[cfg(unix)]
fn read_client_frame(reader: &mut BufReader<std::os::unix::net::UnixStream>) -> ClientFrame {
    let mut line = String::new();
    reader.read_line(&mut line).expect("read request line");
    serde_json::from_str::<ClientFrame>(line.trim()).expect("decode client frame")
}

#[cfg(unix)]
fn response_for_request(frame: &ClientFrame) -> Response {
    match frame.request {
        Request::WorkspaceLoad => Response::Workspace(WorkspaceState {
            profiles: Vec::new(),
            active_profile_id: None,
            sessions: Vec::new(),
            active_session_id: None,
        }),
        Request::SessionList => Response::Sessions(Vec::<SessionInfo>::new()),
        _ => Response::Ping(PingResponse {
            message: "unexpected".to_string(),
        }),
    }
}

#[cfg(unix)]
fn write_server_frame(writer: &mut std::os::unix::net::UnixStream, frame: ServerFrame) {
    let encoded = serde_json::to_string(&frame).expect("encode frame");
    writer.write_all(encoded.as_bytes()).expect("write frame");
    writer.write_all(b"\n").expect("write newline");
    writer.flush().expect("flush frame");
}
