use std::sync::Arc;

use crate::{ClientId, RuntimeId, SessionTerminalHandle};
use terminal_emulator::{Alert, ClipboardSelection};

#[derive(Clone, Debug)]
pub enum HostRuntimeNotification {
    PaneOutput(SessionTerminalHandle),
    PaneAdded(SessionTerminalHandle),
    PaneRemoved(SessionTerminalHandle),
    WindowInvalidated,
    WindowWorkspaceChanged,
    ActiveWorkspaceChanged(Arc<ClientId>),
    Alert {
        pane_id: SessionTerminalHandle,
        alert: Alert,
    },
    Empty,
    AssignClipboard {
        pane_id: SessionTerminalHandle,
        selection: ClipboardSelection,
        clipboard: Option<String>,
    },
    SaveToDownloads {
        name: Option<String>,
        data: Arc<Vec<u8>>,
    },
    TabAddedToWindow {
        runtime_id: RuntimeId,
    },
    PaneFocused(SessionTerminalHandle),
    TabResized(RuntimeId),
    TabTitleChanged {
        runtime_id: RuntimeId,
        title: String,
    },
    WindowTitleChanged {
        title: String,
    },
    WorkspaceRenamed {
        old_workspace: String,
        new_workspace: String,
    },
}
