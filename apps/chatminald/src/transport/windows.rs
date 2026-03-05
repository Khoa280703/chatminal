use std::fs::File;
use std::io::{Read, Write};
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_NO_DATA, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED,
    ERROR_PIPE_LISTENING, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_NOWAIT, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    SetNamedPipeHandleState,
};

use super::{TransportBackend, TransportListener};

pub(crate) struct WindowsTransport;

pub(crate) struct LocalListener {
    endpoint: String,
    pending: Mutex<Option<PendingNamedPipe>>,
}

pub(crate) struct LocalStream {
    file: File,
}

impl LocalStream {
    fn from_connected_handle(handle: HANDLE) -> Result<Self, String> {
        let mode = PIPE_READMODE_BYTE | PIPE_WAIT;
        let mode_set =
            unsafe { SetNamedPipeHandleState(handle, &mode, std::ptr::null(), std::ptr::null()) };
        if mode_set == 0 {
            close_raw_handle(handle);
            return Err(format!(
                "set connected named pipe mode failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let file = unsafe { File::from_raw_handle(handle as RawHandle) };
        Ok(Self { file })
    }

    pub(crate) fn try_clone(&self) -> std::io::Result<Self> {
        self.file.try_clone().map(|file| Self { file })
    }
}

impl Read for LocalStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Write for LocalStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl TransportListener for LocalListener {
    type Stream = LocalStream;

    fn accept_stream(&self) -> Result<Option<Self::Stream>, String> {
        let mut pending_guard = self
            .pending
            .lock()
            .map_err(|_| "windows transport pending lock poisoned".to_string())?;

        if pending_guard.is_none() {
            match PendingNamedPipe::create(&self.endpoint, false) {
                Ok(value) => *pending_guard = Some(value),
                Err(err) => {
                    log::warn!("create initial pending named pipe failed: {err}");
                    return Ok(None);
                }
            }
        }

        let connect_state = {
            let pending = pending_guard
                .as_ref()
                .ok_or_else(|| "missing pending named pipe instance".to_string())?;
            pending.try_connect(&self.endpoint)?
        };

        match connect_state {
            PipeConnectState::Pending => Ok(None),
            PipeConnectState::Recreate => {
                let _ = pending_guard.take();
                match PendingNamedPipe::create(&self.endpoint, false) {
                    Ok(next_pending) => *pending_guard = Some(next_pending),
                    Err(err) => {
                        log::warn!("recreate pending named pipe failed: {err}");
                    }
                }
                Ok(None)
            }
            PipeConnectState::Connected => {
                let connected = pending_guard
                    .take()
                    .ok_or_else(|| "missing connected named pipe instance".to_string())?;

                // Pre-create the next listening instance to keep endpoint availability continuous.
                match PendingNamedPipe::create(&self.endpoint, false) {
                    Ok(next_pending) => *pending_guard = Some(next_pending),
                    Err(err) => {
                        log::warn!("pre-create next pending named pipe failed: {err}");
                    }
                }

                drop(pending_guard);
                match connected.into_stream() {
                    Ok(stream) => Ok(Some(stream)),
                    Err(err) => {
                        log::warn!("promote connected named pipe stream failed: {err}");
                        Ok(None)
                    }
                }
            }
        }
    }
}

impl TransportBackend for WindowsTransport {
    type Listener = LocalListener;

    fn bind(endpoint: &str) -> Result<Self::Listener, String> {
        validate_named_pipe_endpoint(endpoint)?;
        let pending = PendingNamedPipe::create(endpoint, true)?;
        Ok(LocalListener {
            endpoint: endpoint.to_string(),
            pending: Mutex::new(Some(pending)),
        })
    }

    fn cleanup(_endpoint: &str) {}
}

enum PipeConnectState {
    Pending,
    Connected,
    Recreate,
}

struct PendingNamedPipe {
    handle: HANDLE,
}

impl PendingNamedPipe {
    fn create(endpoint: &str, first_instance: bool) -> Result<Self, String> {
        let endpoint_wide = to_wide(endpoint);
        let open_mode = if first_instance {
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            PIPE_ACCESS_DUPLEX
        };
        let handle = unsafe {
            CreateNamedPipeW(
                endpoint_wide.as_ptr(),
                open_mode,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                64 * 1024,
                64 * 1024,
                0,
                std::ptr::null(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let code = unsafe { GetLastError() };
            if first_instance && code == ERROR_ACCESS_DENIED {
                return Err(format!("daemon endpoint already in use ('{endpoint}')"));
            }
            return Err(format!(
                "create named pipe failed ('{endpoint}'): {}",
                std::io::Error::from_raw_os_error(code as i32)
            ));
        }
        Ok(Self { handle })
    }

    fn try_connect(&self, endpoint: &str) -> Result<PipeConnectState, String> {
        let connected = unsafe { ConnectNamedPipe(self.handle, std::ptr::null_mut()) };
        if connected != 0 {
            return Ok(PipeConnectState::Connected);
        }

        let code = unsafe { GetLastError() };
        match code {
            ERROR_PIPE_CONNECTED => Ok(PipeConnectState::Connected),
            ERROR_PIPE_LISTENING | ERROR_PIPE_BUSY => Ok(PipeConnectState::Pending),
            ERROR_NO_DATA => {
                let _ = unsafe { DisconnectNamedPipe(self.handle) };
                Ok(PipeConnectState::Recreate)
            }
            _ => Err(format!(
                "connect named pipe failed ('{endpoint}'): {}",
                std::io::Error::from_raw_os_error(code as i32)
            )),
        }
    }

    fn into_stream(self) -> Result<LocalStream, String> {
        let handle = self.handle;
        std::mem::forget(self);
        LocalStream::from_connected_handle(handle)
    }
}

impl Drop for PendingNamedPipe {
    fn drop(&mut self) {
        close_raw_handle(self.handle);
    }
}

fn close_raw_handle(handle: HANDLE) {
    if handle == INVALID_HANDLE_VALUE {
        return;
    }
    let _ = unsafe { CloseHandle(handle) };
}

fn validate_named_pipe_endpoint(endpoint: &str) -> Result<(), String> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err("daemon endpoint cannot be empty".to_string());
    }
    if !trimmed.starts_with(r"\\.\pipe\") {
        return Err(format!(
            "windows endpoint must start with \\\\.\\pipe\\ ('{endpoint}')"
        ));
    }
    Ok(())
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
