use std::io::{Read, Write};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Output {
        session_id: String,
        generation: u64,
        chunk: String,
        ts: u64,
    },
    Exited {
        session_id: String,
        generation: u64,
        exit_code: Option<i32>,
        reason: String,
    },
    Error {
        session_id: String,
        generation: u64,
        message: String,
    },
}

pub struct SessionRuntime {
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    input_tx: Option<std_mpsc::SyncSender<Vec<u8>>>,
    reader_handle: Option<thread::JoinHandle<()>>,
    writer_handle: Option<thread::JoinHandle<()>>,
    waiter_handle: Option<thread::JoinHandle<()>>,
}

impl SessionRuntime {
    pub fn spawn(
        session_id: String,
        generation: u64,
        shell: String,
        cwd: String,
        cols: usize,
        rows: usize,
        events: std_mpsc::SyncSender<SessionEvent>,
    ) -> Result<Self, String> {
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: rows.clamp(2, u16::MAX as usize) as u16,
                cols: cols.clamp(2, u16::MAX as usize) as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("open pty failed: {err}"))?;

        let mut command = CommandBuilder::new(shell.clone());
        command.cwd(cwd);
        command.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| format!("spawn command failed: {err}"))?;
        let child = Arc::new(Mutex::new(child));

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| format!("clone reader failed: {err}"))?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|err| format!("take writer failed: {err}"))?;

        let (input_tx, input_rx) = std_mpsc::sync_channel::<Vec<u8>>(256);

        let reader_session = session_id.clone();
        let reader_events = events.clone();
        let reader_handle = thread::spawn(move || {
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        let chunk = String::from_utf8_lossy(&buffer[..read]).to_string();
                        let _ = reader_events.send(SessionEvent::Output {
                            session_id: reader_session.clone(),
                            generation,
                            chunk,
                            ts: now_millis(),
                        });
                    }
                    Err(err) => {
                        let _ = reader_events.send(SessionEvent::Error {
                            session_id: reader_session.clone(),
                            generation,
                            message: format!("pty read failed: {err}"),
                        });
                        break;
                    }
                }
            }
        });

        let writer_session = session_id.clone();
        let writer_events = events.clone();
        let writer_handle = thread::spawn(move || {
            while let Ok(chunk) = input_rx.recv() {
                if let Err(err) = writer.write_all(&chunk) {
                    let _ = writer_events.send(SessionEvent::Error {
                        session_id: writer_session.clone(),
                        generation,
                        message: format!("pty write failed: {err}"),
                    });
                    break;
                }
                let _ = writer.flush();
            }
        });

        let waiter_session = session_id;
        let waiter_events = events;
        let waiter_child = child.clone();
        let waiter_handle = thread::spawn(move || {
            loop {
                let polled = waiter_child
                    .lock()
                    .ok()
                    .and_then(|mut guard| guard.try_wait().ok())
                    .flatten();

                if let Some(status) = polled {
                    let _ = waiter_events.send(SessionEvent::Exited {
                        session_id: waiter_session,
                        generation,
                        exit_code: Some(status.exit_code() as i32),
                        reason: "eof".to_string(),
                    });
                    break;
                }

                thread::sleep(Duration::from_millis(120));
            }
        });

        Ok(Self {
            master: pair.master,
            child,
            input_tx: Some(input_tx),
            reader_handle: Some(reader_handle),
            writer_handle: Some(writer_handle),
            waiter_handle: Some(waiter_handle),
        })
    }

    pub fn write_input(&self, data: &str) -> Result<(), String> {
        let Some(tx) = self.input_tx.as_ref() else {
            return Err("session runtime is closing".to_string());
        };
        tx.try_send(data.as_bytes().to_vec())
            .map_err(|err| format!("queue input failed: {err}"))
    }

    pub fn resize(&self, cols: usize, rows: usize) -> Result<(), String> {
        self.master
            .resize(PtySize {
                rows: rows.clamp(2, u16::MAX as usize) as u16,
                cols: cols.clamp(2, u16::MAX as usize) as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("resize pty failed: {err}"))
    }

    pub fn kill(&mut self) {
        self.input_tx.take();
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
        self.reader_handle.take();
        self.writer_handle.take();
        self.waiter_handle.take();
    }
}

impl Drop for SessionRuntime {
    fn drop(&mut self) {
        self.kill();
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}
