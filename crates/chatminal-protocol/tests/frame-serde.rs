use chatminal_protocol::{
    ClientFrame, DaemonHealthEvent, Event, LifecyclePreferences, PingResponse, PtyOutputEvent,
    Request, Response, ServerBody, ServerFrame, SessionExplorerState,
};

#[test]
fn request_frame_roundtrip() {
    let frame = ClientFrame {
        id: "req-1".to_string(),
        request: Request::SessionCreate {
            name: Some("Dev".to_string()),
            cols: 120,
            rows: 40,
            cwd: Some("/tmp".to_string()),
            persist_history: Some(true),
        },
    };

    let encoded = serde_json::to_string(&frame).expect("serialize request");
    let decoded: ClientFrame = serde_json::from_str(&encoded).expect("deserialize request");

    assert_eq!(decoded.id, "req-1");
    match decoded.request {
        Request::SessionCreate {
            name,
            cols,
            rows,
            cwd,
            persist_history,
        } => {
            assert_eq!(name.as_deref(), Some("Dev"));
            assert_eq!(cols, 120);
            assert_eq!(rows, 40);
            assert_eq!(cwd.as_deref(), Some("/tmp"));
            assert_eq!(persist_history, Some(true));
        }
        other => panic!("unexpected request variant: {other:?}"),
    }
}

#[test]
fn response_frame_roundtrip() {
    let frame = ServerFrame::ok(
        "ping".to_string(),
        Response::Ping(PingResponse {
            message: "pong chatminald/1".to_string(),
        }),
    );

    let encoded = serde_json::to_string(&frame).expect("serialize response");
    let decoded: ServerFrame = serde_json::from_str(&encoded).expect("deserialize response");

    assert_eq!(decoded.id.as_deref(), Some("ping"));
    match decoded.body {
        ServerBody::Response {
            ok,
            response,
            error,
        } => {
            assert!(ok);
            assert!(error.is_none());
            match response {
                Some(Response::Ping(payload)) => {
                    assert_eq!(payload.message, "pong chatminald/1");
                }
                other => panic!("unexpected response payload: {other:?}"),
            }
        }
        other => panic!("unexpected server body: {other:?}"),
    }
}

#[test]
fn event_frame_roundtrip() {
    let frame = ServerFrame::event(Event::PtyOutput(PtyOutputEvent {
        session_id: "session-1".to_string(),
        chunk: "hello\n".to_string(),
        seq: 7,
        ts: 123,
    }));

    let encoded = serde_json::to_string(&frame).expect("serialize event");
    let decoded: ServerFrame = serde_json::from_str(&encoded).expect("deserialize event");

    assert!(decoded.id.is_none());
    match decoded.body {
        ServerBody::Event {
            event: Event::PtyOutput(payload),
        } => {
            assert_eq!(payload.session_id, "session-1");
            assert_eq!(payload.chunk, "hello\n");
            assert_eq!(payload.seq, 7);
            assert_eq!(payload.ts, 123);
        }
        other => panic!("unexpected event body: {other:?}"),
    }
}

#[test]
fn daemon_health_event_roundtrip() {
    let frame = ServerFrame::event(Event::DaemonHealth(DaemonHealthEvent {
        connected_clients: 2,
        session_count: 4,
        running_sessions: 1,
        ts: 777,
    }));
    let encoded = serde_json::to_string(&frame).expect("serialize health event");
    let decoded: ServerFrame = serde_json::from_str(&encoded).expect("deserialize health event");

    match decoded.body {
        ServerBody::Event {
            event: Event::DaemonHealth(payload),
        } => {
            assert_eq!(payload.connected_clients, 2);
            assert_eq!(payload.session_count, 4);
            assert_eq!(payload.running_sessions, 1);
            assert_eq!(payload.ts, 777);
        }
        other => panic!("unexpected health event body: {other:?}"),
    }
}

#[test]
fn lifecycle_preferences_response_roundtrip() {
    let frame = ServerFrame::ok(
        "prefs".to_string(),
        Response::LifecyclePreferences(LifecyclePreferences {
            keep_alive_on_close: true,
            start_in_tray: false,
        }),
    );
    let encoded = serde_json::to_string(&frame).expect("serialize lifecycle response");
    let decoded: ServerFrame = serde_json::from_str(&encoded).expect("deserialize lifecycle");

    match decoded.body {
        ServerBody::Response { ok, response, .. } => {
            assert!(ok);
            match response {
                Some(Response::LifecyclePreferences(value)) => {
                    assert!(value.keep_alive_on_close);
                    assert!(!value.start_in_tray);
                }
                other => panic!("unexpected lifecycle response payload: {other:?}"),
            }
        }
        other => panic!("unexpected lifecycle response body: {other:?}"),
    }
}

#[test]
fn explorer_state_request_and_response_roundtrip() {
    let request = ClientFrame {
        id: "explorer-req".to_string(),
        request: Request::SessionExplorerStateGet {
            session_id: "session-123".to_string(),
        },
    };
    let encoded_request = serde_json::to_string(&request).expect("serialize explorer request");
    let decoded_request: ClientFrame =
        serde_json::from_str(&encoded_request).expect("deserialize explorer request");
    match decoded_request.request {
        Request::SessionExplorerStateGet { session_id } => {
            assert_eq!(session_id, "session-123");
        }
        other => panic!("unexpected explorer request variant: {other:?}"),
    }

    let response = ServerFrame::ok(
        "explorer-req".to_string(),
        Response::SessionExplorerState(SessionExplorerState {
            session_id: "session-123".to_string(),
            root_path: Some("/tmp".to_string()),
            current_dir: "src".to_string(),
            selected_path: Some("src/main.rs".to_string()),
            open_file_path: Some("src/main.rs".to_string()),
        }),
    );
    let encoded_response = serde_json::to_string(&response).expect("serialize explorer response");
    let decoded_response: ServerFrame =
        serde_json::from_str(&encoded_response).expect("deserialize explorer response");
    match decoded_response.body {
        ServerBody::Response {
            ok,
            response: Some(Response::SessionExplorerState(state)),
            ..
        } => {
            assert!(ok);
            assert_eq!(state.session_id, "session-123");
            assert_eq!(state.root_path.as_deref(), Some("/tmp"));
            assert_eq!(state.current_dir, "src");
        }
        other => panic!("unexpected explorer response body: {other:?}"),
    }
}

#[test]
fn explorer_read_file_request_and_file_response_roundtrip() {
    let request = ClientFrame {
        id: "explorer-read".to_string(),
        request: Request::SessionExplorerReadFile {
            session_id: "session-abc".to_string(),
            relative_path: "src/main.rs".to_string(),
            max_bytes: Some(4096),
        },
    };
    let encoded_request = serde_json::to_string(&request).expect("serialize read-file request");
    let decoded_request: ClientFrame =
        serde_json::from_str(&encoded_request).expect("deserialize read-file request");
    match decoded_request.request {
        Request::SessionExplorerReadFile {
            session_id,
            relative_path,
            max_bytes,
        } => {
            assert_eq!(session_id, "session-abc");
            assert_eq!(relative_path, "src/main.rs");
            assert_eq!(max_bytes, Some(4096));
        }
        other => panic!("unexpected explorer read-file request variant: {other:?}"),
    }

    let response = ServerFrame::ok(
        "explorer-read".to_string(),
        Response::SessionExplorerFileContent(chatminal_protocol::SessionExplorerFileContent {
            relative_path: "src/main.rs".to_string(),
            content: "fn main() {}\n".to_string(),
            truncated: false,
            byte_len: 13,
        }),
    );
    let encoded_response =
        serde_json::to_string(&response).expect("serialize explorer file response");
    let decoded_response: ServerFrame =
        serde_json::from_str(&encoded_response).expect("deserialize explorer file response");
    match decoded_response.body {
        ServerBody::Response {
            ok,
            response: Some(Response::SessionExplorerFileContent(file)),
            ..
        } => {
            assert!(ok);
            assert_eq!(file.relative_path, "src/main.rs");
            assert_eq!(file.content, "fn main() {}\n");
            assert!(!file.truncated);
            assert_eq!(file.byte_len, 13);
        }
        other => panic!("unexpected explorer file response body: {other:?}"),
    }
}
