use ::window::ResizeIncrement;
use ::window::parameters::Border;

pub struct ResizeIncrementCalculator {
    pub x: u16,
    pub y: u16,
    pub padding_left: usize,
    pub padding_top: usize,
    pub padding_right: usize,
    pub padding_bottom: usize,
    pub border: Border,
    pub tab_bar_height: usize,
}

impl From<ResizeIncrementCalculator> for ResizeIncrement {
    fn from(calc: ResizeIncrementCalculator) -> ResizeIncrement {
        ResizeIncrement {
            x: calc.x,
            y: calc.y,
            base_width: (calc.padding_left
                + calc.padding_right
                + (calc.border.left + calc.border.right).get()) as u16,
            base_height: (calc.padding_top
                + calc.padding_bottom
                + (calc.border.top + calc.border.bottom).get()
                + calc.tab_bar_height) as u16,
        }
    }
}
