use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub endpoint: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            endpoint: resolve_endpoint()?,
        })
    }
}

fn resolve_endpoint() -> Result<String, String> {
    if let Ok(raw) = std::env::var("CHATMINAL_DAEMON_ENDPOINT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let mut base = dirs::data_dir().ok_or_else(|| "resolve data directory failed".to_string())?;
    base.push("chatminal");
    let socket_name = if cfg!(target_os = "macos") {
        "chatminald.sock"
    } else {
        "chatminald-linux.sock"
    };
    Ok(base.join(socket_name).display().to_string())
}

pub fn usage() -> &'static str {
    "Usage:
  chatminal-app workspace
  chatminal-app sessions
  chatminal-app create <name>
  chatminal-app activate <session_id> [cols] [rows] [preview_lines]
  chatminal-app activate-wezterm <session_id> [cols] [rows] [preview_lines]
  chatminal-app snapshot <session_id> [preview_lines]
  chatminal-app input <session_id> <data>
  chatminal-app resize <session_id> [cols] [rows]
  chatminal-app events [seconds]
  chatminal-app workspace-wezterm [preview_lines] [cols] [rows]
  chatminal-app events-wezterm [seconds]
  chatminal-app dashboard-wezterm [preview_lines] [cols] [rows] [max_pane_preview_lines]
  chatminal-app dashboard-watch-wezterm [seconds] [refresh_ms] [preview_lines] [cols] [rows] [max_pane_preview_lines]
  chatminal-app dashboard-tui-wezterm [refresh_ms] [preview_lines] [cols] [rows] [max_pane_preview_lines]
"
}

pub fn parse_usize(input: Option<&String>, default: usize) -> usize {
    input
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

#[allow(dead_code)]
fn _root_path() -> PathBuf {
    PathBuf::from("/")
}
