use crate::ui::theme::{DEFAULT_FONT_SIZE, SIDEBAR_WIDTH};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub shell: Option<String>,
    pub scrollback_lines: Option<usize>,
    pub font_size: Option<f32>,
    pub sidebar_width: Option<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shell: None,
            scrollback_lines: Some(SCROLLBACK_DEFAULT_LINES),
            font_size: Some(DEFAULT_FONT_SIZE),
            sidebar_width: Some(SIDEBAR_WIDTH),
        }
    }
}

impl Config {
    pub fn normalized(self) -> Self {
        let defaults = Self::default().normalized_defaults();
        Self {
            shell: self.shell,
            scrollback_lines: Some(
                self.scrollback_lines
                    .unwrap_or(
                        defaults
                            .scrollback_lines
                            .unwrap_or(SCROLLBACK_DEFAULT_LINES),
                    )
                    .clamp(SCROLLBACK_MIN_LINES, SCROLLBACK_HARD_MAX_LINES),
            ),
            font_size: Some(clamp_f32(
                self.font_size,
                defaults.font_size.unwrap_or(DEFAULT_FONT_SIZE),
                FONT_SIZE_MIN,
                FONT_SIZE_MAX,
            )),
            sidebar_width: Some(clamp_f32(
                self.sidebar_width,
                defaults.sidebar_width.unwrap_or(SIDEBAR_WIDTH),
                SIDEBAR_WIDTH_MIN,
                SIDEBAR_WIDTH_MAX,
            )),
        }
    }

    fn normalized_defaults(mut self) -> Self {
        self.scrollback_lines = Some(
            self.scrollback_lines
                .unwrap_or(SCROLLBACK_DEFAULT_LINES)
                .clamp(SCROLLBACK_MIN_LINES, SCROLLBACK_HARD_MAX_LINES),
        );
        self.font_size = Some(clamp_f32(
            self.font_size,
            DEFAULT_FONT_SIZE,
            FONT_SIZE_MIN,
            FONT_SIZE_MAX,
        ));
        self.sidebar_width = Some(clamp_f32(
            self.sidebar_width,
            SIDEBAR_WIDTH,
            SIDEBAR_WIDTH_MIN,
            SIDEBAR_WIDTH_MAX,
        ));
        self
    }
}

pub const SCROLLBACK_DEFAULT_LINES: usize = 10_000;
pub const SCROLLBACK_MIN_LINES: usize = 100;
pub const SCROLLBACK_HARD_MAX_LINES: usize = 200_000;
pub const SCROLL_LINES_PER_TICK: usize = 3;
pub const FONT_SIZE_MIN: f32 = 8.0;
pub const FONT_SIZE_MAX: f32 = 48.0;
pub const SIDEBAR_WIDTH_MIN: f32 = 160.0;
pub const SIDEBAR_WIDTH_MAX: f32 = 640.0;

fn clamp_f32(value: Option<f32>, default: f32, min: f32, max: f32) -> f32 {
    match value {
        Some(number) if number.is_finite() => number.clamp(min, max),
        _ => default.clamp(min, max),
    }
}

pub fn load_config() -> Config {
    let Some(mut config_path) = dirs::config_dir() else {
        return Config::default();
    };

    config_path.push("chatminal");
    config_path.push("config.toml");

    match std::fs::read_to_string(config_path) {
        Ok(raw) => toml::from_str::<Config>(&raw)
            .map(Config::normalized)
            .unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::theme::{DEFAULT_FONT_SIZE, SIDEBAR_WIDTH};

    use super::{
        Config, FONT_SIZE_MAX, FONT_SIZE_MIN, SCROLLBACK_HARD_MAX_LINES, SCROLLBACK_MIN_LINES,
        SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN,
    };

    #[test]
    fn normalized_clamps_numeric_values() {
        let normalized = Config {
            shell: None,
            scrollback_lines: Some(usize::MAX),
            font_size: Some(99.0),
            sidebar_width: Some(3.0),
        }
        .normalized();

        assert_eq!(normalized.scrollback_lines, Some(SCROLLBACK_HARD_MAX_LINES));
        assert_eq!(normalized.font_size, Some(FONT_SIZE_MAX));
        assert_eq!(normalized.sidebar_width, Some(SIDEBAR_WIDTH_MIN));
    }

    #[test]
    fn normalized_handles_non_finite_values() {
        let normalized = Config {
            shell: None,
            scrollback_lines: Some(0),
            font_size: Some(f32::NAN),
            sidebar_width: Some(f32::INFINITY),
        }
        .normalized();

        assert_eq!(normalized.scrollback_lines, Some(SCROLLBACK_MIN_LINES));
        assert_eq!(
            normalized.font_size,
            Some(DEFAULT_FONT_SIZE.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX))
        );
        assert_eq!(
            normalized.sidebar_width,
            Some(SIDEBAR_WIDTH.clamp(SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX))
        );
    }
}
