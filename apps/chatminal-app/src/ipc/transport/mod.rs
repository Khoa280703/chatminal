use std::io::{Read, Write};
use std::time::Duration;

#[cfg(unix)]
mod unix;
#[cfg(all(not(unix), not(windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

pub trait ReadWriteStream: Read + Write + Send {
    fn try_clone_boxed(&self) -> std::io::Result<Box<dyn ReadWriteStream>>;

    fn set_write_timeout(&self, _duration: Option<Duration>) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
pub fn connect_local_stream(endpoint: &str) -> Result<Box<dyn ReadWriteStream>, String> {
    unix::connect_local_stream(endpoint)
}

#[cfg(windows)]
pub fn connect_local_stream(endpoint: &str) -> Result<Box<dyn ReadWriteStream>, String> {
    windows::connect_local_stream(endpoint)
}

#[cfg(all(not(unix), not(windows)))]
pub fn connect_local_stream(endpoint: &str) -> Result<Box<dyn ReadWriteStream>, String> {
    unsupported::connect_local_stream(endpoint)
}
