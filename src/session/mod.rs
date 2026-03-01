pub mod grid;
pub mod manager;
pub mod pty_worker;

#[cfg(test)]
mod tests;

pub use grid::{Cell, CellAttrs, CellColor, CursorStyle, SessionId, TerminalGrid};
pub use manager::SessionManager;
pub use pty_worker::SessionEvent;

pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub input_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    pub reader_handle: Option<std::thread::JoinHandle<()>>,
    pub writer_handle: Option<std::thread::JoinHandle<()>>,
}
