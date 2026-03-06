use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::AppConfig;

const CHATMINAL_WEZTERM_GUI_PACKAGE: &str = "chatminal-wezterm-gui";
const CHATMINAL_WEZTERM_GUI_BIN: &str = "chatminal-wezterm-gui";

pub fn run_window_wezterm_gui(config: &AppConfig, args: &[String]) -> Result<(), String> {
    ensure_linux_gui_session_available()?;

    let current_exe =
        std::env::current_exe().map_err(|err| format!("resolve current exe failed: {err}"))?;
    let session_id = match args.get(2) {
        Some(value) if value.parse::<usize>().is_err() => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        _ => None,
    };

    if let Some(binary) = resolve_wezterm_override_binary() {
        spawn_wezterm_process(
            &binary,
            &current_exe,
            &config.endpoint,
            session_id.as_deref(),
        )?;
        return Ok(());
    }

    let mut workspace_build_error = None;
    if workspace_wezterm_gui_manifest().is_some() {
        match ensure_workspace_wezterm_gui_binary() {
            Ok(binary) => {
                spawn_wezterm_process(
                    &binary,
                    &current_exe,
                    &config.endpoint,
                    session_id.as_deref(),
                )?;
                return Ok(());
            }
            Err(err) => workspace_build_error = Some(err),
        }
    }

    if let Some(binary) = resolve_wezterm_binary_from_path_or_platform() {
        spawn_wezterm_process(
            &binary,
            &current_exe,
            &config.endpoint,
            session_id.as_deref(),
        )?;
        return Ok(());
    }

    if let Some(err) = workspace_build_error {
        return Err(format!(
            "{err}. Fallback to system WezTerm was unavailable. Set CHATMINAL_WEZTERM_BIN or install a system wezterm binary."
        ));
    }

    Err("wezterm runtime not found. Set CHATMINAL_WEZTERM_BIN, install a system wezterm binary, or ensure apps/chatminal-wezterm-gui exists in the workspace.".to_string())
}

fn ensure_workspace_wezterm_gui_binary() -> Result<PathBuf, String> {
    let workspace_manifest =
        workspace_manifest_path().ok_or_else(|| "workspace Cargo.toml not found".to_string())?;
    let manifest = workspace_wezterm_gui_manifest()
        .ok_or_else(|| "chatminal-wezterm-gui manifest not found".to_string())?;
    if !manifest.is_file() {
        return Err(format!("missing manifest: {}", manifest.display()));
    }

    let workspace_root = workspace_root().ok_or_else(|| "workspace root not found".to_string())?;
    let profile = build_profile_name();
    let binary_path = resolve_workspace_wezterm_gui_binary_path(&workspace_root, profile);
    let target_dir = resolve_target_dir(&workspace_root);
    let mut command = Command::new("cargo");
    command
        .arg("build")
        .arg("--manifest-path")
        .arg(&workspace_manifest)
        .arg("--package")
        .arg(CHATMINAL_WEZTERM_GUI_PACKAGE)
        .env("CARGO_TARGET_DIR", &target_dir);
    if profile == "release" {
        command.arg("--release");
    }
    let output = command
        .output()
        .map_err(|err| format!("build {CHATMINAL_WEZTERM_GUI_PACKAGE} failed: {err}"))?;
    if !output.status.success() {
        let detail = summarize_cargo_failure_output(&output);
        return Err(format!(
            "build {} failed with status {}{}",
            CHATMINAL_WEZTERM_GUI_PACKAGE, output.status, detail
        ));
    }
    if !is_launchable_path(&binary_path) {
        return Err(format!(
            "built {} but binary is missing at {}",
            CHATMINAL_WEZTERM_GUI_PACKAGE,
            binary_path.display()
        ));
    }
    Ok(binary_path)
}

fn summarize_cargo_failure_output(output: &std::process::Output) -> String {
    let stderr = tail_lines(&String::from_utf8_lossy(&output.stderr), 80);
    let stdout = tail_lines(&String::from_utf8_lossy(&output.stdout), 40);

    match (stderr.is_empty(), stdout.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("\n---- cargo stderr ----\n{stderr}"),
        (true, false) => format!("\n---- cargo stdout ----\n{stdout}"),
        (false, false) => {
            format!("\n---- cargo stderr ----\n{stderr}\n---- cargo stdout ----\n{stdout}")
        }
    }
}

fn tail_lines(text: &str, max_lines: usize) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n").trim().to_string()
}

fn spawn_wezterm_process(
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

fn ensure_linux_gui_session_available() -> Result<(), String> {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let skip_check =
            std::env::var_os("CHATMINAL_SKIP_GUI_DISPLAY_CHECK").is_some_and(|value| value == "1");
        if skip_check {
            return Ok(());
        }

        let has_display = linux_gui_session_available_from_env(
            std::env::var_os("DISPLAY").as_deref(),
            std::env::var_os("WAYLAND_DISPLAY").as_deref(),
        );
        if !has_display {
            return Err(
                "không tìm thấy desktop display cho WezTerm GUI. Hãy chạy trong phiên đồ họa có DISPLAY hoặc WAYLAND_DISPLAY."
                    .to_string(),
            );
        }
    }

    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_gui_session_available_from_env(display: Option<&OsStr>, wayland: Option<&OsStr>) -> bool {
    display.is_some_and(|value| !value.is_empty()) || wayland.is_some_and(|value| !value.is_empty())
}

fn workspace_manifest_path() -> Option<PathBuf> {
    let manifest = workspace_root()?.join("Cargo.toml");
    manifest.is_file().then_some(manifest)
}

fn workspace_root() -> Option<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .ok()
}

fn workspace_wezterm_gui_manifest() -> Option<PathBuf> {
    let workspace_root = workspace_manifest_path()?.parent().map(Path::to_path_buf)?;
    let manifest = workspace_root
        .join("apps")
        .join(CHATMINAL_WEZTERM_GUI_PACKAGE)
        .join("Cargo.toml");
    manifest.is_file().then_some(manifest)
}

fn resolve_workspace_wezterm_gui_binary_path(workspace_root: &Path, profile: &str) -> PathBuf {
    resolve_target_dir(workspace_root)
        .join(profile)
        .join(binary_filename(CHATMINAL_WEZTERM_GUI_BIN))
}

fn resolve_target_dir(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CHATMINAL_CARGO_TARGET_DIR")
        .or_else(|| std::env::var_os("CARGO_TARGET_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"))
}

fn build_profile_name() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn resolve_wezterm_override_binary() -> Option<PathBuf> {
    std::env::var("CHATMINAL_WEZTERM_BIN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| is_launchable_path(path))
}

fn resolve_wezterm_binary_from_path_or_platform() -> Option<PathBuf> {
    let path_value = std::env::var_os("PATH");
    resolve_wezterm_binary_from_inputs(path_value.as_deref())
        .or_else(resolve_wezterm_binary_from_platform_defaults)
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

fn resolve_wezterm_binary_from_inputs(path_value: Option<&OsStr>) -> Option<PathBuf> {
    path_value.and_then(|paths| {
        std::env::split_paths(paths)
            .map(|entry| entry.join(binary_filename("wezterm")))
            .find(|path| is_launchable_path(path))
    })
}

fn resolve_wezterm_binary_from_platform_defaults() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let candidates = [
            "/Applications/WezTerm.app/Contents/MacOS/wezterm",
            "/System/Applications/WezTerm.app/Contents/MacOS/wezterm",
        ];
        return candidates
            .iter()
            .map(PathBuf::from)
            .find(|path| is_launchable_path(path));
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn binary_filename(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
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
        false
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(unix, not(target_os = "macos")))]
    use super::linux_gui_session_available_from_env;
    use super::{
        build_profile_name, build_wezterm_start_args, resolve_target_dir,
        resolve_wezterm_binary_from_inputs, resolve_workspace_wezterm_gui_binary_path,
        summarize_cargo_failure_output, tail_lines,
    };
    use std::ffi::OsStr;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;
    use std::path::{Path, PathBuf};
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
    fn resolve_wezterm_binary_falls_back_to_path() {
        let path_dir = temp_dir_with_prefix("chatminal-wezterm-path-only");
        let path_bin = path_dir.join(if cfg!(windows) {
            "wezterm.exe"
        } else {
            "wezterm"
        });
        write_launchable_file(&path_bin);
        let path_value = std::env::join_paths([path_dir]).expect("join path");

        let resolved = resolve_wezterm_binary_from_inputs(Some(path_value.as_os_str()));
        assert_eq!(resolved, Some(path_bin));
    }

    #[test]
    fn build_start_args_contains_proxy_command() {
        let args = build_wezterm_start_args(Path::new("/tmp/chatminal-app"), Some("session-1"));
        assert_eq!(
            args,
            vec![
                "start",
                "--",
                "/tmp/chatminal-app",
                "proxy-wezterm-session",
                "session-1",
            ]
        );
    }

    #[test]
    fn workspace_binary_path_is_sibling_of_current_exe() {
        let path =
            resolve_workspace_wezterm_gui_binary_path(Path::new("/tmp/chatminal-root"), "debug");
        assert_eq!(
            path,
            PathBuf::from("/tmp/chatminal-root/target/debug/chatminal-wezterm-gui")
        );
    }

    #[test]
    fn target_dir_defaults_to_workspace_target() {
        let target_dir = resolve_target_dir(Path::new("/tmp/chatminal-root"));
        assert_eq!(target_dir, PathBuf::from("/tmp/chatminal-root/target"));
    }

    #[test]
    fn profile_name_matches_compiler_mode() {
        let profile = build_profile_name();
        assert!(profile == "debug" || profile == "release");
    }

    #[test]
    fn tail_lines_keeps_only_requested_suffix() {
        let text = "a\nb\nc\nd";
        assert_eq!(tail_lines(text, 2), "c\nd");
    }

    #[test]
    fn summarize_cargo_failure_output_prefers_stderr() {
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: b"stdout-line\n".to_vec(),
            stderr: b"stderr-line\n".to_vec(),
        };
        let summary = summarize_cargo_failure_output(&output);
        assert!(summary.contains("cargo stderr"));
        assert!(summary.contains("stderr-line"));
        assert!(summary.contains("cargo stdout"));
        assert!(summary.contains("stdout-line"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_gui_session_available_accepts_display_or_wayland() {
        assert!(linux_gui_session_available_from_env(
            Some(OsStr::new(":99")),
            None
        ));
        assert!(linux_gui_session_available_from_env(
            None,
            Some(OsStr::new("wayland-1"))
        ));
        assert!(!linux_gui_session_available_from_env(None, None));
        assert!(!linux_gui_session_available_from_env(
            Some(OsStr::new("")),
            Some(OsStr::new(""))
        ));
    }
}
