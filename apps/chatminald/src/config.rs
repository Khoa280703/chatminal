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

    let mut base = dirs::data_dir().ok_or_else(|| "resolve data directory failed".to_string())?;
    base.push("chatminal");
    std::fs::create_dir_all(&base).map_err(|err| format!("create data directory failed: {err}"))?;

    let socket_name = if cfg!(target_os = "macos") {
        "chatminald.sock"
    } else {
        "chatminald-linux.sock"
    };
    Ok(base.join(socket_name).display().to_string())
}

fn resolve_default_shell() -> String {
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
        assert!(shell.starts_with('/'));
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
}
