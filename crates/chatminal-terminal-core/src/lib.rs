pub mod color {
    pub use chatminal_upstream_term::color::ColorPalette;
}

pub use chatminal_upstream_term::{Terminal, TerminalConfiguration, TerminalSize};

#[cfg(test)]
mod tests {
    use super::{Terminal, TerminalConfiguration, TerminalSize, color::ColorPalette};
    use std::sync::Arc;

    #[derive(Debug)]
    struct TestConfig;

    impl TerminalConfiguration for TestConfig {
        fn scrollback_size(&self) -> usize {
            1000
        }

        fn color_palette(&self) -> ColorPalette {
            ColorPalette::default()
        }
    }

    #[test]
    fn public_api_can_construct_terminal() {
        let _terminal = Terminal::new(
            TerminalSize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                dpi: 96,
            },
            Arc::new(TestConfig),
            "chatminal-test",
            "0.1.0",
            Box::new(std::io::sink()),
        );
    }
}
