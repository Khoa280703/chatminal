use std::io::{Read, Write};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

pub trait ReadWriteStream: Read + Write + Send {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()>;
    fn set_write_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()>;
    fn try_clone_boxed(&self) -> std::io::Result<Box<dyn ReadWriteStream>>;
}

#[cfg(unix)]
impl ReadWriteStream for UnixStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        UnixStream::set_read_timeout(self, timeout)
    }

    fn set_write_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        UnixStream::set_write_timeout(self, timeout)
    }

    fn try_clone_boxed(&self) -> std::io::Result<Box<dyn ReadWriteStream>> {
        self.try_clone()
            .map(|value| Box::new(value) as Box<dyn ReadWriteStream>)
    }
}

#[cfg(unix)]
pub fn connect_local_stream(endpoint: &str) -> Result<Box<dyn ReadWriteStream>, String> {
    let stream = UnixStream::connect(endpoint)
        .map_err(|err| format!("connect unix socket failed ('{endpoint}'): {err}"))?;
    Ok(Box::new(stream))
}

#[cfg(not(unix))]
pub fn connect_local_stream(_endpoint: &str) -> Result<Box<dyn ReadWriteStream>, String> {
    Err("chatminal-app scaffold currently supports unix platforms only".to_string())
}
