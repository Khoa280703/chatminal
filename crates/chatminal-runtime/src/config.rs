use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub endpoint: String,
    pub default_shell: String,
    pub default_preview_lines: usize,
    pub max_scrollback_lines_per_session: usize,
    pub default_cols: usize,
    pub default_rows: usize,
    pub health_interval_ms: u64,
}

impl DaemonConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            endpoint: resolve_endpoint()?,
            default_shell: resolve_default_shell(),
            default_preview_lines: std::env::var("CHATMINAL_PREVIEW_LINES")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(1000)
                .clamp(10, 5000),
            max_scrollback_lines_per_session: std::env::var("CHATMINAL_MAX_LINES_PER_SESSION")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(5000)
                .clamp(100, 20_000),
            default_cols: std::env::var("CHATMINAL_DEFAULT_COLS")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(120)
                .clamp(20, 400),
            default_rows: std::env::var("CHATMINAL_DEFAULT_ROWS")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(32)
                .clamp(5, 200),
            health_interval_ms: std::env::var("CHATMINAL_HEALTH_INTERVAL_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(5_000)
                .clamp(1_000, 60_000),
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
    std::fs::create_dir_all(&base).map_err(|err| format!("create data directory failed: {err}"))?;

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

fn resolve_default_shell() -> String {
    if let Ok(shell) = std::env::var("CHATMINAL_DEFAULT_SHELL") {
        let trimmed = shell.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if cfg!(windows) {
        if let Ok(shell) = std::env::var("COMSPEC") {
            let trimmed = shell.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        return "cmd.exe".to_string();
    }

    if let Ok(shell) = std::env::var("SHELL") {
        let trimmed = shell.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if cfg!(target_os = "macos") {
        "/bin/zsh".to_string()
    } else {
        "/bin/bash".to_string()
    }
}

pub fn resolve_session_cwd(input: Option<String>) -> String {
    if let Some(value) = input {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(home) = dirs::home_dir() {
        return home.display().to_string();
    }

    PathBuf::from("/").display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_session_cwd_uses_non_empty_input() {
        let resolved = resolve_session_cwd(Some(" /tmp/project ".to_string()));
        assert_eq!(resolved, "/tmp/project");
    }

    #[test]
    fn resolve_session_cwd_falls_back_to_home_or_root() {
        let resolved = resolve_session_cwd(None);
        if let Some(home) = dirs::home_dir() {
            assert_eq!(resolved, home.display().to_string());
        } else {
            assert_eq!(resolved, "/");
        }
    }

    #[test]
    fn resolve_default_shell_is_non_empty_path() {
        let shell = resolve_default_shell();
        assert!(!shell.trim().is_empty());
        if cfg!(windows) {
            assert!(shell.to_ascii_lowercase().contains("cmd"));
        } else {
            assert!(shell.starts_with('/'));
        }
    }

    #[test]
    fn default_terminal_size_has_valid_bounds() {
        let config = DaemonConfig::from_env().expect("load config");
        assert!(config.default_cols >= 20);
        assert!(config.default_rows >= 5);
    }

    #[test]
    fn max_scrollback_line_retention_has_valid_bounds() {
        let config = DaemonConfig::from_env().expect("load config");
        assert!((100..=20_000).contains(&config.max_scrollback_lines_per_session));
    }

    #[test]
    fn health_interval_has_valid_bounds() {
        let config = DaemonConfig::from_env().expect("load config");
        assert!((1_000..=60_000).contains(&config.health_interval_ms));
    }

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
}
