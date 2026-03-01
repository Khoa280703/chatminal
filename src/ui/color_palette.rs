use iced::Color;

use crate::session::CellColor;

const BASE16: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (205, 49, 49),
    (13, 188, 121),
    (229, 229, 16),
    (36, 114, 200),
    (188, 63, 188),
    (17, 168, 205),
    (229, 229, 229),
    (102, 102, 102),
    (241, 76, 76),
    (35, 209, 139),
    (245, 245, 67),
    (59, 142, 234),
    (214, 112, 214),
    (41, 184, 219),
    (255, 255, 255),
];

pub fn indexed_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => BASE16[index as usize],
        16..=231 => {
            let idx = index - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            (component(r), component(g), component(b))
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

pub fn cell_color_to_iced(color: CellColor, is_fg: bool) -> Color {
    match color {
        CellColor::Default if is_fg => Color::WHITE,
        CellColor::Default => Color::BLACK,
        CellColor::Indexed(idx) => {
            let (r, g, b) = indexed_to_rgb(idx);
            Color::from_rgb8(r, g, b)
        }
        CellColor::Rgb(r, g, b) => Color::from_rgb8(r, g, b),
    }
}

fn component(value: u8) -> u8 {
    match value {
        0 => 0,
        _ => 55 + value * 40,
    }
}
