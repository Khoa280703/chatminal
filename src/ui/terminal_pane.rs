use std::sync::Arc;

use iced::mouse;
use iced::widget::canvas::{self, Canvas};
use iced::{Element, Length, Pixels, Point, Rectangle, Size, Theme, font};

use crate::config::SCROLL_LINES_PER_TICK;
use crate::message::Message;
use crate::session::{Cell, CellColor, TerminalGrid};
use crate::ui::color_palette::cell_color_to_iced;
use crate::ui::theme::TERMINAL_BG;

#[derive(Debug, Clone)]
pub struct TerminalCanvas {
    pub grid: Option<Arc<TerminalGrid>>,
    pub scroll_offset: usize,
    pub cell_width: f32,
    pub cell_height: f32,
    pub font_size: f32,
    pub generation: u64,
}

#[derive(Default)]
pub struct TerminalCanvasState {
    cache: canvas::Cache,
    last_generation: u64,
}

pub fn terminal_pane_view<'a>(
    grid: Option<Arc<TerminalGrid>>,
    scroll_offset: usize,
    cell_width: f32,
    cell_height: f32,
    font_size: f32,
    generation: u64,
) -> Element<'a, Message> {
    Canvas::new(TerminalCanvas {
        grid,
        scroll_offset,
        cell_width,
        cell_height,
        font_size,
        generation,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

impl canvas::Program<Message> for TerminalCanvas {
    type State = TerminalCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if self.generation != state.last_generation {
            state.cache.clear();
            state.last_generation = self.generation;
        }

        if let canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            let lines = match delta {
                mouse::ScrollDelta::Lines { y, .. } => {
                    (*y * SCROLL_LINES_PER_TICK as f32).round() as i32
                }
                mouse::ScrollDelta::Pixels { y, .. } => {
                    (*y / self.cell_height.max(1.0)).round() as i32
                }
            };

            if lines != 0 {
                return Some(
                    canvas::Action::publish(Message::ScrollTerminal { delta: lines }).and_capture(),
                );
            }
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), TERMINAL_BG);

            let Some(grid) = self.grid.as_ref() else {
                return;
            };

            let visible_rows = (bounds.height / self.cell_height.max(1.0)).floor() as usize;
            let rows = build_render_rows(grid, self.scroll_offset, visible_rows.max(1));

            for (row_idx, row) in rows.iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    let x = col_idx as f32 * self.cell_width;
                    let y = row_idx as f32 * self.cell_height;

                    if x >= bounds.width || y >= bounds.height {
                        continue;
                    }

                    let bg = cell_color_to_iced(cell.bg, false);
                    if cell.bg != CellColor::Default {
                        frame.fill_rectangle(
                            Point::new(x, y),
                            Size::new(self.cell_width, self.cell_height),
                            bg,
                        );
                    }

                    if self.scroll_offset == 0
                        && row_idx == grid.cursor_row
                        && col_idx == grid.cursor_col
                    {
                        frame.fill_rectangle(
                            Point::new(x, y),
                            Size::new(self.cell_width, self.cell_height),
                            iced::Color::from_rgba(0.8, 0.8, 0.8, 0.45),
                        );
                    }

                    if cell.c != ' ' {
                        let fg = cell_color_to_iced(cell.fg, true);
                        let font = if cell.attrs.bold {
                            iced::Font {
                                weight: font::Weight::Bold,
                                ..iced::Font::MONOSPACE
                            }
                        } else {
                            iced::Font::MONOSPACE
                        };

                        frame.fill_text(canvas::Text {
                            content: cell.c.to_string(),
                            position: Point::new(x, y + self.cell_height - 3.0),
                            color: fg,
                            size: Pixels(self.font_size),
                            font,
                            ..Default::default()
                        });

                        if cell.attrs.underline {
                            frame.fill_rectangle(
                                Point::new(x, y + self.cell_height - 2.0),
                                Size::new(self.cell_width, 1.0),
                                fg,
                            );
                        }
                    }
                }
            }
        });

        vec![geometry]
    }
}

pub fn build_render_rows(grid: &TerminalGrid, offset: usize, visible_rows: usize) -> Vec<&[Cell]> {
    if grid.use_alternate || offset == 0 {
        return grid
            .active_cells()
            .iter()
            .take(visible_rows)
            .map(Vec::as_slice)
            .collect();
    }

    let scrollback_len = grid.scrollback.len();
    let start = scrollback_len.saturating_sub(offset);

    let mut rows: Vec<&[Cell]> = grid
        .scrollback
        .iter()
        .skip(start)
        .take(visible_rows)
        .map(Vec::as_slice)
        .collect();

    if rows.len() < visible_rows {
        let missing = visible_rows - rows.len();
        rows.extend(grid.active_cells().iter().take(missing).map(Vec::as_slice));
    }

    rows
}

#[cfg(test)]
mod tests {
    use crate::session::TerminalGrid;

    use super::build_render_rows;

    #[test]
    fn returns_exact_visible_rows_when_scrolling() {
        let mut grid = TerminalGrid::new(4, 2, 100);
        for i in 0..30 {
            grid.scrollback.push_back(vec![
                crate::session::Cell {
                    c: char::from(b'0' + (i % 10) as u8),
                    ..Default::default()
                };
                4
            ]);
        }

        let rows = build_render_rows(&grid, 10, 6);
        assert_eq!(rows.len(), 6);
    }
}
