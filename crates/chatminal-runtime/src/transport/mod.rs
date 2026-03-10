#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub(crate) trait TransportListener {
    type Stream;
    fn accept_stream(&self) -> Result<Option<Self::Stream>, String>;
}

pub(crate) trait TransportBackend {
    type Listener: TransportListener;
    fn bind(endpoint: &str) -> Result<Self::Listener, String>;
    fn cleanup(endpoint: &str);
}

#[cfg(unix)]
pub(crate) use unix::{LocalStream, UnixTransport as ActiveTransport};
#[cfg(windows)]
pub(crate) use windows::{LocalStream, WindowsTransport as ActiveTransport};

#[cfg(all(unix, test))]
pub(crate) use unix::ensure_socket_path;
