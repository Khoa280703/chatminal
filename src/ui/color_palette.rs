use iced::Color;

use crate::session::CellColor;

pub fn cell_color_to_iced(color: CellColor, is_fg: bool) -> Color {
    match color {
        CellColor::Default if is_fg => Color::WHITE,
        CellColor::Default => Color::BLACK,
        CellColor::Rgb(r, g, b) => Color::from_rgb8(r, g, b),
    }
}
