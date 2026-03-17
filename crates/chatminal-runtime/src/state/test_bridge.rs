// Test-only execution bridge using portable-pty directly.
// Avoids the session_engine dev-dependency diamond problem.
// Used by both state/tests.rs and server.rs integration tests.

use std::io::{Read, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};

use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

use crate::api::{RuntimeSessionBridgeAction, RuntimeSessionLookup};
use crate::session::{InputWriteStats, SessionEvent, WriteInputError};
use crate::state::runtime_bridge::{RuntimeExecutionAdapter, RuntimeHandle, RuntimeSessionHandleTrait};
use crate::workspace_ids::{RuntimeId, TerminalInstanceId};
use crate::workspace_layout::WorkspaceLayoutRegistry;
use crate::DaemonState;

// ─── TestSessionHandle ───────────────────────────────────────────────────────
// Wraps master in Mutex to satisfy Sync bound on RuntimeSessionHandleTrait.

struct TestSessionHandle {
    master: Mutex<Box<dyn MasterPty + Send>>,
    size: (usize, usize),
    closed: bool,
}

impl std::fmt::Debug for TestSessionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestSessionHandle")
            .field("size", &self.size)
            .field("closed", &self.closed)
            .finish()
    }
}

impl RuntimeSessionHandleTrait for TestSessionHandle {
    fn write_input(&self, data: &str) -> Result<InputWriteStats, WriteInputError> {
        if self.closed {
            return Err(WriteInputError::Closing);
        }
        let master = self.master.lock().map_err(|_| WriteInputError::Disconnected)?;
        let mut writer = master
            .take_writer()
            .map_err(|_| WriteInputError::Disconnected)?;
        writer
            .write_all(data.as_bytes())
            .map_err(|_| WriteInputError::Disconnected)?;
        Ok(InputWriteStats::default())
    }

    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), String> {
        let master = self.master.lock().map_err(|e| format!("lock master: {e}"))?;
        master
            .resize(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("resize PTY: {e}"))?;
        self.size = (cols, rows);
        Ok(())
    }

    fn kill(&mut self) {
        self.closed = true;
    }

    fn size(&self) -> Result<(usize, usize), String> {
        if self.closed {
            return Err("session runtime is closed".to_string());
        }
        Ok(self.size)
    }
}

// ─── TestExecutionBridge ─────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TestExecutionBridge {
    workspace_layouts: Arc<Mutex<WorkspaceLayoutRegistry>>,
    events_tx: OnceLock<mpsc::SyncSender<SessionEvent>>,
}

impl TestExecutionBridge {
    pub fn new() -> Self {
        Self {
            workspace_layouts: Arc::new(Mutex::new(WorkspaceLayoutRegistry::default())),
            events_tx: OnceLock::new(),
        }
    }
}

impl RuntimeExecutionAdapter for TestExecutionBridge {
    fn connect_session_events(&self, events_tx: mpsc::SyncSender<SessionEvent>) {
        let _ = self.events_tx.set(events_tx);
    }

    fn spawn_handle(
        &self,
        session_id: &str,
        generation: u64,
        shell: &str,
        cwd: &str,
        cols: usize,
        rows: usize,
    ) -> Result<RuntimeHandle, String> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("openpty: {e}"))?;
        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(cwd);
        pair.slave
            .spawn_command(cmd)
            .map_err(|e| format!("spawn: {e}"))?;

        // Spawn output reader thread if events channel is connected.
        if let Some(tx) = self.events_tx.get().cloned() {
            let session_id = session_id.to_string();
            let mut reader = pair.master.try_clone_reader().map_err(|e| {
                format!("clone pty reader: {e}")
            })?;
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
                            let _ = tx.send(SessionEvent::Output {
                                session_id: session_id.clone(),
                                generation,
                                chunk,
                                ts: 0,
                            });
                        }
                    }
                }
                let _ = tx.send(SessionEvent::Exited {
                    session_id: session_id.clone(),
                    generation,
                    exit_code: Some(0),
                    reason: "eof".to_string(),
                });
            });
        }

        Ok(Arc::new(Mutex::new(TestSessionHandle {
            master: Mutex::new(pair.master),
            size: (cols, rows),
            closed: false,
        })))
    }

    fn workspace_layouts(&self) -> Arc<Mutex<WorkspaceLayoutRegistry>> {
        Arc::clone(&self.workspace_layouts)
    }

    fn attachment(&self, _session_id: &str) -> Option<(RuntimeId, TerminalInstanceId)> {
        None
    }

    fn reconcile_session_lookup(
        &self,
        host: &DaemonState,
        lookup: &RuntimeSessionLookup,
    ) -> Result<RuntimeSessionBridgeAction, String> {
        let workspace_active = host
            .workspace_load_passive()
            .ok()
            .and_then(|w| w.active_session_id);
        match (&workspace_active, &lookup.active_session_id) {
            (Some(runtime_active), Some(client_active)) if runtime_active != client_active => {
                Ok(RuntimeSessionBridgeAction::FocusSession {
                    session_id: runtime_active.clone(),
                })
            }
            _ => Ok(RuntimeSessionBridgeAction::Noop),
        }
    }

    fn notify_session_activated(
        &self,
        _host: &DaemonState,
        _session_id: &str,
        _runtime_id: RuntimeId,
    ) -> Result<(), String> {
        Ok(())
    }

    fn notify_session_closed(
        &self,
        _host: &DaemonState,
        _session_id: &str,
        _runtime_id: RuntimeId,
        _lookup_after_close: &RuntimeSessionLookup,
    ) -> Result<(), String> {
        Ok(())
    }
}

pub fn make_test_bridge() -> Arc<dyn RuntimeExecutionAdapter> {
    Arc::new(TestExecutionBridge::new()) as Arc<dyn RuntimeExecutionAdapter>
}
