use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPipelineMode {
    Desktop,
    Legacy,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub endpoint: String,
    pub input_pipeline_mode: InputPipelineMode,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            endpoint: resolve_endpoint()?,
            input_pipeline_mode: resolve_input_pipeline_mode(),
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

    if cfg!(windows) {
        return Ok(default_windows_pipe_endpoint());
    }

    let base = resolve_data_dir()?;
    let socket_name = if cfg!(target_os = "macos") {
        "chatminald.sock"
    } else {
        "chatminald-linux.sock"
    };
    Ok(base.join(socket_name).display().to_string())
}

fn default_windows_pipe_endpoint() -> String {
    let suffix = resolve_windows_pipe_suffix();
    format!(r"\\.\pipe\chatminald-{suffix}")
}

const WINDOWS_PIPE_SUFFIX_MAX_LEN: usize = 64;

fn sanitize_pipe_segment(input: &str) -> String {
    input
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch == '-' || ch == '_' {
                Some(ch)
            } else {
                None
            }
        })
        .collect()
}

fn resolve_windows_pipe_suffix() -> String {
    let raw_username = whoami::username();
    let username = sanitize_pipe_segment(raw_username.trim());
    if !username.is_empty() {
        return cap_windows_pipe_suffix(username);
    }
    if !raw_username.trim().is_empty() {
        return cap_windows_pipe_suffix(format!(
            "u{}",
            fnv1a64_hex(raw_username.trim().as_bytes())
        ));
    }

    if let Ok(raw_hostname) = whoami::fallible::hostname() {
        let hostname = sanitize_pipe_segment(raw_hostname.trim());
        if !hostname.is_empty() {
            return cap_windows_pipe_suffix(format!("host-{hostname}"));
        }
        if !raw_hostname.trim().is_empty() {
            return cap_windows_pipe_suffix(format!(
                "h{}",
                fnv1a64_hex(raw_hostname.trim().as_bytes())
            ));
        }
    }

    cap_windows_pipe_suffix("u0000000000000000".to_string())
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{hash:016x}")
}

fn cap_windows_pipe_suffix(mut value: String) -> String {
    if value.len() > WINDOWS_PIPE_SUFFIX_MAX_LEN {
        value.truncate(WINDOWS_PIPE_SUFFIX_MAX_LEN);
    }
    value
}

fn resolve_data_dir() -> Result<PathBuf, String> {
    if let Ok(raw) = std::env::var("CHATMINAL_DATA_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let configured = PathBuf::from(trimmed);
            if configured.is_absolute() {
                return Ok(configured);
            }
            if let Some(home) = dirs::home_dir() {
                return Ok(home.join(configured));
            }
            let cwd = std::env::current_dir()
                .map_err(|err| format!("resolve current dir failed: {err}"))?;
            return Ok(cwd.join(configured));
        }
    }
    let mut base = dirs::data_dir().ok_or_else(|| "resolve data directory failed".to_string())?;
    base.push("chatminal");
    Ok(base)
}

fn resolve_input_pipeline_mode() -> InputPipelineMode {
    let raw = std::env::var("CHATMINAL_INPUT_PIPELINE_MODE")
        .ok()
        .unwrap_or_else(|| "desktop".to_string());
    resolve_input_pipeline_mode_from_raw(&raw)
}

fn resolve_input_pipeline_mode_from_raw(raw: &str) -> InputPipelineMode {
    match parse_input_pipeline_mode(&raw) {
        Some(value) => value,
        None => InputPipelineMode::Desktop,
    }
}

fn parse_input_pipeline_mode(raw: &str) -> Option<InputPipelineMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "desktop" => Some(InputPipelineMode::Desktop),
        "legacy" => Some(InputPipelineMode::Legacy),
        _ => None,
    }
}

pub fn usage() -> &'static str {
    "Usage:
  chatminal-app workspace
  chatminal-app sessions
  chatminal-app create <name>
  chatminal-app activate <session_id> [cols] [rows] [preview_lines]
  chatminal-app snapshot <session_id> [preview_lines]
  chatminal-app input <session_id> <data>
  chatminal-app resize <session_id> [cols] [rows]
  chatminal-app events [seconds]
  chatminal-app workspace-terminal [preview_lines] [cols] [rows]
  chatminal-app activate-terminal <session_id> [cols] [rows] [preview_lines]
  chatminal-app events-terminal [seconds]
  chatminal-app dashboard [preview_lines] [cols] [rows] [max_pane_preview_lines]
  chatminal-app dashboard-watch [seconds] [refresh_ms] [preview_lines] [cols] [rows] [max_pane_preview_lines]
  chatminal-app dashboard-tui [refresh_ms] [preview_lines] [cols] [rows] [max_pane_preview_lines]
  chatminal-app attach [session_id] [cols] [rows] [preview_lines]
  chatminal-app window [session_id] [preview_lines] [cols] [rows]
  chatminal-app window-desktop [session_id]
  chatminal-app bench-rtt [samples] [warmup] [timeout_ms] [cols] [rows]

Environment:
  CHATMINAL_INPUT_PIPELINE_MODE=desktop|legacy
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_pipe_segment_keeps_safe_characters() {
        let value = sanitize_pipe_segment("User.Name-01_@Chatminal");
        assert_eq!(value, "username-01_chatminal");
    }

    #[test]
    fn default_windows_pipe_endpoint_uses_safe_suffix_or_default() {
        let endpoint = default_windows_pipe_endpoint();
        assert!(endpoint.starts_with(r"\\.\pipe\chatminald-"));
    }

    #[test]
    fn parse_input_pipeline_mode_accepts_expected_values() {
        assert_eq!(
            parse_input_pipeline_mode("desktop"),
            Some(InputPipelineMode::Desktop)
        );
        assert_eq!(
            parse_input_pipeline_mode("legacy"),
            Some(InputPipelineMode::Legacy)
        );
        assert_eq!(
            parse_input_pipeline_mode("  LEGACY  "),
            Some(InputPipelineMode::Legacy)
        );
        assert_eq!(parse_input_pipeline_mode("unknown"), None);
    }

    #[test]
    fn resolve_input_pipeline_mode_falls_back_to_desktop_on_invalid_value() {
        assert_eq!(
            resolve_input_pipeline_mode_from_raw("invalid"),
            InputPipelineMode::Desktop
        );
        assert_eq!(
            resolve_input_pipeline_mode_from_raw("legacy"),
            InputPipelineMode::Legacy
        );
    }
}
