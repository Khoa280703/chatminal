use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use iced::futures::SinkExt;
use iced::keyboard::{self, Key};
use iced::widget::{Space, container, row};
use iced::{Element, Length, Subscription, Task, event, window};
use tokio::sync::mpsc;

use crate::config::{Config, SCROLLBACK_DEFAULT_LINES, load_config};
use crate::message::Message;
use crate::session::{SessionEvent, SessionId, SessionManager, TerminalGrid};
use crate::ui::input_handler::key_to_bytes;
use crate::ui::sidebar::sidebar_view;
use crate::ui::terminal_pane::terminal_pane_view;
use crate::ui::theme::{DEFAULT_FONT_SIZE, SIDEBAR_WIDTH, metrics_for_font};

static SESSION_EVENT_RX: OnceLock<Mutex<Option<mpsc::Receiver<SessionEvent>>>> = OnceLock::new();

pub struct AppState {
    config: Config,
    pub session_manager: SessionManager,
    pub active_session_id: Option<SessionId>,
    pub session_grids: HashMap<SessionId, Arc<TerminalGrid>>,
    pub scroll_offsets: HashMap<SessionId, usize>,
    pub next_session_num: usize,
    pub current_cols: usize,
    pub current_rows: usize,
    pub cell_width: f32,
    pub cell_height: f32,
    pub font_size: f32,
    pub terminal_generation: u64,
}

impl AppState {
    pub fn boot() -> (Self, Task<Message>) {
        let config = load_config();
        let font_size = config.font_size.unwrap_or(DEFAULT_FONT_SIZE);
        let (cell_width, cell_height) = metrics_for_font(font_size);
        let (event_tx, event_rx) = mpsc::channel(64);

        if let Some(lock) = SESSION_EVENT_RX.get() {
            if let Ok(mut guard) = lock.lock() {
                *guard = Some(event_rx);
            }
        } else {
            let _ = SESSION_EVENT_RX.set(Mutex::new(Some(event_rx)));
        }

        let mut state = Self {
            cell_width,
            cell_height,
            font_size,
            current_cols: 80,
            current_rows: 24,
            session_manager: SessionManager::new(
                event_tx,
                config.shell.clone(),
                config.scrollback_lines.unwrap_or(SCROLLBACK_DEFAULT_LINES),
            ),
            config,
            active_session_id: None,
            session_grids: HashMap::new(),
            scroll_offsets: HashMap::new(),
            next_session_num: 1,
            terminal_generation: 0,
        };

        state.create_new_session();

        (state, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NewSession => {
                self.create_new_session();
                Task::none()
            }
            Message::SelectSession(id) => {
                if self.session_manager.contains(id) {
                    self.active_session_id = Some(id);
                    self.scroll_offsets.insert(id, 0);
                    self.terminal_generation += 1;
                }
                Task::none()
            }
            Message::CloseSession(id) => {
                let session_ids_before = self.session_manager.session_ids();
                let removed_index = session_ids_before.iter().position(|sid| *sid == id);

                self.session_manager.close_session(id);
                self.session_grids.remove(&id);
                self.scroll_offsets.remove(&id);

                if self.active_session_id == Some(id) {
                    let session_ids_after = self.session_manager.session_ids();
                    self.active_session_id = removed_index.and_then(|index| {
                        if session_ids_after.is_empty() {
                            None
                        } else if index > 0 {
                            session_ids_after.get(index - 1).copied()
                        } else {
                            session_ids_after.first().copied()
                        }
                    });
                }

                self.terminal_generation += 1;
                Task::none()
            }
            Message::SessionExited(id) => Task::done(Message::CloseSession(id)),
            Message::TerminalUpdated {
                session_id,
                grid,
                lines_added,
            } => {
                if self.session_manager.contains(session_id) {
                    let offset = self.scroll_offsets.entry(session_id).or_insert(0);
                    if grid.use_alternate {
                        *offset = 0;
                    } else if *offset > 0 {
                        *offset = (*offset + lines_added).min(grid.scrollback.len());
                    }

                    self.session_grids.insert(session_id, grid);
                    self.terminal_generation += 1;
                }
                Task::none()
            }
            Message::KeyboardEvent(event) => self.handle_event(event),
            Message::WindowResized(width, height) => {
                self.handle_resize(width, height);
                Task::none()
            }
            Message::ScrollTerminal { delta } => {
                if let Some(active) = self.active_session_id
                    && let Some(grid) = self.session_grids.get(&active)
                {
                    let offset = self.scroll_offsets.entry(active).or_insert(0);
                    let max = if grid.use_alternate {
                        0
                    } else {
                        grid.scrollback.len() as i32
                    };
                    *offset = (*offset as i32 + delta).clamp(0, max) as usize;
                    self.terminal_generation += 1;
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sessions = self.session_manager.list_sessions();
        let sidebar = sidebar_view(
            &sessions,
            self.active_session_id,
            self.config.sidebar_width.unwrap_or(SIDEBAR_WIDTH),
        );

        let active_grid = self
            .active_session_id
            .and_then(|id| self.session_grids.get(&id).cloned());
        let scroll_offset = self
            .active_session_id
            .and_then(|id| self.scroll_offsets.get(&id).copied())
            .unwrap_or(0);

        let terminal = terminal_pane_view(
            active_grid,
            scroll_offset,
            self.cell_width,
            self.cell_height,
            self.font_size,
            self.terminal_generation,
        );

        row![
            sidebar,
            container(Space::new().width(Length::Fixed(1.0))).height(Length::Fill),
            terminal,
        ]
        .height(Length::Fill)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let pty_sub = Subscription::run(pty_event_stream);
        let event_sub = event::listen().map(Message::KeyboardEvent);
        Subscription::batch([pty_sub, event_sub])
    }

    fn create_new_session(&mut self) {
        let name = format!("Session {}", self.next_session_num);
        self.next_session_num += 1;

        match self
            .session_manager
            .create_session(name, self.current_cols, self.current_rows)
        {
            Ok(id) => {
                self.active_session_id = Some(id);
                self.scroll_offsets.insert(id, 0);
                self.terminal_generation += 1;
            }
            Err(err) => {
                log::error!("Cannot create session: {err:?}");
            }
        }
    }

    fn handle_event(&mut self, ev: iced::Event) -> Task<Message> {
        match ev {
            iced::Event::Window(window::Event::Resized(size)) => Task::done(
                Message::WindowResized(size.width as u32, size.height as u32),
            ),
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                ..
            }) => {
                if modifiers.alt() {
                    match key.as_ref() {
                        Key::Character("n") | Key::Character("N") => {
                            return Task::done(Message::NewSession);
                        }
                        Key::Character("w") | Key::Character("W") => {
                            if let Some(id) = self.active_session_id {
                                return Task::done(Message::CloseSession(id));
                            }
                        }
                        _ => {}
                    }
                }

                if modifiers.shift() {
                    match key.as_ref() {
                        Key::Named(keyboard::key::Named::PageUp) => {
                            return Task::done(Message::ScrollTerminal {
                                delta: self.current_rows as i32,
                            });
                        }
                        Key::Named(keyboard::key::Named::PageDown) => {
                            return Task::done(Message::ScrollTerminal {
                                delta: -(self.current_rows as i32),
                            });
                        }
                        _ => {}
                    }
                }

                if let Some(active_id) = self.active_session_id {
                    let bytes = key_to_bytes(&key, physical_key, modifiers);
                    if !bytes.is_empty()
                        && let Err(err) = self.session_manager.send_input(active_id, bytes)
                    {
                        log::warn!("Failed to send PTY input: {err}");
                    }
                }

                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn handle_resize(&mut self, width: u32, height: u32) {
        let sidebar = self.config.sidebar_width.unwrap_or(SIDEBAR_WIDTH);
        let cols =
            (((width as f32 - sidebar).max(self.cell_width)) / self.cell_width).floor() as usize;
        let rows = ((height as f32).max(self.cell_height) / self.cell_height).floor() as usize;

        self.current_cols = cols.max(10);
        self.current_rows = rows.max(5);

        self.session_manager
            .resize_all_sessions(self.current_cols, self.current_rows);
    }
}

fn pty_event_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(64, async |mut output| {
        let receiver = SESSION_EVENT_RX
            .get()
            .and_then(|lock| lock.lock().ok().and_then(|mut guard| guard.take()));

        let Some(mut rx) = receiver else {
            return;
        };

        while let Some(event) = rx.recv().await {
            let message = match event {
                SessionEvent::Update {
                    session_id,
                    grid,
                    lines_added,
                } => Message::TerminalUpdated {
                    session_id,
                    grid,
                    lines_added,
                },
                SessionEvent::Exited(id) => Message::SessionExited(id),
            };

            let _ = output.send(message).await;
        }
    })
}
