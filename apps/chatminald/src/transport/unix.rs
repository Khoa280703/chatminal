use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};

use super::{TransportBackend, TransportListener};

pub(crate) type LocalStream = UnixStream;
pub(crate) struct UnixTransport;

pub(crate) struct LocalListener {
    listener: UnixListener,
}

impl TransportListener for LocalListener {
    type Stream = LocalStream;

    fn accept_stream(&self) -> Result<Option<Self::Stream>, String> {
        match self.listener.accept() {
            Ok((stream, _)) => Ok(Some(stream)),
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::ConnectionAborted
                ) =>
            {
                Ok(None)
            }
            Err(err) => Err(format!("accept client failed: {err}")),
        }
    }
}

impl TransportBackend for UnixTransport {
    type Listener = LocalListener;

    fn bind(endpoint: &str) -> Result<Self::Listener, String> {
        bind_local_listener(endpoint)
    }

    fn cleanup(endpoint: &str) {
        cleanup_local_endpoint(endpoint);
    }
}

fn bind_local_listener(endpoint: &str) -> Result<LocalListener, String> {
    ensure_socket_path(endpoint)?;

    if let Some(parent) = std::path::Path::new(endpoint).parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create socket directory failed: {err}"))?;
    }

    let listener = UnixListener::bind(endpoint)
        .map_err(|err| format!("bind unix socket failed ('{endpoint}'): {err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("set nonblocking listener failed: {err}"))?;

    fs::set_permissions(endpoint, fs::Permissions::from_mode(0o600))
        .map_err(|err| format!("set endpoint permissions failed ('{endpoint}'): {err}"))?;
    Ok(LocalListener { listener })
}

fn cleanup_local_endpoint(endpoint: &str) {
    let _ = fs::remove_file(endpoint);
}

pub(crate) fn ensure_socket_path(endpoint: &str) -> Result<(), String> {
    let path = std::path::Path::new(endpoint);
    if !path.exists() {
        return Ok(());
    }

    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("read existing endpoint metadata failed ('{endpoint}'): {err}"))?;
    if !metadata.file_type().is_socket() {
        return Err(format!(
            "daemon endpoint path exists but is not a unix socket ('{}')",
            endpoint
        ));
    }

    let probe_result = connect_with_retry(endpoint, 3);
    match probe_result {
        Ok(_) => Err(format!("daemon endpoint already in use ('{}')", endpoint)),
        Err(err) => match err.kind() {
            std::io::ErrorKind::ConnectionRefused => {
                fs::remove_file(endpoint).map_err(|remove_err| {
                    format!("remove stale socket failed ('{endpoint}'): {remove_err}")
                })
            }
            std::io::ErrorKind::NotFound => Ok(()),
            _ => Err(format!(
                "probe existing endpoint failed ('{endpoint}'): {err}"
            )),
        },
    }
}

fn connect_with_retry(endpoint: &str, max_attempts: usize) -> std::io::Result<UnixStream> {
    let attempts = max_attempts.max(1);
    let mut last_error = None;
    for _ in 0..attempts {
        match UnixStream::connect(endpoint) {
            Ok(stream) => return Ok(stream),
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {
                last_error = Some(err);
                continue;
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_error.unwrap_or_else(|| std::io::Error::from(std::io::ErrorKind::Interrupted)))
}
