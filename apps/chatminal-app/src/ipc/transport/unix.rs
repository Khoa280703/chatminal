use std::os::unix::net::UnixStream;
use std::time::Duration;

use super::ReadWriteStream;

impl ReadWriteStream for UnixStream {
    fn try_clone_boxed(&self) -> std::io::Result<Box<dyn ReadWriteStream>> {
        self.try_clone()
            .map(|value| Box::new(value) as Box<dyn ReadWriteStream>)
    }

    fn set_write_timeout(&self, duration: Option<Duration>) -> std::io::Result<()> {
        UnixStream::set_write_timeout(self, duration)
    }
}

pub(super) fn connect_local_stream(endpoint: &str) -> Result<Box<dyn ReadWriteStream>, String> {
    let stream = UnixStream::connect(endpoint)
        .map_err(|err| format!("connect unix socket failed ('{endpoint}'): {err}"))?;
    Ok(Box::new(stream))
}
