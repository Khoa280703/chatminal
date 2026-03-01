use std::sync::Arc;

use crate::session::{SessionId, TerminalGrid};

#[derive(Debug, Clone)]
pub enum Message {
    TerminalUpdated {
        session_id: SessionId,
        grid: Arc<TerminalGrid>,
        lines_added: usize,
    },
    SessionExited(SessionId),
    SelectSession(SessionId),
    NewSession,
    CloseSession(SessionId),
    KeyboardEvent(iced::Event),
    WindowResized(u32, u32),
    ScrollTerminal {
        delta: i32,
    },
}
