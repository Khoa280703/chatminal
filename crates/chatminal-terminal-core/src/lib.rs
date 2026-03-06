use std::ops::Range;
use std::sync::Arc;

pub mod color {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ColorPalette;
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalSize {
    pub rows: usize,
    pub cols: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
    pub dpi: u32,
}

pub trait TerminalConfiguration: std::fmt::Debug + Send + Sync {
    fn scrollback_size(&self) -> usize;
    fn color_palette(&self) -> color::ColorPalette;
}

#[derive(Debug, Clone, Copy)]
pub struct CursorPosition {
    pub x: usize,
    pub y: i64,
}

#[derive(Debug, Clone)]
pub struct ScreenLine(String);

impl ScreenLine {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct ScreenSnapshot {
    pub physical_rows: usize,
    lines: Vec<String>,
}

impl ScreenSnapshot {
    pub fn scrollback_rows(&self) -> usize {
        self.lines.len()
    }

    pub fn lines_in_phys_range(&self, range: Range<usize>) -> Vec<ScreenLine> {
        let len = self.lines.len();
        let start = range.start.min(len);
        let end = range.end.min(len);
        if start >= end {
            return Vec::new();
        }
        self.lines[start..end]
            .iter()
            .cloned()
            .map(ScreenLine)
            .collect()
    }
}

pub struct Terminal {
    parser: vt100::Parser,
    rows: usize,
    cols: usize,
    screen_cache: ScreenSnapshot,
}

impl Terminal {
    pub fn new(
        size: TerminalSize,
        config: Arc<dyn TerminalConfiguration>,
        _term_program: &str,
        _term_version: &str,
        _writer: Box<dyn std::io::Write + Send>,
    ) -> Self {
        let rows = size.rows.max(1);
        let cols = size.cols.max(1);
        let scrollback = config.scrollback_size().max(100);
        let parser = vt100::Parser::new(rows as u16, cols as u16, scrollback);
        let mut terminal = Self {
            parser,
            rows,
            cols,
            screen_cache: ScreenSnapshot {
                physical_rows: rows,
                lines: vec![String::new()],
            },
        };
        terminal.rebuild_screen_cache();
        terminal
    }

    pub fn advance_bytes(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.rebuild_screen_cache();
    }

    pub fn resize(&mut self, size: TerminalSize) {
        self.rows = size.rows.max(1);
        self.cols = size.cols.max(1);
        self.parser.screen_mut().set_size(self.rows as u16, self.cols as u16);
        self.rebuild_screen_cache();
    }

    pub fn screen(&self) -> ScreenSnapshot {
        self.screen_cache.clone()
    }

    pub fn cursor_pos(&self) -> CursorPosition {
        let (row, col) = self.parser.screen().cursor_position();
        CursorPosition {
            x: col as usize,
            y: row as i64,
        }
    }

    fn rebuild_screen_cache(&mut self) {
        let screen = self.parser.screen().clone();
        let (rows, cols) = screen.size();
        let mut probe = screen.clone();
        probe.set_scrollback(usize::MAX);
        let max_offset = probe.scrollback();

        let mut lines = if max_offset == 0 {
            screen
                .rows(0, cols)
                .map(trim_trailing_newline)
                .collect::<Vec<String>>()
        } else {
            let mut staged = Vec::with_capacity(rows as usize + max_offset);
            let mut capture = screen.clone();
            capture.set_scrollback(max_offset);
            staged.extend(capture.rows(0, cols).map(trim_trailing_newline));
            for offset in (0..max_offset).rev() {
                capture.set_scrollback(offset);
                if let Some(line) = capture.rows(0, cols).last() {
                    staged.push(trim_trailing_newline(line));
                }
            }
            staged
        };

        if lines.is_empty() {
            lines.push(String::new());
        }

        self.screen_cache = ScreenSnapshot {
            physical_rows: rows as usize,
            lines,
        };
    }
}

fn trim_trailing_newline(mut value: String) -> String {
    while value.ends_with('\n') {
        value.pop();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::{ScreenLine, Terminal, TerminalConfiguration, TerminalSize, color::ColorPalette};
    use std::sync::Arc;

    #[derive(Debug)]
    struct TestConfig;

    impl TerminalConfiguration for TestConfig {
        fn scrollback_size(&self) -> usize {
            1000
        }

        fn color_palette(&self) -> ColorPalette {
            ColorPalette
        }
    }

    #[test]
    fn public_api_can_construct_terminal() {
        let mut terminal = Terminal::new(
            TerminalSize {
                rows: 3,
                cols: 20,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            Arc::new(TestConfig),
            "chatminal-test",
            "0.1.0",
            Box::new(std::io::sink()),
        );
        terminal.advance_bytes(b"line-1\r\nline-2\r\nline-3\r\nline-4");
        let screen = terminal.screen();
        let rendered = screen
            .lines_in_phys_range(0..screen.scrollback_rows())
            .iter()
            .map(ScreenLine::as_str)
            .collect::<Vec<&str>>()
            .join("\n");
        assert!(rendered.contains("line-1"));
        assert!(rendered.contains("line-4"));
    }
}
