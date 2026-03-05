use std::fs::File;
use std::io::{Read, Write};
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, ERROR_SEM_TIMEOUT, GetLastError,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Pipes::{
    PIPE_NOWAIT, PIPE_READMODE_BYTE, SetNamedPipeHandleState, WaitNamedPipeW,
};

use super::ReadWriteStream;

const PIPE_CONNECT_TIMEOUT_MS: u32 = 3_000;
const PIPE_CONNECT_RETRY_SLICE_MS: u32 = 150;

struct NamedPipeStream {
    file: File,
}

impl ReadWriteStream for NamedPipeStream {
    fn try_clone_boxed(&self) -> std::io::Result<Box<dyn ReadWriteStream>> {
        self.file
            .try_clone()
            .map(|file| Box::new(Self { file }) as Box<dyn ReadWriteStream>)
    }
}

impl Read for NamedPipeStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Write for NamedPipeStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

pub(super) fn connect_local_stream(endpoint: &str) -> Result<Box<dyn ReadWriteStream>, String> {
    validate_named_pipe_endpoint(endpoint)?;
    let endpoint_wide = to_wide(endpoint);
    let deadline = Instant::now() + Duration::from_millis(PIPE_CONNECT_TIMEOUT_MS as u64);
    let handle = loop {
        let handle = unsafe {
            CreateFileW(
                endpoint_wide.as_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            break handle;
        }

        let code = unsafe { GetLastError() };
        let can_retry = matches!(
            code,
            ERROR_FILE_NOT_FOUND | ERROR_PIPE_BUSY | ERROR_SEM_TIMEOUT
        ) && Instant::now() < deadline;
        if !can_retry {
            return Err(format!(
                "connect named pipe failed ('{endpoint}'): {}",
                std::io::Error::from_raw_os_error(code as i32)
            ));
        }

        let remaining_ms = deadline
            .saturating_duration_since(Instant::now())
            .as_millis() as u32;
        let wait_ms = remaining_ms.min(PIPE_CONNECT_RETRY_SLICE_MS).max(1);
        let waited = unsafe { WaitNamedPipeW(endpoint_wide.as_ptr(), wait_ms) };
        if waited == 0 {
            let wait_code = unsafe { GetLastError() };
            if !matches!(
                wait_code,
                ERROR_FILE_NOT_FOUND | ERROR_PIPE_BUSY | ERROR_SEM_TIMEOUT
            ) {
                return Err(format!(
                    "wait named pipe failed ('{endpoint}'): {}",
                    std::io::Error::from_raw_os_error(wait_code as i32)
                ));
            }
        }
    };

    let read_mode = PIPE_READMODE_BYTE | PIPE_NOWAIT;
    let mode_set = unsafe {
        SetNamedPipeHandleState(
            handle,
            &read_mode,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if mode_set == 0 {
        let err = std::io::Error::last_os_error();
        let _ = unsafe { CloseHandle(handle) };
        return Err(format!(
            "set named pipe read mode failed ('{endpoint}'): {err}"
        ));
    }

    let file = unsafe { File::from_raw_handle(handle as RawHandle) };
    Ok(Box::new(NamedPipeStream { file }))
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
