use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::AppConfig;

const CHATMINAL_DESKTOP_PACKAGE: &str = "chatminal-desktop";
const CHATMINAL_DESKTOP_BIN: &str = "chatminal-desktop";
const DESKTOP_PROXY_COMMAND: &str = "proxy-desktop-session";
const SIDEBAR_ENV: &str = "CHATMINAL_DESKTOP_SESSIONS_SIDEBAR";
const DESKTOP_BIN_ENV: &str = "CHATMINAL_DESKTOP_BIN";

pub fn run_window_desktop(config: &AppConfig, args: &[String]) -> Result<(), String> {
    let _ = config;
    ensure_linux_gui_session_available()?;
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

    if let Some(binary) = resolve_desktop_override_binary() {
        spawn_desktop_process(&binary, session_id.as_deref())?;
        return Ok(());
    }

    if workspace_desktop_manifest().is_some() {
        let binary = ensure_workspace_desktop_binary()?;
        spawn_desktop_process(&binary, session_id.as_deref())?;
        return Ok(());
    }

    Err(format!(
        "chatminal desktop runtime not found. Set {DESKTOP_BIN_ENV} to a compatible {CHATMINAL_DESKTOP_BIN} binary or ensure apps/{CHATMINAL_DESKTOP_PACKAGE} exists in the workspace."
    ))
}

fn ensure_workspace_desktop_binary() -> Result<PathBuf, String> {
    let workspace_manifest =
        workspace_manifest_path().ok_or_else(|| "workspace Cargo.toml not found".to_string())?;
    let manifest = workspace_desktop_manifest()
        .ok_or_else(|| "chatminal desktop manifest not found".to_string())?;
    if !manifest.is_file() {
        return Err(format!("missing manifest: {}", manifest.display()));
    }

    let workspace_root = workspace_root().ok_or_else(|| "workspace root not found".to_string())?;
    let profile = build_profile_name();
    let binary_path = resolve_workspace_desktop_binary_path(&workspace_root, profile);
    let target_dir = resolve_target_dir(&workspace_root);
    let mut command = Command::new("cargo");
    command
        .arg("build")
        .arg("--manifest-path")
        .arg(&workspace_manifest)
        .arg("--package")
        .arg(CHATMINAL_DESKTOP_PACKAGE)
        .env("CARGO_TARGET_DIR", &target_dir);
    if profile == "release" {
        command.arg("--release");
    }
    let output = command
        .output()
        .map_err(|err| format!("build {CHATMINAL_DESKTOP_PACKAGE} failed: {err}"))?;
    if !output.status.success() {
        let detail = summarize_cargo_failure_output(&output);
        return Err(format!(
            "build {} failed with status {}{}",
            CHATMINAL_DESKTOP_PACKAGE, output.status, detail
        ));
    }
    if !is_launchable_path(&binary_path) {
        return Err(format!(
            "built {} but binary is missing at {}",
            CHATMINAL_DESKTOP_PACKAGE,
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

fn spawn_desktop_process(desktop_bin: &Path, session_id: Option<&str>) -> Result<(), String> {
    let mut command = Command::new(desktop_bin);
    for arg in build_desktop_start_args(session_id) {
        command.arg(arg);
    }
    command.env(SIDEBAR_ENV, "1");
    command
        .spawn()
        .map_err(|err| format!("spawn chatminal desktop failed: {err}"))?;
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
                "không tìm thấy desktop display cho Chatminal Desktop. Hãy chạy trong phiên đồ họa có DISPLAY hoặc WAYLAND_DISPLAY."
                    .to_string(),
            );
        }
    }

    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
use std::ffi::OsStr;

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

fn workspace_desktop_manifest() -> Option<PathBuf> {
    let workspace_root = workspace_manifest_path()?.parent().map(Path::to_path_buf)?;
    let manifest = workspace_root
        .join("apps")
        .join(CHATMINAL_DESKTOP_PACKAGE)
        .join("Cargo.toml");
    manifest.is_file().then_some(manifest)
}

fn resolve_workspace_desktop_binary_path(workspace_root: &Path, profile: &str) -> PathBuf {
    resolve_target_dir(workspace_root)
        .join(profile)
        .join(binary_filename(CHATMINAL_DESKTOP_BIN))
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

fn resolve_desktop_override_binary() -> Option<PathBuf> {
    std::env::var(DESKTOP_BIN_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| is_launchable_path(path))
}

fn build_desktop_start_args(session_id: Option<&str>) -> Vec<String> {
    let mut args = Vec::new();
    if cfg!(target_os = "macos") {
        args.push("--config".to_string());
        args.push(
            "window_decorations=\"INTEGRATED_BUTTONS|RESIZE|MACOS_USE_BACKGROUND_COLOR_AS_TITLEBAR_COLOR\""
                .to_string(),
        );
    }
    args.extend([
        "start".to_string(),
        "--".to_string(),
        "chatminal-runtime".to_string(),
        DESKTOP_PROXY_COMMAND.to_string(),
    ]);
    if let Some(value) = session_id {
        args.push(value.to_string());
    }
    args
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
        build_desktop_start_args, build_profile_name, resolve_target_dir,
        resolve_workspace_desktop_binary_path, summarize_cargo_failure_output, tail_lines,
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    use std::ffi::OsStr;
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;
    use std::path::{Path, PathBuf};

    #[test]
    fn build_start_args_contains_proxy_command() {
        let args = build_desktop_start_args(Some("session-1"));
        assert_eq!(
            args,
            vec![
                "start",
                "--",
                "chatminal-runtime",
                "proxy-desktop-session",
                "session-1",
            ]
        );
    }

    #[test]
    fn target_dir_defaults_to_workspace_target() {
        let workspace_root = Path::new("/tmp/chatminal-root");
        let target = resolve_target_dir(workspace_root);
        assert_eq!(target, PathBuf::from("/tmp/chatminal-root/target"));
    }

    #[test]
    fn profile_name_matches_compiler_mode() {
        let profile = build_profile_name();
        assert!(profile == "debug" || profile == "release");
    }

    #[test]
    fn workspace_binary_path_is_sibling_of_current_exe() {
        let binary =
            resolve_workspace_desktop_binary_path(Path::new("/tmp/chatminal-root"), "debug");
        assert_eq!(
            binary,
            PathBuf::from("/tmp/chatminal-root/target/debug/chatminal-desktop")
        );
    }

    #[test]
    fn summarize_cargo_failure_output_prefers_stderr() {
        let output = std::process::Output {
            status: std::process::ExitStatus::from_raw(1),
            stdout: b"stdout line 1\nstdout line 2\n".to_vec(),
            stderr: b"stderr line 1\nstderr line 2\n".to_vec(),
        };

        let summary = summarize_cargo_failure_output(&output);
        assert!(summary.contains("cargo stderr"));
        assert!(summary.contains("stderr line 2"));
        assert!(summary.contains("cargo stdout"));
    }

    #[test]
    fn tail_lines_keeps_only_requested_suffix() {
        let text = "1\n2\n3\n4\n5\n";
        assert_eq!(tail_lines(text, 2), "4\n5");
        assert_eq!(tail_lines(text, 10), "1\n2\n3\n4\n5");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_gui_session_requires_display_or_wayland() {
        assert!(!linux_gui_session_available_from_env(None, None));
        assert!(linux_gui_session_available_from_env(
            Some(OsStr::new(":1")),
            None
        ));
        assert!(linux_gui_session_available_from_env(
            None,
            Some(OsStr::new("wayland-0"))
        ));
    }
}
