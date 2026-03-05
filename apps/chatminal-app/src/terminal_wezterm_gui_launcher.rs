use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::AppConfig;

pub fn run_window_wezterm_gui(config: &AppConfig, args: &[String]) -> Result<(), String> {
    let current_exe =
        std::env::current_exe().map_err(|err| format!("resolve current exe failed: {err}"))?;
    let session_id = args
        .get(2)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(binary) = resolve_wezterm_binary() {
        launch_with_wezterm_binary(
            &binary,
            &current_exe,
            &config.endpoint,
            session_id.as_deref(),
        )?;
        return Ok(());
    }

    launch_with_wezterm_source_build(&current_exe, &config.endpoint, session_id.as_deref())
}

fn launch_with_wezterm_binary(
    wezterm_bin: &Path,
    current_exe: &Path,
    endpoint: &str,
    session_id: Option<&str>,
) -> Result<(), String> {
    let mut command = Command::new(wezterm_bin);
    for arg in build_wezterm_start_args(current_exe, session_id) {
        command.arg(arg);
    }
    command
        .env("CHATMINAL_DAEMON_ENDPOINT", endpoint)
        .env("CHATMINAL_INTERNAL_PROXY", "1");
    command
        .spawn()
        .map_err(|err| format!("spawn wezterm failed: {err}"))?;
    Ok(())
}

fn launch_with_wezterm_source_build(
    current_exe: &Path,
    endpoint: &str,
    session_id: Option<&str>,
) -> Result<(), String> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .map_err(|err| format!("resolve workspace root failed: {err}"))?;
    let wezterm_manifest = workspace_root
        .join("third_party")
        .join("wezterm")
        .join("wezterm")
        .join("Cargo.toml");
    if !wezterm_manifest.exists() {
        return Err(
            "wezterm binary not found in PATH and third_party/wezterm source missing".to_string(),
        );
    }

    let mut command = Command::new("cargo");
    for arg in build_wezterm_source_run_args(&wezterm_manifest, current_exe, session_id) {
        command.arg(arg);
    }
    command
        .current_dir(&workspace_root)
        .env("CHATMINAL_DAEMON_ENDPOINT", endpoint)
        .env("CHATMINAL_INTERNAL_PROXY", "1");

    command
        .spawn()
        .map_err(|err| format!("spawn wezterm from source failed: {err}"))?;
    Ok(())
}

fn resolve_wezterm_binary() -> Option<PathBuf> {
    let override_value = std::env::var("CHATMINAL_WEZTERM_BIN").ok();
    let path_value = std::env::var_os("PATH");
    resolve_wezterm_binary_from_inputs(override_value.as_deref(), path_value.as_deref())
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "wezterm.exe"
    } else {
        "wezterm"
    }
}

fn build_wezterm_start_args(current_exe: &Path, session_id: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "start".to_string(),
        "--".to_string(),
        current_exe.to_string_lossy().to_string(),
        "proxy-wezterm-session".to_string(),
    ];
    if let Some(value) = session_id {
        args.push(value.to_string());
    }
    args
}

fn build_wezterm_source_run_args(
    wezterm_manifest: &Path,
    current_exe: &Path,
    session_id: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "run".to_string(),
        "--manifest-path".to_string(),
        wezterm_manifest.to_string_lossy().to_string(),
        "--".to_string(),
    ];
    args.extend(build_wezterm_start_args(current_exe, session_id));
    args
}

fn resolve_wezterm_binary_from_inputs(
    override_value: Option<&str>,
    path_value: Option<&OsStr>,
) -> Option<PathBuf> {
    if let Some(raw) = override_value {
        let value = raw.trim();
        if !value.is_empty() {
            let path = PathBuf::from(value);
            if is_launchable_path(&path) {
                return Some(path);
            }
        }
    }

    path_value.and_then(|paths| {
        std::env::split_paths(paths)
            .map(|entry| entry.join(binary_name()))
            .find(|path| is_launchable_path(path))
    })
}

fn is_launchable_path(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            return metadata.permissions().mode() & 0o111 != 0;
        }
        return false;
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        binary_name, build_wezterm_source_run_args, build_wezterm_start_args,
        resolve_wezterm_binary_from_inputs,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir_with_prefix(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock error")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn write_launchable_file(path: &PathBuf) {
        fs::write(path, b"#!/bin/sh\n").expect("write binary file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).expect("metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).expect("set permissions");
        }
    }

    #[test]
    fn resolve_wezterm_binary_prefers_override() {
        let dir = temp_dir_with_prefix("chatminal-wezterm-override");
        let override_bin = dir.join("custom-wezterm");
        write_launchable_file(&override_bin);

        let path_dir = temp_dir_with_prefix("chatminal-wezterm-path");
        let path_bin = path_dir.join(binary_name());
        write_launchable_file(&path_bin);
        let path_value = std::env::join_paths([path_dir]).expect("join path");

        let resolved = resolve_wezterm_binary_from_inputs(
            Some(override_bin.to_string_lossy().as_ref()),
            Some(path_value.as_os_str()),
        );
        assert_eq!(resolved, Some(override_bin));
    }

    #[test]
    fn resolve_wezterm_binary_falls_back_to_path() {
        let path_dir = temp_dir_with_prefix("chatminal-wezterm-path-only");
        let path_bin = path_dir.join(binary_name());
        write_launchable_file(&path_bin);
        let path_value = std::env::join_paths([path_dir]).expect("join path");

        let resolved = resolve_wezterm_binary_from_inputs(
            Some("/path/not/exist"),
            Some(path_value.as_os_str()),
        );
        assert_eq!(resolved, Some(path_bin));
    }

    #[test]
    fn resolve_wezterm_binary_skips_non_launchable_override() {
        let override_dir = temp_dir_with_prefix("chatminal-wezterm-override-noexec");
        let override_bin = override_dir.join("custom-wezterm");
        fs::write(&override_bin, b"#!/bin/sh\n").expect("write non-launchable override");

        let path_dir = temp_dir_with_prefix("chatminal-wezterm-path-fallback");
        let path_bin = path_dir.join(binary_name());
        write_launchable_file(&path_bin);
        let path_value = std::env::join_paths([path_dir]).expect("join path");

        let resolved = resolve_wezterm_binary_from_inputs(
            Some(override_bin.to_string_lossy().as_ref()),
            Some(path_value.as_os_str()),
        );
        assert_eq!(resolved, Some(path_bin));
    }

    #[test]
    fn build_wezterm_start_args_appends_optional_session_id() {
        let current_exe = PathBuf::from("/tmp/chatminal-app");
        let args = build_wezterm_start_args(&current_exe, Some("session-123"));
        assert_eq!(
            args,
            vec![
                "start",
                "--",
                "/tmp/chatminal-app",
                "proxy-wezterm-session",
                "session-123",
            ]
        );
    }

    #[test]
    fn build_wezterm_source_run_args_embeds_manifest_and_proxy_payload() {
        let manifest = PathBuf::from("/tmp/wezterm/Cargo.toml");
        let current_exe = PathBuf::from("/tmp/chatminal-app");
        let args = build_wezterm_source_run_args(&manifest, &current_exe, None);
        assert_eq!(
            args,
            vec![
                "run",
                "--manifest-path",
                "/tmp/wezterm/Cargo.toml",
                "--",
                "start",
                "--",
                "/tmp/chatminal-app",
                "proxy-wezterm-session",
            ]
        );
    }
}
