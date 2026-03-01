use std::collections::VecDeque;

pub type SessionId = uuid::Uuid;
const MIN_SCROLLBACK_MAX_LINES: usize = 100;
const MAX_SCROLLBACK_MAX_LINES: usize = 200_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellColor {
    Default,
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellAttrs {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub c: String,
    pub fg: CellColor,
    pub bg: CellColor,
    pub attrs: CellAttrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: String::new(),
            fg: CellColor::Default,
            bg: CellColor::Default,
            attrs: CellAttrs::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalGrid {
    pub cols: usize,
    pub rows: usize,
    pub primary_grid: Vec<Vec<Cell>>,
    pub alternate_grid: Vec<Vec<Cell>>,
    pub use_alternate: bool,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cursor_style: CursorStyle,
    pub scrollback: VecDeque<Vec<Cell>>,
    pub scrollback_max_lines: usize,
}

impl TerminalGrid {
    pub fn new(cols: usize, rows: usize, scrollback_max_lines: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let scrollback_max_lines =
            scrollback_max_lines.clamp(MIN_SCROLLBACK_MAX_LINES, MAX_SCROLLBACK_MAX_LINES);

        Self {
            cols,
            rows,
            primary_grid: vec![vec![Cell::default(); cols]; rows],
            alternate_grid: vec![vec![Cell::default(); cols]; rows],
            use_alternate: false,
            cursor_row: 0,
            cursor_col: 0,
            cursor_style: CursorStyle::default(),
            scrollback: VecDeque::new(),
            scrollback_max_lines,
        }
    }

    pub fn active_cells(&self) -> &Vec<Vec<Cell>> {
        if self.use_alternate {
            &self.alternate_grid
        } else {
            &self.primary_grid
        }
    }

    #[allow(dead_code)]
    pub fn active_cells_mut(&mut self) -> &mut Vec<Vec<Cell>> {
        if self.use_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        }
    }

    #[allow(dead_code)]
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);

        Self::resize_buffer(&mut self.primary_grid, self.cols, self.rows);
        Self::resize_buffer(&mut self.alternate_grid, self.cols, self.rows);

        self.cursor_col = self.cursor_col.min(self.cols.saturating_sub(1));
        self.cursor_row = self.cursor_row.min(self.rows.saturating_sub(1));
    }

    #[allow(dead_code)]
    pub fn set_cell(&mut self, row: usize, col: usize, cell: Cell) {
        if row < self.rows && col < self.cols {
            self.active_cells_mut()[row][col] = cell;
        }
    }

    #[allow(dead_code)]
    pub fn clear_row(&mut self, row: usize) {
        if row < self.rows {
            self.active_cells_mut()[row].fill(Cell::default());
        }
    }

    #[allow(dead_code)]
    pub fn clear_screen(&mut self) {
        for row in self.active_cells_mut().iter_mut() {
            row.fill(Cell::default());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    #[allow(dead_code)]
    pub fn switch_alternate(&mut self, enabled: bool) {
        self.use_alternate = enabled;
        self.cursor_row = self.cursor_row.min(self.rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.cols.saturating_sub(1));
    }

    #[allow(dead_code)]
    pub fn scroll_up(&mut self, count: usize) -> usize {
        if count == 0 {
            return 0;
        }

        let mut lines_added = 0;

        for _ in 0..count {
            let first_row = {
                let cells = self.active_cells_mut();
                if cells.is_empty() {
                    continue;
                }
                cells.remove(0)
            };

            if self.use_alternate {
                let cols = self.cols;
                self.active_cells_mut().push(vec![Cell::default(); cols]);
            } else {
                if self.scrollback.len() >= self.scrollback_max_lines {
                    let _ = self.scrollback.pop_front();
                }
                self.scrollback.push_back(first_row);
                self.primary_grid.push(vec![Cell::default(); self.cols]);
                lines_added += 1;
            }
        }

        lines_added
    }

    #[allow(dead_code)]
    pub fn scroll_down(&mut self, count: usize) {
        if count == 0 {
            return;
        }

        let cols = self.cols;
        for _ in 0..count {
            let _ = self.active_cells_mut().pop();
            self.active_cells_mut()
                .insert(0, vec![Cell::default(); cols]);
        }
    }

    #[allow(dead_code)]
    fn resize_buffer(buffer: &mut Vec<Vec<Cell>>, cols: usize, rows: usize) {
        if buffer.len() < rows {
            buffer.extend((0..(rows - buffer.len())).map(|_| vec![Cell::default(); cols]));
        } else if buffer.len() > rows {
            buffer.truncate(rows);
        }

        for row in buffer.iter_mut() {
            if row.len() < cols {
                row.resize(cols, Cell::default());
            } else if row.len() > cols {
                row.truncate(cols);
            }
        }
    }
}
