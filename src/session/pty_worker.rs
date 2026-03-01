use std::io::{Read, Write};
use std::sync::Arc;

use tokio::sync::mpsc;

use super::{Cell, CellAttrs, CellColor, SessionId, TerminalGrid};

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Update {
        session_id: SessionId,
        grid: Arc<TerminalGrid>,
        lines_added: usize,
    },
    Exited(SessionId),
}

pub fn pty_reader_thread(
    mut reader: Box<dyn Read + Send>,
    event_tx: mpsc::Sender<SessionEvent>,
    session_id: SessionId,
    cols: usize,
    rows: usize,
    scrollback_max_lines: usize,
) {
    let mut parser = vte::Parser::new();
    let mut performer = PtyPerformer::new(session_id, event_tx, cols, rows, scrollback_max_lines);
    let mut buf = [0u8; 4096];

    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                let _ = performer
                    .event_tx
                    .blocking_send(SessionEvent::Exited(session_id));
                break;
            }
            Ok(n) => {
                parser.advance(&mut performer, &buf[..n]);
                performer.flush_update();
            }
            Err(err) => {
                log::info!("PTY reader exited for {session_id}: {err}");
                let _ = performer
                    .event_tx
                    .blocking_send(SessionEvent::Exited(session_id));
                break;
            }
        }
    }
}

pub fn pty_writer_thread(mut writer: Box<dyn Write + Send>, mut input_rx: mpsc::Receiver<Vec<u8>>) {
    while let Some(bytes) = input_rx.blocking_recv() {
        if writer.write_all(&bytes).is_err() {
            break;
        }

        if writer.flush().is_err() {
            break;
        }
    }
}

struct PtyPerformer {
    session_id: SessionId,
    event_tx: mpsc::Sender<SessionEvent>,
    grid: TerminalGrid,
    fg: CellColor,
    bg: CellColor,
    attrs: CellAttrs,
    saved_cursor: Option<(usize, usize)>,
    dirty: bool,
    lines_added: usize,
}

impl PtyPerformer {
    fn new(
        session_id: SessionId,
        event_tx: mpsc::Sender<SessionEvent>,
        cols: usize,
        rows: usize,
        scrollback_max_lines: usize,
    ) -> Self {
        Self {
            session_id,
            event_tx,
            grid: TerminalGrid::new(cols, rows, scrollback_max_lines),
            fg: CellColor::Default,
            bg: CellColor::Default,
            attrs: CellAttrs::default(),
            saved_cursor: None,
            dirty: true,
            lines_added: 0,
        }
    }

    fn flush_update(&mut self) {
        if !self.dirty {
            return;
        }

        match self.event_tx.try_send(SessionEvent::Update {
            session_id: self.session_id,
            grid: Arc::new(self.grid.clone()),
            lines_added: self.lines_added,
        }) {
            Ok(_) => {
                self.lines_added = 0;
                self.dirty = false;
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                log::warn!(
                    "PTY update queue full for session {}, will retry latest snapshot",
                    self.session_id
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.lines_added = 0;
                self.dirty = false;
            }
        }
    }

    fn put_char(&mut self, c: char) {
        let row = self.grid.cursor_row;
        let col = self.grid.cursor_col;

        self.grid.set_cell(
            row,
            col,
            Cell {
                c,
                fg: self.fg,
                bg: self.bg,
                attrs: self.attrs,
            },
        );

        self.grid.cursor_col += 1;
        if self.grid.cursor_col >= self.grid.cols {
            self.grid.cursor_col = 0;
            self.newline();
        }
        self.dirty = true;
    }

    fn newline(&mut self) {
        self.grid.cursor_row += 1;
        if self.grid.cursor_row >= self.grid.rows {
            self.grid.cursor_row = self.grid.rows.saturating_sub(1);
            self.lines_added += self.grid.scroll_up(1);
        }
        self.dirty = true;
    }

    fn clamp_cursor(&mut self) {
        self.grid.cursor_row = self.grid.cursor_row.min(self.grid.rows.saturating_sub(1));
        self.grid.cursor_col = self.grid.cursor_col.min(self.grid.cols.saturating_sub(1));
    }

    fn reset_style(&mut self) {
        self.fg = CellColor::Default;
        self.bg = CellColor::Default;
        self.attrs = CellAttrs::default();
    }

    fn first_param(params: &vte::Params, default: u16) -> u16 {
        params
            .iter()
            .next()
            .and_then(|sub| sub.first().copied())
            .unwrap_or(default)
    }

    fn handle_sgr(&mut self, params: &vte::Params) {
        let flat: Vec<u16> = params.iter().flat_map(|s| s.iter().copied()).collect();

        if flat.is_empty() {
            self.reset_style();
            return;
        }

        let mut i = 0;
        while i < flat.len() {
            match flat[i] {
                0 => self.reset_style(),
                1 => self.attrs.bold = true,
                3 => self.attrs.italic = true,
                4 => self.attrs.underline = true,
                22 => self.attrs.bold = false,
                23 => self.attrs.italic = false,
                24 => self.attrs.underline = false,
                30..=37 => self.fg = CellColor::Indexed((flat[i] - 30) as u8),
                39 => self.fg = CellColor::Default,
                40..=47 => self.bg = CellColor::Indexed((flat[i] - 40) as u8),
                49 => self.bg = CellColor::Default,
                90..=97 => self.fg = CellColor::Indexed((flat[i] - 90 + 8) as u8),
                100..=107 => self.bg = CellColor::Indexed((flat[i] - 100 + 8) as u8),
                38 if i + 2 < flat.len() && flat[i + 1] == 5 => {
                    self.fg = CellColor::Indexed(flat[i + 2] as u8);
                    i += 2;
                }
                48 if i + 2 < flat.len() && flat[i + 1] == 5 => {
                    self.bg = CellColor::Indexed(flat[i + 2] as u8);
                    i += 2;
                }
                38 if i + 4 < flat.len() && flat[i + 1] == 2 => {
                    self.fg =
                        CellColor::Rgb(flat[i + 2] as u8, flat[i + 3] as u8, flat[i + 4] as u8);
                    i += 4;
                }
                48 if i + 4 < flat.len() && flat[i + 1] == 2 => {
                    self.bg =
                        CellColor::Rgb(flat[i + 2] as u8, flat[i + 3] as u8, flat[i + 4] as u8);
                    i += 4;
                }
                _ => {}
            }

            i += 1;
        }

        self.dirty = true;
    }

    fn clear_to_end_of_line(&mut self) {
        let row = self.grid.cursor_row;
        let start = self.grid.cursor_col;
        for col in start..self.grid.cols {
            self.grid.set_cell(row, col, Cell::default());
        }
    }

    fn clear_to_start_of_line(&mut self) {
        let row = self.grid.cursor_row;
        let end = self.grid.cursor_col;
        for col in 0..=end.min(self.grid.cols.saturating_sub(1)) {
            self.grid.set_cell(row, col, Cell::default());
        }
    }
}

impl vte::Perform for PtyPerformer {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            b'\r' => {
                self.grid.cursor_col = 0;
                self.dirty = true;
            }
            0x08 => {
                self.grid.cursor_col = self.grid.cursor_col.saturating_sub(1);
                self.dirty = true;
            }
            b'\t' => {
                let next_tab = ((self.grid.cursor_col / 8) + 1) * 8;
                self.grid.cursor_col = next_tab.min(self.grid.cols.saturating_sub(1));
                self.dirty = true;
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let p0 = Self::first_param(params, 0);

        if intermediates.contains(&b'?') && p0 == 1049 {
            match action {
                'h' => {
                    self.saved_cursor = Some((self.grid.cursor_row, self.grid.cursor_col));
                    self.grid.switch_alternate(true);
                    self.grid.clear_screen();
                    self.dirty = true;
                }
                'l' => {
                    self.grid.switch_alternate(false);
                    if let Some((row, col)) = self.saved_cursor.take() {
                        self.grid.cursor_row = row;
                        self.grid.cursor_col = col;
                        self.clamp_cursor();
                    }
                    self.dirty = true;
                }
                _ => {}
            }
            return;
        }

        match action {
            'm' => self.handle_sgr(params),
            'A' => {
                let n = Self::first_param(params, 1) as usize;
                self.grid.cursor_row = self.grid.cursor_row.saturating_sub(n);
                self.dirty = true;
            }
            'B' => {
                let n = Self::first_param(params, 1) as usize;
                self.grid.cursor_row =
                    (self.grid.cursor_row + n).min(self.grid.rows.saturating_sub(1));
                self.dirty = true;
            }
            'C' => {
                let n = Self::first_param(params, 1) as usize;
                self.grid.cursor_col =
                    (self.grid.cursor_col + n).min(self.grid.cols.saturating_sub(1));
                self.dirty = true;
            }
            'D' => {
                let n = Self::first_param(params, 1) as usize;
                self.grid.cursor_col = self.grid.cursor_col.saturating_sub(n);
                self.dirty = true;
            }
            'H' | 'f' => {
                let row = params
                    .iter()
                    .next()
                    .and_then(|sub| sub.first().copied())
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                let col = params
                    .iter()
                    .nth(1)
                    .and_then(|sub| sub.first().copied())
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                self.grid.cursor_row = row;
                self.grid.cursor_col = col;
                self.clamp_cursor();
                self.dirty = true;
            }
            'J' => {
                match p0 {
                    0 => {
                        self.clear_to_end_of_line();
                        for row in (self.grid.cursor_row + 1)..self.grid.rows {
                            self.grid.clear_row(row);
                        }
                    }
                    1 => {
                        self.clear_to_start_of_line();
                        for row in 0..self.grid.cursor_row {
                            self.grid.clear_row(row);
                        }
                    }
                    2 => self.grid.clear_screen(),
                    _ => {}
                }
                self.dirty = true;
            }
            'K' => {
                match p0 {
                    0 => self.clear_to_end_of_line(),
                    1 => self.clear_to_start_of_line(),
                    2 => self.grid.clear_row(self.grid.cursor_row),
                    _ => {}
                }
                self.dirty = true;
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'c' => {
                self.grid.clear_screen();
                self.reset_style();
                self.dirty = true;
            }
            b'7' => {
                self.saved_cursor = Some((self.grid.cursor_row, self.grid.cursor_col));
            }
            b'8' => {
                if let Some((row, col)) = self.saved_cursor {
                    self.grid.cursor_row = row;
                    self.grid.cursor_col = col;
                    self.clamp_cursor();
                    self.dirty = true;
                }
            }
            b'M' => {
                if self.grid.cursor_row == 0 {
                    self.grid.scroll_down(1);
                } else {
                    self.grid.cursor_row = self.grid.cursor_row.saturating_sub(1);
                }
                self.dirty = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::{PtyPerformer, SessionEvent, SessionId};

    #[test]
    fn reverse_index_esc_m_scrolls_down_from_top_row() {
        let session_id = SessionId::new_v4();
        let (tx, _rx) = mpsc::channel(8);
        let mut parser = vte::Parser::new();
        let mut performer = PtyPerformer::new(session_id, tx, 4, 2, 100);

        parser.advance(&mut performer, b"A\r\nB");
        performer.grid.cursor_row = 0;
        performer.grid.cursor_col = 0;

        parser.advance(&mut performer, b"\x1bM");

        assert_eq!(performer.grid.active_cells()[0][0].c, ' ');
        assert_eq!(performer.grid.active_cells()[1][0].c, 'A');
    }

    #[test]
    fn flush_update_retries_after_queue_full() {
        let session_id = SessionId::new_v4();
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(SessionEvent::Exited(session_id))
            .expect("channel must be fillable");

        let mut performer = PtyPerformer::new(session_id, tx, 4, 2, 100);
        performer.put_char('X');
        performer.flush_update();

        assert!(performer.dirty);
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Exited(_))));

        performer.flush_update();
        assert!(!performer.dirty);
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Update { .. })));
    }
}
