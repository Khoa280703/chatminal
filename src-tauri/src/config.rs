use serde::{Deserialize, Serialize};

const DEFAULT_PREVIEW_LINES: usize = 100;
const DEFAULT_MAX_LINES_PER_SESSION: usize = 5_000;
const DEFAULT_AUTO_DELETE_AFTER_DAYS: u32 = 30;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub shell: Option<String>,
    pub settings: UserSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub theme: String,
    pub font_size: f32,
    pub default_shell: Option<String>,
    pub persist_scrollback_enabled: bool,
    pub max_lines_per_session: usize,
    pub auto_delete_after_days: u32,
    pub preview_lines: usize,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            font_size: 14.0,
            default_shell: None,
            persist_scrollback_enabled: false,
            max_lines_per_session: DEFAULT_MAX_LINES_PER_SESSION,
            auto_delete_after_days: DEFAULT_AUTO_DELETE_AFTER_DAYS,
            preview_lines: DEFAULT_PREVIEW_LINES,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
struct LegacyConfig {
    shell: Option<String>,
}

pub fn load_config() -> AppConfig {
    let legacy_shell = load_legacy_shell();
    let settings = load_settings_file();

    let shell = settings
        .default_shell
        .clone()
        .or(legacy_shell)
        .filter(|value| !value.trim().is_empty());

    AppConfig { shell, settings }
}

fn load_legacy_shell() -> Option<String> {
    let Some(mut path) = dirs::config_dir() else {
        return None;
    };

    path.push("chatminal");
    path.push("config.toml");

    let Ok(contents) = std::fs::read_to_string(path) else {
        return None;
    };

    toml::from_str::<LegacyConfig>(&contents)
        .ok()
        .and_then(|cfg| cfg.shell)
}

fn load_settings_file() -> UserSettings {
    let path = settings_path();

    let settings = std::fs::read_to_string(&path)
        .ok()
        .and_then(|contents| serde_json::from_str::<UserSettings>(&contents).ok())
        .map(normalize_settings)
        .unwrap_or_default();

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&settings).unwrap_or_else(|_| "{}".to_string()),
    );

    settings
}

fn settings_path() -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("chatminal");
    path.push("settings.json");
    path
}

fn normalize_settings(mut settings: UserSettings) -> UserSettings {
    settings.max_lines_per_session = settings.max_lines_per_session.clamp(100, 5_000);
    settings.auto_delete_after_days = settings.auto_delete_after_days.clamp(0, 3650);
    settings.preview_lines = settings.preview_lines.clamp(10, 5000);
    settings.font_size = settings.font_size.clamp(8.0, 48.0);
    settings
}
