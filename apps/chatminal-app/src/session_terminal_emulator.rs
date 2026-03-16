use std::collections::HashMap;
use std::sync::Arc;

use chatminal_protocol::{PtyOutputEvent, SessionSnapshot};
use chatminal_terminal_core::color::ColorPalette;
use chatminal_terminal_core::{Terminal, TerminalConfiguration, TerminalSize};
use serde::Serialize;

use crate::session_terminal_adapter::SessionTerminalAdapter;

const DEFAULT_DPI: u32 = 96;
const TERM_PROGRAM: &str = "chatminal-app";
const TERM_VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[derive(Debug, Clone, Serialize)]
pub struct TerminalSnapshotSummary {
    pub terminal_id: String,
    pub session_id: String,
    pub cols: usize,
    pub rows: usize,
    pub visible_text: String,
}

struct SessionTerminal {
    session_id: String,
    cols: usize,
    rows: usize,
    terminal: Terminal,
}

impl SessionTerminal {
    fn new(session_id: &str, cols: usize, rows: usize, scrollback_size: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let config = Arc::new(EmbeddedTerminalConfig { scrollback_size });
        let terminal = Terminal::new(
            TerminalSize {
                rows,
                cols,
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
            session_id: session_id.to_string(),
            cols,
            rows,
            terminal,
        }
    }

    fn reset_with_snapshot(&mut self, snapshot: &SessionSnapshot, scrollback_size: usize) {
        let next = Self::new(&self.session_id, self.cols, self.rows, scrollback_size);
        self.terminal = next.terminal;
        if !snapshot.content.is_empty() {
            self.terminal.advance_bytes(snapshot.content.as_bytes());
        }
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
        self.terminal.resize(TerminalSize {
            rows: self.rows,
            cols: self.cols,
            pixel_width: 0,
            pixel_height: 0,
            dpi: DEFAULT_DPI,
        });
    }

    fn apply_output(&mut self, event: &PtyOutputEvent) {
        if !event.chunk.is_empty() {
            self.terminal.advance_bytes(event.chunk.as_bytes());
        }
    }

    fn visible_text(&self) -> String {
        let screen = self.terminal.screen();
        let rows = screen.physical_rows.max(1);
        let total_rows = screen.scrollback_rows();
        let visible_start = total_rows.saturating_sub(rows);
        let lines = screen.lines_in_phys_range(visible_start..total_rows);
        let mut rendered = lines
            .iter()
            .map(|line| line.as_str().to_string())
            .collect::<Vec<String>>();

        if !rendered.is_empty() {
            let cursor = self.terminal.cursor_pos();
            let cursor_row = usize::try_from(cursor.y.max(0))
                .unwrap_or(0)
                .min(rendered.len().saturating_sub(1));
            let cursor_col = cursor.x;
            if let Some(line) = rendered.get_mut(cursor_row) {
                overlay_cursor_glyph(line, cursor_col);
            }
        }

        rendered.join("\n")
    }
}

fn overlay_cursor_glyph(line: &mut String, cursor_col: usize) {
    const CURSOR_GLYPH: &str = "█";
    let char_len = line.chars().count();
    if cursor_col >= char_len {
        if cursor_col > char_len {
            line.push_str(&" ".repeat(cursor_col - char_len));
        }
        line.push_str(CURSOR_GLYPH);
        return;
    }

    let start = nth_char_boundary(line, cursor_col);
    let end = nth_char_boundary(line, cursor_col.saturating_add(1));
    line.replace_range(start..end, CURSOR_GLYPH);
}

fn nth_char_boundary(value: &str, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    value
        .char_indices()
        .nth(n)
        .map(|(idx, _)| idx)
        .unwrap_or(value.len())
}

pub struct SessionTerminalEmulator {
    terminals: HashMap<String, SessionTerminal>,
    default_cols: usize,
    default_rows: usize,
    scrollback_size: usize,
}

impl SessionTerminalEmulator {
    pub fn new(default_cols: usize, default_rows: usize, scrollback_size: usize) -> Self {
        Self {
            terminals: HashMap::new(),
            default_cols: default_cols.max(1),
            default_rows: default_rows.max(1),
            scrollback_size: scrollback_size.max(100),
        }
    }

    fn ensure_terminal(&mut self, session_id: &str, terminal_id: &str) -> &mut SessionTerminal {
        self.terminals.entry(terminal_id.to_string()).or_insert_with(|| {
            SessionTerminal::new(
                session_id,
                self.default_cols,
                self.default_rows,
                self.scrollback_size,
            )
        })
    }

    pub fn terminal_snapshot(&self, terminal_id: &str) -> Option<TerminalSnapshotSummary> {
        let terminal = self.terminals.get(terminal_id)?;
        Some(TerminalSnapshotSummary {
            terminal_id: terminal_id.to_string(),
            session_id: terminal.session_id.clone(),
            cols: terminal.cols,
            rows: terminal.rows,
            visible_text: terminal.visible_text(),
        })
    }

    pub fn all_terminal_snapshots(&self) -> Vec<TerminalSnapshotSummary> {
        self.terminals
            .iter()
            .filter_map(|(terminal_id, _)| self.terminal_snapshot(terminal_id))
            .collect()
    }
}

impl SessionTerminalAdapter for SessionTerminalEmulator {
    fn on_session_activated(&mut self, session_id: &str, terminal_id: &str, cols: usize, rows: usize) {
        let terminal = self.ensure_terminal(session_id, terminal_id);
        terminal.session_id = session_id.to_string();
        terminal.resize(cols, rows);
    }

    fn on_session_snapshot(&mut self, session_id: &str, terminal_id: &str, snapshot: &SessionSnapshot) {
        let scrollback_size = self.scrollback_size;
        let terminal = self.ensure_terminal(session_id, terminal_id);
        terminal.session_id = session_id.to_string();
        terminal.reset_with_snapshot(snapshot, scrollback_size);
    }

    fn on_session_output(&mut self, session_id: &str, terminal_id: &str, event: &PtyOutputEvent) {
        let terminal = self.ensure_terminal(session_id, terminal_id);
        terminal.session_id = session_id.to_string();
        terminal.apply_output(event);
    }

    fn on_session_resize(&mut self, session_id: &str, terminal_id: &str, cols: usize, rows: usize) {
        let terminal = self.ensure_terminal(session_id, terminal_id);
        terminal.session_id = session_id.to_string();
        terminal.resize(cols, rows);
    }
}

#[cfg(test)]
#[path = "session_terminal_emulator_tests.rs"]
mod session_terminal_emulator_tests;
