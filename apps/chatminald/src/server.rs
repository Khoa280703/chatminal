use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::Duration;

use chatminal_protocol::{ClientFrame, ServerFrame};

use crate::state::DaemonState;
use crate::transport::{ActiveTransport, LocalStream, TransportBackend, TransportListener};
const MAX_REQUEST_LINE_BYTES: usize = 256 * 1024;

pub fn run_server(endpoint: &str, state: DaemonState) -> Result<(), String> {
    let listener = ActiveTransport::bind(endpoint)?;

    log::info!("chatminald listening on {}", endpoint);

    let next_client_id = Arc::new(AtomicU64::new(1));
    let health_interval_ms = state.health_interval_ms();
    let health_state = state.clone();
    thread::spawn(move || {
        while !health_state.is_shutdown_requested() {
            thread::sleep(Duration::from_millis(health_interval_ms));
            if health_state.is_shutdown_requested() {
                break;
            }
            health_state.broadcast_daemon_health();
            health_state.log_runtime_metrics();
        }
    });

    while !state.is_shutdown_requested() {
        match listener.accept_stream()? {
            Some(stream) => {
                let client_id = next_client_id.fetch_add(1, Ordering::Relaxed);
                let state_for_client = state.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_client(client_id, stream, state_for_client) {
                        log::warn!("client {} disconnected with error: {}", client_id, err);
                    }
                });
            }
            None => {
                thread::sleep(Duration::from_millis(25));
            }
        }
    }

    ActiveTransport::cleanup(endpoint);
    log::info!("chatminald stopped");
    Ok(())
}

fn handle_client(
    client_id: u64,
    mut stream: LocalStream,
    state: DaemonState,
) -> Result<(), String> {
    let writer_stream = stream
        .try_clone()
        .map_err(|err| format!("clone stream failed: {err}"))?;

    let (tx, rx) = std_mpsc::sync_channel::<ServerFrame>(1024);
    state.register_client(client_id, tx.clone());

    let writer = thread::spawn(move || {
        let mut writer = writer_stream;
        while let Ok(frame) = rx.recv() {
            let encoded = match serde_json::to_string(&frame) {
                Ok(value) => value,
                Err(err) => {
                    log::warn!("serialize frame failed: {err}");
                    continue;
                }
            };
            if writer.write_all(encoded.as_bytes()).is_err() {
                break;
            }
            if writer.write_all(b"\n").is_err() {
                break;
            }
            let _ = writer.flush();
        }
    });

    let mut read_buf = [0u8; 4096];
    let mut pending = Vec::<u8>::new();

    loop {
        let read = stream
            .read(&mut read_buf)
            .map_err(|err| format!("read client bytes failed: {err}"))?;
        if read == 0 {
            break;
        }

        pending.extend_from_slice(&read_buf[..read]);
        if pending.len() > MAX_REQUEST_LINE_BYTES {
            let _ = tx.try_send(ServerFrame::err(
                "too-large".to_string(),
                format!(
                    "request buffer too large (>{} bytes without newline)",
                    MAX_REQUEST_LINE_BYTES
                ),
            ));
            break;
        }

        while let Some(line_end) = pending.iter().position(|byte| *byte == b'\n') {
            let mut line_bytes = pending.drain(..=line_end).collect::<Vec<u8>>();
            if line_bytes.len() > MAX_REQUEST_LINE_BYTES {
                let _ = tx.try_send(ServerFrame::err(
                    "too-large".to_string(),
                    format!(
                        "request line too large ({} bytes > {} bytes)",
                        line_bytes.len(),
                        MAX_REQUEST_LINE_BYTES
                    ),
                ));
                continue;
            }

            if line_bytes.ends_with(b"\n") {
                line_bytes.pop();
            }
            if line_bytes.ends_with(b"\r") {
                line_bytes.pop();
            }
            if line_bytes.is_empty() {
                continue;
            }

            let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
            if line.is_empty() {
                continue;
            }

            let frame: ClientFrame = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(err) => {
                    let _ = tx.try_send(ServerFrame::err(
                        "invalid".to_string(),
                        format!("invalid request frame: {err}"),
                    ));
                    continue;
                }
            };

            let response = state.handle_request(frame);
            let _ = tx.try_send(response);

            if state.is_shutdown_requested() {
                break;
            }
        }
    }

    state.unregister_client(client_id);
    drop(tx);
    let _ = writer.join();
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use chatminal_protocol::{ClientFrame, Event, Request, Response, ServerBody, ServerFrame};
    use chatminal_store::Store;

    use crate::config::DaemonConfig;
    use crate::state::DaemonState;
    use crate::transport::ensure_socket_path;

    use super::{MAX_REQUEST_LINE_BYTES, run_server};

    struct TestServer {
        endpoint: String,
        db_path: PathBuf,
        handle: Option<std::thread::JoinHandle<Result<(), String>>>,
    }

    impl TestServer {
        fn spawn() -> Self {
            let unique = format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_nanos())
                    .unwrap_or(0)
            );
            let endpoint = format!("/tmp/chatminald-test-{unique}.sock");
            let db_path = std::env::temp_dir().join(format!("chatminald-test-{unique}.db"));
            Self::spawn_with_endpoint(endpoint, db_path)
        }

        fn spawn_with_endpoint(endpoint: String, db_path: PathBuf) -> Self {
            let store = Store::initialize(&db_path).expect("initialize test store");
            let config = DaemonConfig {
                endpoint: endpoint.clone(),
                default_shell: "/bin/sh".to_string(),
                default_preview_lines: 1000,
                max_scrollback_lines_per_session: 5_000,
                default_cols: 120,
                default_rows: 32,
                health_interval_ms: 1_000,
            };
            let state = DaemonState::new(config.clone(), store).expect("create daemon state");

            let endpoint_clone = endpoint.clone();
            let handle = std::thread::spawn(move || run_server(&endpoint_clone, state));
            wait_for_server(&endpoint);

            Self {
                endpoint,
                db_path,
                handle: Some(handle),
            }
        }

        fn connect(&self) -> UnixStream {
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                match UnixStream::connect(&self.endpoint) {
                    Ok(stream) => return stream,
                    Err(err) if Instant::now() < deadline => {
                        let _ = err;
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(err) => panic!("connect test server failed: {err}"),
                }
            }
        }

        fn shutdown_assert_ok(&mut self) {
            let _ = send_shutdown(&self.endpoint);
            if let Some(handle) = self.handle.take() {
                let result = handle.join().expect("join daemon server thread");
                assert!(result.is_ok(), "server returned error: {result:?}");
            }
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            if self.handle.is_some() {
                let _ = send_shutdown(&self.endpoint);
            }
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
            let _ = std::fs::remove_file(&self.endpoint);
            let _ = std::fs::remove_file(&self.db_path);
        }
    }

    fn wait_for_server(endpoint: &str) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match UnixStream::connect(endpoint) {
                Ok(stream) => {
                    drop(stream);
                    return;
                }
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(err) => panic!("server did not become ready: {err}"),
            }
        }
    }

    fn write_client_frame(
        stream: &mut UnixStream,
        request_id: &str,
        request: Request,
    ) -> Result<(), String> {
        let frame = ClientFrame {
            id: request_id.to_string(),
            request,
        };
        let encoded = serde_json::to_string(&frame)
            .map_err(|err| format!("encode client frame failed: {err}"))?;
        stream
            .write_all(encoded.as_bytes())
            .map_err(|err| format!("write client frame failed: {err}"))?;
        stream
            .write_all(b"\n")
            .map_err(|err| format!("write newline failed: {err}"))?;
        stream
            .flush()
            .map_err(|err| format!("flush client frame failed: {err}"))?;
        Ok(())
    }

    fn read_frame_by_id(
        reader: &mut BufReader<UnixStream>,
        expected_id: &str,
    ) -> Result<ServerFrame, String> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if Instant::now() >= deadline {
                return Err(format!(
                    "timeout waiting response frame with id '{expected_id}'"
                ));
            }

            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                }
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(err) if err.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(err) => return Err(format!("read daemon frame failed: {err}")),
            }

            if line.trim().is_empty() {
                continue;
            }
            let frame = serde_json::from_str::<ServerFrame>(line.trim())
                .map_err(|err| format!("decode daemon frame failed: {err}"))?;
            if frame.id.as_deref() == Some(expected_id) {
                return Ok(frame);
            }
        }
    }

    fn assert_error_response(frame: ServerFrame, expected_id: &str, expected_message: &str) {
        assert_eq!(frame.id.as_deref(), Some(expected_id));
        match frame.body {
            ServerBody::Response {
                ok,
                response: _,
                error,
            } => {
                assert!(!ok, "expected failed response");
                let message = error.unwrap_or_default();
                assert!(
                    message.contains(expected_message),
                    "expected error message to contain '{expected_message}', got '{message}'"
                );
            }
            _ => panic!("expected response frame, got event"),
        }
    }

    fn assert_ping_response(frame: ServerFrame, expected_id: &str) {
        let response = extract_response(frame, expected_id).expect("extract ping response");
        match response {
            Response::Ping(value) => {
                assert_eq!(value.message, "pong chatminald/1");
            }
            other => panic!("expected ping response, got {other:?}"),
        }
    }

    fn send_shutdown(endpoint: &str) -> Result<(), String> {
        let mut stream = UnixStream::connect(endpoint)
            .map_err(|err| format!("connect shutdown stream failed: {err}"))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .map_err(|err| format!("set read timeout failed: {err}"))?;
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .map_err(|err| format!("clone shutdown stream failed: {err}"))?,
        );
        write_client_frame(&mut stream, "shutdown", Request::AppShutdown)?;
        let _ = read_frame_by_id(&mut reader, "shutdown")?;
        Ok(())
    }

    fn read_output_until_contains(
        reader: &mut BufReader<UnixStream>,
        needle_a: &str,
        needle_b: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        let mut aggregate = String::new();
        while Instant::now() < deadline {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                }
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(err) if err.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(err) => return Err(format!("read daemon output failed: {err}")),
            }
            if line.trim().is_empty() {
                continue;
            }
            let frame = serde_json::from_str::<ServerFrame>(line.trim())
                .map_err(|err| format!("decode daemon frame failed: {err}"))?;
            if let ServerBody::Event {
                event: Event::PtyOutput(output),
            } = frame.body
            {
                aggregate.push_str(&output.chunk);
                if aggregate.contains(needle_a) && aggregate.contains(needle_b) {
                    return Ok(());
                }
            }
        }
        Err(format!(
            "timeout waiting pty output containing '{needle_a}' and '{needle_b}'"
        ))
    }

    fn extract_response(frame: ServerFrame, expected_id: &str) -> Result<Response, String> {
        if frame.id.as_deref() != Some(expected_id) {
            return Err(format!(
                "unexpected response id: expected '{expected_id}', got '{:?}'",
                frame.id
            ));
        }
        match frame.body {
            ServerBody::Response {
                ok: true,
                response: Some(response),
                ..
            } => Ok(response),
            ServerBody::Response {
                ok: false, error, ..
            } => Err(error.unwrap_or_else(|| "unknown request failure".to_string())),
            _ => Err("expected response frame".to_string()),
        }
    }

    #[test]
    fn ensure_socket_path_rejects_regular_files() {
        let path = std::env::temp_dir().join(format!(
            "chatminald-non-socket-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, b"not-a-socket").expect("write temp regular file");

        let err = ensure_socket_path(path.to_str().expect("path utf8"))
            .expect_err("regular file path must be rejected");
        assert!(err.contains("not a unix socket"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn ensure_socket_path_rejects_active_socket() {
        let path = std::env::temp_dir().join(format!(
            "chatminald-active-socket-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        let listener = UnixListener::bind(&path).expect("bind active test socket");

        let err = ensure_socket_path(path.to_str().expect("path utf8"))
            .expect_err("active socket must be rejected");
        assert!(err.contains("already in use"));

        drop(listener);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn ensure_socket_path_cleans_stale_socket() {
        let path = std::env::temp_dir().join(format!(
            "chatminald-stale-socket-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        {
            let listener = UnixListener::bind(&path).expect("bind stale test socket");
            drop(listener);
        }
        assert!(path.exists(), "stale socket path must exist before cleanup");

        ensure_socket_path(path.to_str().expect("path utf8")).expect("cleanup stale socket");
        assert!(!path.exists(), "stale socket must be removed");
    }

    #[test]
    fn listener_sets_socket_permissions_to_user_only() {
        let mut server = TestServer::spawn();
        let mode = std::fs::metadata(&server.endpoint)
            .expect("socket metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        server.shutdown_assert_ok();
    }

    #[test]
    fn client_can_reconnect_after_disconnect() {
        let mut server = TestServer::spawn();

        let mut stream = server.connect();
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("set read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone first stream"));
        write_client_frame(&mut stream, "ping-1", Request::Ping).expect("write first ping");
        let frame = read_frame_by_id(&mut reader, "ping-1").expect("read first ping response");
        assert_ping_response(frame, "ping-1");
        drop(reader);
        drop(stream);

        let mut stream = server.connect();
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("set read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone second stream"));
        write_client_frame(&mut stream, "ping-2", Request::Ping).expect("write second ping");
        let frame = read_frame_by_id(&mut reader, "ping-2").expect("read second ping response");
        assert_ping_response(frame, "ping-2");

        server.shutdown_assert_ok();
    }

    #[test]
    fn server_boots_on_stale_socket_and_serves_requests() {
        let unique = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        );
        let endpoint = format!("/tmp/chatminald-stale-boot-{unique}.sock");
        let db_path = std::env::temp_dir().join(format!("chatminald-stale-boot-{unique}.db"));
        {
            let listener = UnixListener::bind(&endpoint).expect("create stale socket");
            drop(listener);
        }
        assert!(
            std::path::Path::new(&endpoint).exists(),
            "stale socket must exist before server boot"
        );

        let mut server = TestServer::spawn_with_endpoint(endpoint.clone(), db_path);
        let mut stream = server.connect();
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("set read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        write_client_frame(&mut stream, "ping-stale", Request::Ping).expect("write ping");
        let frame =
            read_frame_by_id(&mut reader, "ping-stale").expect("read ping response after stale");
        assert_ping_response(frame, "ping-stale");
        server.shutdown_assert_ok();
    }

    #[test]
    fn oversized_request_without_newline_returns_too_large_error() {
        let mut server = TestServer::spawn();
        let mut stream = server.connect();
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("set read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone client stream"));

        let payload = vec![b'a'; MAX_REQUEST_LINE_BYTES + 1];
        stream.write_all(&payload).expect("write oversized payload");
        stream.flush().expect("flush oversized payload");

        let frame =
            read_frame_by_id(&mut reader, "too-large").expect("must receive too-large response");
        assert_error_response(frame, "too-large", "request buffer too large");

        server.shutdown_assert_ok();
    }

    #[test]
    fn invalid_json_line_returns_invalid_error() {
        let mut server = TestServer::spawn();
        let mut stream = server.connect();
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("set read timeout");
        let mut reader = BufReader::new(stream.try_clone().expect("clone client stream"));

        stream
            .write_all(b"{not-json}\n")
            .expect("write malformed frame");
        stream.flush().expect("flush malformed frame");

        let frame =
            read_frame_by_id(&mut reader, "invalid").expect("must receive invalid response");
        assert_error_response(frame, "invalid", "invalid request frame");

        server.shutdown_assert_ok();
    }

    #[test]
    fn restart_restores_persisted_session_snapshot() {
        let unique = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        );
        let endpoint = format!("/tmp/chatminald-restart-{unique}.sock");
        let db_path = std::env::temp_dir().join(format!("chatminald-restart-{unique}.db"));

        let session_id = {
            let store = Store::initialize(&db_path).expect("initialize store");
            let config = DaemonConfig {
                endpoint: endpoint.clone(),
                default_shell: "/bin/sh".to_string(),
                default_preview_lines: 1_000,
                max_scrollback_lines_per_session: 5_000,
                default_cols: 120,
                default_rows: 32,
                health_interval_ms: 1_000,
            };
            let state = DaemonState::new(config, store).expect("create daemon state");
            let endpoint_clone = endpoint.clone();
            let handle = std::thread::spawn(move || run_server(&endpoint_clone, state));
            wait_for_server(&endpoint);

            let mut stream = UnixStream::connect(&endpoint).expect("connect first server");
            stream
                .set_read_timeout(Some(Duration::from_millis(200)))
                .expect("set read timeout");
            let mut reader = BufReader::new(stream.try_clone().expect("clone first stream"));

            write_client_frame(
                &mut stream,
                "create",
                Request::SessionCreate {
                    name: Some("restart-test".to_string()),
                    cols: 120,
                    rows: 32,
                    cwd: None,
                    persist_history: Some(true),
                },
            )
            .expect("write create request");
            let created = extract_response(
                read_frame_by_id(&mut reader, "create").expect("read create response"),
                "create",
            )
            .expect("parse create response");
            let session_id = match created {
                Response::SessionCreate(value) => value.session_id,
                other => panic!("unexpected create response: {other:?}"),
            };

            write_client_frame(
                &mut stream,
                "input",
                Request::SessionInputWrite {
                    session_id: session_id.clone(),
                    data: "printf 'retain-a\\nretain-b\\n'\n".to_string(),
                },
            )
            .expect("write input request");
            let _ = extract_response(
                read_frame_by_id(&mut reader, "input").expect("read input response"),
                "input",
            )
            .expect("parse input response");
            read_output_until_contains(&mut reader, "retain-a", "retain-b", Duration::from_secs(5))
                .expect("wait for output chunks");

            write_client_frame(&mut stream, "shutdown-1", Request::AppShutdown)
                .expect("write shutdown request");
            let _ = read_frame_by_id(&mut reader, "shutdown-1").expect("read shutdown response");
            let result = handle.join().expect("join first server");
            assert!(result.is_ok(), "first server returned error: {result:?}");
            session_id
        };

        {
            let store = Store::initialize(&db_path).expect("initialize store for restart");
            let config = DaemonConfig {
                endpoint: endpoint.clone(),
                default_shell: "/bin/sh".to_string(),
                default_preview_lines: 1_000,
                max_scrollback_lines_per_session: 5_000,
                default_cols: 120,
                default_rows: 32,
                health_interval_ms: 1_000,
            };
            let state = DaemonState::new(config, store).expect("create daemon state restart");
            let endpoint_clone = endpoint.clone();
            let handle = std::thread::spawn(move || run_server(&endpoint_clone, state));
            wait_for_server(&endpoint);

            let mut stream = UnixStream::connect(&endpoint).expect("connect restarted server");
            stream
                .set_read_timeout(Some(Duration::from_millis(200)))
                .expect("set read timeout");
            let mut reader = BufReader::new(stream.try_clone().expect("clone restart stream"));

            write_client_frame(
                &mut stream,
                "snapshot",
                Request::SessionSnapshotGet {
                    session_id: session_id.clone(),
                    preview_lines: Some(200),
                },
            )
            .expect("write snapshot request");
            let snapshot_response = extract_response(
                read_frame_by_id(&mut reader, "snapshot").expect("read snapshot response"),
                "snapshot",
            )
            .expect("parse snapshot response");
            let snapshot = match snapshot_response {
                Response::SessionSnapshot(value) => value,
                other => panic!("unexpected snapshot response: {other:?}"),
            };
            assert!(snapshot.content.contains("retain-a"));
            assert!(snapshot.content.contains("retain-b"));
            assert!(snapshot.content.find("retain-a") <= snapshot.content.find("retain-b"));

            write_client_frame(&mut stream, "shutdown-2", Request::AppShutdown)
                .expect("write second shutdown request");
            let _ = read_frame_by_id(&mut reader, "shutdown-2").expect("read second shutdown");
            let result = handle.join().expect("join second server");
            assert!(result.is_ok(), "second server returned error: {result:?}");
        }

        let _ = std::fs::remove_file(&endpoint);
        let _ = std::fs::remove_file(&db_path);
    }
}
