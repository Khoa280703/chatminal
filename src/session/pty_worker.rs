use std::io::{Read, Write};
use std::sync::Arc;

use tokio::sync::mpsc;
use wezterm_surface::{CursorShape, CursorVisibility};
use wezterm_term::color::{ColorAttribute, ColorPalette, SrgbaTuple};
use wezterm_term::{
    CellAttributes, Intensity, Line, Terminal, TerminalConfiguration, TerminalSize, Underline,
};

use super::{
    Cell as GridCell, CellAttrs as GridCellAttrs, CellColor as GridCellColor, CursorStyle,
    SessionId, TerminalGrid,
};

const DEFAULT_DPI: u32 = 96;
const TERM_PROGRAM: &str = "chatminal";
const TERM_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Update {
        session_id: SessionId,
        grid: Arc<TerminalGrid>,
        lines_added: usize,
    },
    Exited(SessionId),
}

#[derive(Debug)]
struct EmbeddedTerminalConfig {
    scrollback_size: usize,
}

impl TerminalConfiguration for EmbeddedTerminalConfig {
    fn scrollback_size(&self) -> usize {
        self.scrollback_size
    }

    fn color_palette(&self) -> ColorPalette {
        ColorPalette::default()
    }
}

struct PtyEngine {
    session_id: SessionId,
    event_tx: mpsc::Sender<SessionEvent>,
    terminal: Terminal,
    scrollback_max_lines: usize,
    last_top_stable_row: Option<isize>,
    dirty: bool,
}

impl PtyEngine {
    fn new(
        session_id: SessionId,
        event_tx: mpsc::Sender<SessionEvent>,
        cols: usize,
        rows: usize,
        scrollback_max_lines: usize,
    ) -> Self {
        let config = Arc::new(EmbeddedTerminalConfig {
            scrollback_size: scrollback_max_lines,
        });
        let terminal = Terminal::new(
            TerminalSize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
                dpi: DEFAULT_DPI,
            },
            config,
            TERM_PROGRAM,
            TERM_VERSION,
            Box::new(std::io::sink()),
        );

        Self {
            session_id,
            event_tx,
            terminal,
            scrollback_max_lines,
            last_top_stable_row: None,
            dirty: true,
        }
    }

    fn advance_bytes(&mut self, bytes: &[u8]) {
        self.terminal.advance_bytes(bytes);
        self.dirty = true;
    }

    fn flush_update(&mut self) {
        if !self.dirty {
            return;
        }

        let current_top_stable_row = self.current_top_stable_row();
        let lines_added = match (self.last_top_stable_row, current_top_stable_row) {
            (Some(previous), Some(current)) if current >= previous => (current - previous) as usize,
            _ => 0,
        };
        let snapshot = Arc::new(self.snapshot_grid());

        match self.event_tx.try_send(SessionEvent::Update {
            session_id: self.session_id,
            grid: snapshot,
            lines_added,
        }) {
            Ok(_) => {
                self.last_top_stable_row = current_top_stable_row;
                self.dirty = false;
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                log::warn!(
                    "PTY update queue full for session {}, will retry latest snapshot",
                    self.session_id
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.last_top_stable_row = current_top_stable_row;
                self.dirty = false;
            }
        }
    }

    fn current_top_stable_row(&self) -> Option<isize> {
        if self.terminal.is_alt_screen_active() {
            return None;
        }

        let screen = self.terminal.screen();
        Some(screen.visible_row_to_stable_row(0))
    }

    fn snapshot_grid(&self) -> TerminalGrid {
        let screen = self.terminal.screen();
        let rows = screen.physical_rows.max(1);
        let cols = screen.physical_cols.max(1);
        let total_rows = screen.scrollback_rows();
        let visible_start = total_rows.saturating_sub(rows);
        let is_alt_screen = self.terminal.is_alt_screen_active();
        let snapshot_start = if is_alt_screen {
            visible_start
        } else {
            visible_start.saturating_sub(self.scrollback_max_lines)
        };
        let lines = screen.lines_in_phys_range(snapshot_start..total_rows);
        let visible_offset = visible_start.saturating_sub(snapshot_start);
        let palette = self.terminal.palette();

        let mut grid = TerminalGrid::new(cols, rows, self.scrollback_max_lines);
        let mut visible_rows = vec![vec![GridCell::default(); cols]; rows];

        for (row_idx, line) in lines.iter().skip(visible_offset).take(rows).enumerate() {
            visible_rows[row_idx] = convert_line(line, cols, &palette);
        }

        if is_alt_screen {
            grid.use_alternate = true;
            grid.alternate_grid = visible_rows;
            grid.primary_grid = vec![vec![GridCell::default(); cols]; rows];
            grid.scrollback.clear();
        } else {
            grid.use_alternate = false;
            grid.primary_grid = visible_rows;
            for line in lines.iter().take(visible_offset) {
                if grid.scrollback.len() >= grid.scrollback_max_lines {
                    let _ = grid.scrollback.pop_front();
                }
                grid.scrollback
                    .push_back(convert_line(line, cols, &palette));
            }
        }

        let cursor = self.terminal.cursor_pos();
        grid.cursor_row = (cursor.y.max(0) as usize).min(rows.saturating_sub(1));
        grid.cursor_col = cursor.x.min(cols.saturating_sub(1));
        grid.cursor_style = map_cursor_style(cursor.shape, cursor.visibility);

        grid
    }
}

fn send_exited_event_async(event_tx: mpsc::Sender<SessionEvent>, session_id: SessionId) {
    let _ = std::thread::Builder::new()
        .name(format!("chatminal-exit-{session_id}"))
        .spawn(move || {
            let _ = event_tx.blocking_send(SessionEvent::Exited(session_id));
        });
}

fn convert_line(line: &Line, cols: usize, palette: &ColorPalette) -> Vec<GridCell> {
    let mut row = vec![GridCell::default(); cols];

    for cell_ref in line.visible_cells() {
        let col = cell_ref.cell_index();
        if col >= cols {
            continue;
        }

        let attrs = cell_ref.attrs();
        let fg = map_fg_color(attrs.foreground(), palette);
        let bg = map_bg_color(attrs.background(), palette);
        let cell_attrs = map_attrs(attrs);
        let ch = cell_ref.str().to_string();

        row[col] = GridCell {
            c: ch,
            fg,
            bg,
            attrs: cell_attrs,
        };

        let width = cell_ref.width().max(1);
        for offset in 1..width {
            let next_col = col + offset;
            if next_col >= cols {
                break;
            }
            row[next_col] = GridCell {
                c: String::new(),
                fg,
                bg,
                attrs: cell_attrs,
            };
        }
    }

    row
}

fn map_attrs(attrs: &CellAttributes) -> GridCellAttrs {
    GridCellAttrs {
        bold: matches!(attrs.intensity(), Intensity::Bold),
        italic: attrs.italic(),
        underline: !matches!(attrs.underline(), Underline::None),
    }
}

fn map_fg_color(color: ColorAttribute, palette: &ColorPalette) -> GridCellColor {
    match color {
        ColorAttribute::Default => GridCellColor::Default,
        other => srgba_to_cell_color(palette.resolve_fg(other)),
    }
}

fn map_bg_color(color: ColorAttribute, palette: &ColorPalette) -> GridCellColor {
    match color {
        ColorAttribute::Default => GridCellColor::Default,
        other => srgba_to_cell_color(palette.resolve_bg(other)),
    }
}

fn srgba_to_cell_color(color: SrgbaTuple) -> GridCellColor {
    GridCellColor::Rgb(
        channel_to_u8(color.0),
        channel_to_u8(color.1),
        channel_to_u8(color.2),
    )
}

fn channel_to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn map_cursor_style(shape: CursorShape, visibility: CursorVisibility) -> CursorStyle {
    if matches!(visibility, CursorVisibility::Hidden) {
        return CursorStyle::Hidden;
    }

    match shape {
        CursorShape::Default | CursorShape::BlinkingBlock | CursorShape::SteadyBlock => {
            CursorStyle::Block
        }
        CursorShape::BlinkingUnderline | CursorShape::SteadyUnderline => CursorStyle::Underline,
        CursorShape::BlinkingBar | CursorShape::SteadyBar => CursorStyle::Bar,
    }
}

pub fn pty_reader_thread(
    mut reader: Box<dyn Read + Send>,
    event_tx: mpsc::Sender<SessionEvent>,
    session_id: SessionId,
    cols: usize,
    rows: usize,
    scrollback_max_lines: usize,
) {
    let mut engine = PtyEngine::new(session_id, event_tx, cols, rows, scrollback_max_lines);
    let mut buf = [0u8; 4096];

    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                engine.flush_update();
                send_exited_event_async(engine.event_tx.clone(), session_id);
                break;
            }
            Ok(n) => {
                engine.advance_bytes(&buf[..n]);
                engine.flush_update();
            }
            Err(err) => {
                log::info!("PTY reader exited for {session_id}: {err}");
                engine.flush_update();
                send_exited_event_async(engine.event_tx.clone(), session_id);
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::session::CursorStyle;
    use tokio::sync::mpsc;

    use super::{PtyEngine, SessionEvent, SessionId, TerminalGrid, send_exited_event_async};

    #[test]
    fn csi_p_delete_sequence_is_parsed_by_wezterm_term() {
        let session_id = SessionId::new_v4();
        let (tx, _rx) = mpsc::channel(8);
        let mut engine = PtyEngine::new(session_id, tx, 8, 2, 100);

        engine.advance_bytes(b"abc\x1b[2D\x1b[P");
        let grid = engine.snapshot_grid();

        assert_eq!(grid.active_cells()[0][0].c, "a");
        assert_eq!(grid.active_cells()[0][1].c, "c");
        assert_eq!(grid.active_cells()[0][2].c, "");
    }

    #[test]
    fn reverse_index_sequence_is_handled_by_wezterm_term() {
        let session_id = SessionId::new_v4();
        let (tx, _rx) = mpsc::channel(8);
        let mut engine = PtyEngine::new(session_id, tx, 4, 2, 100);

        engine.advance_bytes(b"A\r\nB\x1b[H\x1bM");
        let grid = engine.snapshot_grid();

        assert_eq!(grid.active_cells()[0][0].c, "");
        assert_eq!(grid.active_cells()[1][0].c, "A");
    }

    #[test]
    fn flush_update_retries_after_queue_full() {
        let session_id = SessionId::new_v4();
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(SessionEvent::Exited(session_id))
            .expect("channel must be fillable");

        let mut engine = PtyEngine::new(session_id, tx, 4, 2, 100);
        engine.advance_bytes(b"X");
        engine.flush_update();

        assert!(engine.dirty);
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Exited(_))));

        engine.flush_update();
        assert!(!engine.dirty);
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Update { .. })));
    }

    #[test]
    fn lines_added_advances_even_when_scrollback_is_full() {
        let session_id = SessionId::new_v4();
        let (tx, mut rx) = mpsc::channel(16);
        let mut engine = PtyEngine::new(session_id, tx, 8, 2, 3);

        engine.advance_bytes(b"1\r\n2\r\n3\r\n4\r\n");
        engine.flush_update();
        let _ = rx.try_recv();

        let before = engine.snapshot_grid().scrollback.len();
        assert_eq!(before, 3);

        engine.advance_bytes(b"5\r\n6\r\n");
        engine.flush_update();

        match rx.try_recv() {
            Ok(SessionEvent::Update {
                lines_added, grid, ..
            }) => {
                assert!(lines_added > 0);
                assert_eq!(grid.scrollback.len(), 3);
            }
            other => panic!("expected update event, got {other:?}"),
        }
    }

    #[test]
    fn cursor_shape_sequences_are_mapped_from_wezterm() {
        let session_id = SessionId::new_v4();
        let (tx, _rx) = mpsc::channel(8);
        let mut engine = PtyEngine::new(session_id, tx, 8, 2, 100);

        engine.advance_bytes(b"\x1b[4 q");
        assert_eq!(engine.snapshot_grid().cursor_style, CursorStyle::Underline);

        engine.advance_bytes(b"\x1b[6 q");
        assert_eq!(engine.snapshot_grid().cursor_style, CursorStyle::Bar);
    }

    #[test]
    fn cursor_visibility_sequence_is_mapped_from_wezterm() {
        let session_id = SessionId::new_v4();
        let (tx, _rx) = mpsc::channel(8);
        let mut engine = PtyEngine::new(session_id, tx, 8, 2, 100);

        engine.advance_bytes(b"\x1b[?25l");
        assert_eq!(engine.snapshot_grid().cursor_style, CursorStyle::Hidden);

        engine.advance_bytes(b"\x1b[?25h");
        assert_eq!(engine.snapshot_grid().cursor_style, CursorStyle::Block);
    }

    #[test]
    fn cursor_row_tracks_visible_viewport_after_scrollback() {
        let session_id = SessionId::new_v4();
        let (tx, _rx) = mpsc::channel(8);
        let mut engine = PtyEngine::new(session_id, tx, 4, 2, 100);

        engine.advance_bytes(b"1\r\n2\r\n3");
        let grid = engine.snapshot_grid();

        assert_eq!(grid.cursor_row, 1);
        assert_eq!(grid.cursor_col, 1);
        assert_eq!(grid.active_cells()[1][0].c, "3");
    }

    #[test]
    fn exited_event_is_eventually_delivered_when_queue_frees_up() {
        let session_id = SessionId::new_v4();
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(SessionEvent::Update {
            session_id,
            grid: Arc::new(TerminalGrid::new(4, 2, 100)),
            lines_added: 0,
        })
        .expect("channel must be fillable");

        send_exited_event_async(tx, session_id);
        assert!(matches!(rx.try_recv(), Ok(SessionEvent::Update { .. })));
        assert!(matches!(rx.blocking_recv(), Some(SessionEvent::Exited(_))));
    }
}
