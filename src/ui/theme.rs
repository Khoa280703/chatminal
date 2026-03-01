use iced::Color;

pub const TERMINAL_BG: Color = Color::BLACK;
pub const SIDEBAR_WIDTH: f32 = 240.0;
pub const SESSION_ITEM_HEIGHT: f32 = 48.0;
pub const DEFAULT_FONT_SIZE: f32 = 14.0;
const CELL_WIDTH_RATIO: f32 = 0.6;
const CELL_HEIGHT_RATIO: f32 = 1.35;

pub fn metrics_for_font(font_size: f32) -> (f32, f32) {
    let size = if font_size.is_finite() {
        font_size.clamp(8.0, 48.0)
    } else {
        DEFAULT_FONT_SIZE
    };

    let cell_width = (size * CELL_WIDTH_RATIO).max(5.0);
    let cell_height = (size * CELL_HEIGHT_RATIO).max(size + 2.0);
    (cell_width, cell_height)
}
