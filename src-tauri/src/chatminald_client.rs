use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DaemonPingStatus {
    pub reachable: bool,
    pub latency_ms: Option<u128>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ChatminaldClient {
    endpoint: String,
    connect_timeout: Duration,
    io_timeout: Duration,
}

impl ChatminaldClient {
    pub fn from_env() -> Option<Self> {
        let endpoint = std::env::var("CHATMINAL_DAEMON_ENDPOINT")
            .ok()
            .or_else(|| std::env::var("CHATMINAL_DAEMON_ADDR").ok())
            .or_else(|| std::env::var("CHATMIMAL_DAEMON_ADDR").ok())?;
        let trimmed = endpoint.trim();
        if trimmed.is_empty() {
            return None;
        }

        Some(Self {
            endpoint: trimmed.to_string(),
            connect_timeout: Duration::from_millis(700),
            io_timeout: Duration::from_millis(700),
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn ping(&self) -> DaemonPingStatus {
        let start = Instant::now();
        let mut stream =
            match connect_local_stream(&self.endpoint, self.connect_timeout, self.io_timeout) {
                Ok(stream) => stream,
                Err(None) => {
                    return DaemonPingStatus {
                        reachable: false,
                        latency_ms: None,
                        message: "local IPC is unsupported on this platform".to_string(),
                    };
                }
                Err(Some(message)) => {
                    return DaemonPingStatus {
                        reachable: false,
                        latency_ms: None,
                        message,
                    };
                }
            };

        if let Err(err) = stream.write_all(b"PING chatminal/1\n") {
            return DaemonPingStatus {
                reachable: false,
                latency_ms: None,
                message: format!("write failed: {err}"),
            };
        }

        let mut buffer = [0u8; 128];
        let read = match stream.read(&mut buffer) {
            Ok(value) => value,
            Err(err) => {
                return DaemonPingStatus {
                    reachable: false,
                    latency_ms: None,
                    message: format!("read failed: {err}"),
                };
            }
        };

        let elapsed = start.elapsed().as_millis();
        let response = String::from_utf8_lossy(&buffer[..read]).trim().to_string();
        let is_pong = response == "PONG" || response.starts_with("PONG ");

        if is_pong {
            DaemonPingStatus {
                reachable: true,
                latency_ms: Some(elapsed),
                message: "daemon reachable".to_string(),
            }
        } else {
            DaemonPingStatus {
                reachable: false,
                latency_ms: Some(elapsed),
                message: format!("unexpected daemon response: '{response}'"),
            }
        }
    }
}

trait ReadWriteStream: Read + Write {}

impl<T: Read + Write> ReadWriteStream for T {}

#[cfg(unix)]
fn connect_local_stream(
    endpoint: &str,
    _connect_timeout: Duration,
    io_timeout: Duration,
) -> Result<Box<dyn ReadWriteStream>, Option<String>> {
    let stream = UnixStream::connect(endpoint)
        .map_err(|err| Some(format!("unix socket connect failed ('{endpoint}'): {err}")))?;
    let _ = stream.set_read_timeout(Some(io_timeout));
    let _ = stream.set_write_timeout(Some(io_timeout));
    Ok(Box::new(stream))
}

#[cfg(target_os = "windows")]
fn connect_local_stream(
    endpoint: &str,
    _connect_timeout: Duration,
    _io_timeout: Duration,
) -> Result<Box<dyn ReadWriteStream>, Option<String>> {
    let normalized = normalize_windows_pipe_path(endpoint);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&normalized)
        .map_err(|err| Some(format!("named pipe connect failed ('{normalized}'): {err}")))?;
    Ok(Box::new(file))
}

#[cfg(not(any(unix, target_os = "windows")))]
fn connect_local_stream(
    _endpoint: &str,
    _connect_timeout: Duration,
    _io_timeout: Duration,
) -> Result<Box<dyn ReadWriteStream>, Option<String>> {
    Err(None)
}

#[cfg(target_os = "windows")]
fn normalize_windows_pipe_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with(r"\\.\pipe\") {
        trimmed.to_string()
    } else {
        format!(r"\\.\pipe\{}", trimmed)
    }
}
